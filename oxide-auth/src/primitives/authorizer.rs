//! Authorizers are need to exchange code grants for bearer tokens.
//!
//! The role of an authorizer is the ensure the consistency and security of request in which a
//! client is willing to trade a code grant for a bearer token. As such, it will first issue grants
//! to client according to parameters given by the resource owner and the registrar. Upon a client
//! side request, it will then check the given parameters to determine the authorization of such
//! clients.
use std::collections::HashMap;
use std::sync::{MutexGuard, RwLockWriteGuard, Arc};
use redis::cluster::{ClusterClient, ClusterClientBuilder};
use redis::Commands;

use super::grant::Grant;
use super::generator::TagGrant;

/// Authorizers create and manage authorization codes.
///
/// The authorization code can be traded for a bearer token at the token endpoint.
pub trait Authorizer {
    /// Create a code which allows retrieval of a bearer token at a later time.
    fn authorize(&mut self, _: Grant) -> Result<String, ()>;

    /// Retrieve the parameters associated with a token, invalidating the code in the process. In
    /// particular, a code should not be usable twice (there is no stateless implementation of an
    /// authorizer for this reason).
    fn extract(&mut self, token: &str) -> Result<Option<Grant>, ()>;
}

/// An in-memory hash map.
///
/// This authorizer saves a mapping of generated strings to their associated grants. The generator
/// is itself trait based and can be chosen during construction. It is assumed to not be possible
/// for two different grants to generate the same token in the issuer.
pub struct AuthMap<I: TagGrant = Box<dyn TagGrant + Send + Sync + 'static>> {
    tagger: I,
    _usage: u64,
    _tokens: HashMap<String, Grant>,
    redis: Arc<ClusterClient>
}

impl<I: TagGrant> AuthMap<I> {
    /// Create an authorizer generating tokens with the `tagger`.
    ///
    /// The token map is initially empty and is filled by methods provided in its [`Authorizer`]
    /// implementation.
    ///
    /// [`Authorizer`]: ./trait.Authorizer.html
    pub fn new(tagger: I, redis: Arc<ClusterClient>) -> Self {
        AuthMap {
            tagger,
            _usage: 0,
            _tokens: HashMap::new(),
            redis
        }
    }
}

impl<'a, A: Authorizer + ?Sized> Authorizer for &'a mut A {
    fn authorize(&mut self, grant: Grant) -> Result<String, ()> {
        (**self).authorize(grant)
    }

    fn extract(&mut self, code: &str) -> Result<Option<Grant>, ()> {
        (**self).extract(code)
    }
}

impl<A: Authorizer + ?Sized> Authorizer for Box<A> {
    fn authorize(&mut self, grant: Grant) -> Result<String, ()> {
        (**self).authorize(grant)
    }

    fn extract(&mut self, code: &str) -> Result<Option<Grant>, ()> {
        (**self).extract(code)
    }
}

impl<'a, A: Authorizer + ?Sized> Authorizer for MutexGuard<'a, A> {
    fn authorize(&mut self, grant: Grant) -> Result<String, ()> {
        (**self).authorize(grant)
    }

    fn extract(&mut self, code: &str) -> Result<Option<Grant>, ()> {
        (**self).extract(code)
    }
}

impl<'a, A: Authorizer + ?Sized> Authorizer for RwLockWriteGuard<'a, A> {
    fn authorize(&mut self, grant: Grant) -> Result<String, ()> {
        (**self).authorize(grant)
    }

    fn extract(&mut self, code: &str) -> Result<Option<Grant>, ()> {
        (**self).extract(code)
    }
}

impl<I: TagGrant> Authorizer for AuthMap<I> {
    fn authorize(&mut self, grant: Grant) -> Result<String, ()> {
        // The (usage, grant) tuple needs to be unique. Since this wraps after 2^64 operations, we
        // expect the validity time of the grant to have changed by then. This works when you don't
        // set your system time forward/backward ~20billion seconds, assuming ~10^9 operations per
        // second.
        let mut connection = match self.redis.get_connection() {
            Ok(c) => c,
            Err(err) => {
                error!("get connection error: {}", err.to_string());
                return Err(())
            }
        };
        let curr_usage = match connection.get::<_, u64>("oauth2:authmap_usage") {
            Ok(u) => u,
            Err(err) => {
                error!("get usage error: {}", err.to_string());
                return Err(())
            }
        };
        // let next_usage = self.usage.wrapping_add(1);
        let next_usage = curr_usage.wrapping_add(1);
        let token = self.tagger.tag(next_usage - 1, &grant)?;
        let grant_str = match serde_json::to_string(&grant) {
            Ok(str) => str,
            Err(err) => {
                error!("serde grant error: {}", err.to_string());
                return Err(())
            }
        };
        // self.tokens.insert(token.clone(), grant);
        // self.usage = next_usage;
        match connection.hset::<&str, String, String, usize>("oauth2:authmap", token.clone(), grant_str) {
            Ok(1) => (),
            Err(err) => {
                error!("set authmap error: {}", err.to_string());
                return Err(())
            }
        }
        match connection.set::<_, u64, usize>("oauth2:authmap_usage", next_usage) {
            Ok(1) => (),
            Err(err) => {
                error!("set usage error: {}", err.to_string());
                return Err(())
            }
        }
        Ok(token)
    }

    fn extract<'a>(&mut self, grant: &'a str) -> Result<Option<Grant>, ()> {
        let mut connection = match self.redis.get_connection() {
            Ok(c) => c,
            Err(err) => {
                error!("get connection error: {}", err.to_string());
                return Err(())
            }
        };
        let grant_value = match connection.hget::<&str, &str, String>("oauth2:authmap", grant) {
            Ok(str) => {
                let grant_value = match serde_json::from_str::<Grant>(str.as_str()) {
                    Ok(g) => Some(g),
                    Err(err) => {
                        error!("deserde grant error: {}", err.to_string());
                        return Err(())
                    }
                };
                match connection.hdel::<_, _, usize>("oauth2:authmap", grant) {
                    Ok(1) => (),
                    Err(err) => {
                        error!("del grant error: {}", err.to_string());
                        return Err(())
                    }
                };
                grant_value
            }
            _ => None
        };
        // Ok(self.tokens.remove(grant))
        Ok(grant_value)
    }
}

#[cfg(test)]
/// Tests for authorizer implementations, including those provided here.
pub mod tests {
    use super::*;
    use chrono::Utc;
    use crate::primitives::grant::Extensions;
    use crate::primitives::generator::{Assertion, AssertionKind, RandomGenerator};

    /// Tests some invariants that should be upheld by all authorizers.
    ///
    /// Custom implementations may want to import and use this in their own tests.
    pub fn simple_test_suite(authorizer: &mut dyn Authorizer) {
        let grant = Grant {
            owner_id: "Owner".to_string(),
            client_id: "Client".to_string(),
            scope: "One two three scopes".parse().unwrap(),
            redirect_uri: "https://example.com/redirect_me".parse().unwrap(),
            until: Utc::now(),
            extensions: Extensions::new(),
        };

        let token = authorizer
            .authorize(grant.clone())
            .expect("Authorization should not fail here");
        let recovered_grant = authorizer
            .extract(&token)
            .expect("Primitive failed extracting grant")
            .expect("Could not extract grant for valid token");

        if grant != recovered_grant {
            panic!("Grant was not stored correctly");
        }

        if authorizer.extract(&token).unwrap().is_some() {
            panic!("Token must only be usable once");
        }

        // Authorize the same token again.
        let token_again = authorizer
            .authorize(grant.clone())
            .expect("Authorization should not fail here");
        // We don't produce the same token twice.
        assert_ne!(token, token_again);
    }

    #[test]
    fn random_test_suite() {
        let client = get_local_redis();

        let mut storage = AuthMap::new(RandomGenerator::new(16), Arc::new(client));
        simple_test_suite(&mut storage);
    }

    #[test]
    fn signing_test_suite() {
        let client = get_local_redis();
        let assertion = Assertion::new(
            AssertionKind::HmacSha256,
            b"7EGgy8zManReq9l/ez0AyYE+xPpcTbssgW+8gBnIv3s=",
        );
        let mut storage = AuthMap::new(assertion, Arc::new(client));
        simple_test_suite(&mut storage);
    }

    #[test]
    #[should_panic]
    fn bad_generator() {
        let client = get_local_redis();
        struct BadGenerator;
        impl TagGrant for BadGenerator {
            fn tag(&mut self, _: u64, _: &Grant) -> Result<String, ()> {
                Ok("YOLO.HowBadCanItBeToRepeatTokens?".into())
            }
        }

        let mut storage = AuthMap::new(BadGenerator, Arc::new(client));
        simple_test_suite(&mut storage);
    }

    pub fn get_local_redis() -> ClusterClient {
        let builder = ClusterClientBuilder::new(vec!["redis://127.0.0.1:6379"]);
        builder.open().map_err(|err|{
            error!("{}", err.to_string());
            err
        }).unwrap()
    }
}
