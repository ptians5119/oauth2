//! Generates bearer tokens and refresh tokens.
//!
//! Internally similar to the authorization module, tokens generated here live longer and can be
//! renewed. There exist two fundamental implementation as well, one utilizing in memory hash maps
//! while the other uses cryptographic signing.
use std::collections::HashMap;
use std::sync::{Arc, MutexGuard, RwLockWriteGuard};
use std::sync::atomic::{AtomicUsize, Ordering};
use redis::cluster::ClusterClient;
use redis::Commands;

use chrono::{Duration, Utc};

use super::Time;
use super::grant::Grant;
use super::generator::{TagGrant, TaggedAssertion, Assertion};

/// Issuers create bearer tokens.
///
/// It's the issuers decision whether a refresh token is offered or not. In any case, it is also
/// responsible for determining the validity and parameters of any possible token string. Some
/// backends or frontends may decide not to propagate the refresh token (for example because
/// they do not intend to offer a statefull refresh api).
pub trait Issuer {
    /// Create a token authorizing the request parameters
    fn issue(&mut self, grant: Grant) -> Result<IssuedToken, ()>;

    /// Refresh a token.
    fn refresh(&mut self, _refresh: &str, _grant: Grant) -> Result<RefreshedToken, ()>;

    /// Get the values corresponding to a bearer token
    fn recover_token<'a>(&'a self, _: &'a str) -> Result<Option<Grant>, ()>;

    /// Get the values corresponding to a refresh token
    fn recover_refresh<'a>(&'a self, _: &'a str) -> Result<Option<Grant>, ()>;
}

/// Token parameters returned to a client.
#[derive(Clone, Debug)]
pub struct IssuedToken {
    /// The bearer token
    pub token: String,

    /// The refresh token, if any.
    pub refresh: Option<String>,

    /// Expiration timestamp (Utc).
    ///
    /// Technically, a time to live is expected in the response but this will be transformed later.
    /// In a direct backend access situation, this enables high precision timestamps.
    pub until: Time,

    /// The type of the token.
    pub token_type: TokenType,
}

/// The type of token, describing proper usage.
///
/// There is one other interesting type that is not yet formally specified: The MAC token,
/// see `draft-ietf-oauth-v2-http-mac-02`. The draft has long been expired but for the unlikely
/// case there are others, the enum exist. You might patch this to try out another token type
/// before proposing it for standardization.
///
/// In other context (RFC 8693) the explicitly non-access-token kind `N_A` also exists but this is
/// not a possible response.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum TokenType {
    /// A bearer token used on its own in an Authorization header.
    ///
    /// For this variant and its usage see RFC 6750.
    Bearer,
}

/// Refresh token information returned to a client.
#[derive(Clone, Debug)]
pub struct RefreshedToken {
    /// The bearer token.
    pub token: String,

    /// The new refresh token.
    ///
    /// If this is set, the old refresh token has been invalidated.
    pub refresh: Option<String>,

    /// Expiration timestamp (Utc).
    ///
    /// Technically, a time to live is expected in the response but this will be transformed later.
    /// In a direct backend access situation, this enables high precision timestamps.
    pub until: Time,

    /// The type of the new access token.
    pub token_type: TokenType,
}

/// Keeps track of access and refresh tokens by a hash-map.
///
/// The generator is itself trait based and can be chosen during construction. It is assumed to not
/// be possible (or at least very unlikely during their overlapping lifetime) for two different
/// grants to generate the same token in the grant tagger.
pub struct TokenMap<G: TagGrant = Box<dyn TagGrant + Send + Sync + 'static>> {
    duration: Option<Duration>,
    generator: G,
    _usage: u64,
    _access: HashMap<Arc<str>, Arc<Token>>,
    _refresh: HashMap<Arc<str>, Arc<Token>>,
    redis: Arc<ClusterClient>
    // register: Arc<Registrar>,
}

#[derive(Clone, Debug)]
struct Token {
    /// Back link to the access token.
    // access: Arc<str>,
    access: String,

    /// Link to a refresh token for this grant, if it exists.
    // refresh: Option<Arc<str>>,
    refresh: Option<String>,

    /// The grant that was originally granted.
    grant: Grant,
}

impl<G: TagGrant> TokenMap<G> {
    /// Construct a `TokenMap` from the given generator.
    pub fn new(generator: G, redis: Arc<ClusterClient>) -> Self {
        Self {
            duration: None,
            generator,
            _usage: 0,
            _access: HashMap::new(),
            _refresh: HashMap::new(),
            redis
        }
    }

    /// Set the validity of all issued grants to the specified duration.
    pub fn valid_for(&mut self, duration: Duration) {
        self.duration = Some(duration);
    }

    /// All grants are valid for their default duration.
    pub fn valid_for_default(&mut self) {
        self.duration = None;
    }

    /// Unconditionally delete grant associated with the token.
    ///
    /// This is the main advantage over signing tokens. By keeping internal state of allowed
    /// grants, the resource owner or other instances can revoke a token before it expires
    /// naturally. There is no differentiation between access and refresh tokens since these should
    /// have a marginal probability of colliding.
    pub fn revoke(&mut self, token: &str) {
        self._access.remove(token);
        self._refresh.remove(token);
    }

    /// Directly associate token with grant.
    ///
    /// No checks on the validity of the grant are performed but the expiration time of the grant
    /// is modified (if a `duration` was previously set).
    pub fn import_grant(&mut self, token: String, mut grant: Grant) {
        let mut connection = self.redis.get_connection()
            .map_err(|err| {
                error!("get connection error: {}", err.to_string());
                ()
            }).unwrap();

        self.set_duration(&mut grant);

        let _token = Token::from_access(token.clone(), grant);

        let key = format!("oauth2:token_map:access:{}", &token);

        let _ = super::set_grant(&mut connection, key.as_str(), &_token.grant, 86400);

        let _ = connection.hset::<&str, &str, String, usize>(key.as_str(), "access", _token.access);
    }

    fn set_duration(&self, grant: &mut Grant) {
        if let Some(duration) = &self.duration {
            grant.until = Utc::now() + *duration;
        }
    }
}

impl Token {
    fn from_access(access: String, grant: Grant) -> Self {
        Token {
            access,
            refresh: None,
            grant,
        }
    }

    fn from_refresh(access: String, refresh: String, grant: Grant) -> Self {
        Token {
            access,
            refresh: Some(refresh),
            grant,
        }
    }
}

impl IssuedToken {
    /// Construct a token that can not be refreshed.
    ///
    /// This is essential for issuers that can not revoke their tokens. Since refresh tokens are
    /// both long-lived and more powerful than their access token counterparts, it is more
    /// dangerous to have an unrevokable refresh token.
    ///
    /// This is only a shorthand for initializing the `IssuedToken` with `None` for `refresh`.
    ///
    /// ```
    /// # use oxide_auth::primitives::issuer::RefreshedToken;
    /// use oxide_auth::primitives::grant::Grant;
    /// use oxide_auth::primitives::issuer::{Issuer, IssuedToken};
    ///
    /// struct MyIssuer;
    ///
    /// impl MyIssuer {
    ///     fn access_token(&mut self, grant: &Grant) -> String {
    ///         // .. your implementation
    /// #       unimplemented!()
    ///     }
    /// }
    ///
    /// impl Issuer for MyIssuer {
    ///     fn issue(&mut self, mut grant: Grant) -> Result<IssuedToken, ()> {
    ///         let token = self.access_token(&grant);
    ///         Ok(IssuedToken::without_refresh(token, grant.until))
    ///     }
    ///     // …
    /// # fn recover_token<'t>(&'t self, token: &'t str) -> Result<Option<Grant>, ()> { Err(()) }
    /// # fn recover_refresh<'t>(&'t self, token: &'t str) -> Result<Option<Grant>, ()> { Err(()) }
    /// # fn refresh(&mut self, _: &str, _: Grant) -> Result<RefreshedToken, ()> { Err(()) }
    /// }
    /// ```
    pub fn without_refresh(token: String, until: Time) -> Self {
        IssuedToken {
            token,
            refresh: None,
            until,
            token_type: TokenType::Bearer,
        }
    }

    /// Determine if the access token can be refreshed.
    ///
    /// This returns `false` if `refresh` is `None` and `true` otherwise.
    pub fn refreshable(&self) -> bool {
        self.refresh.is_some()
    }
}

impl<G: TagGrant> Issuer for TokenMap<G> {
    fn issue(&mut self, mut grant: Grant) -> Result<IssuedToken, ()> {
        self.set_duration(&mut grant);
        // The (usage, grant) tuple needs to be unique. Since this wraps after 2^63 operations, we
        // expect the validity time of the grant to have changed by then. This works when you don't
        // set your system time forward/backward ~10billion seconds, assuming ~10^9 operations per
        // second.

        let mut connection = self.redis.get_connection()
            .map_err(|err| {
                error!("get connection error: {}", err.to_string());
                ()
            })?;
        let curr_usage = match connection.get::<_, u64>("oauth2:token_map:usage") {
            Ok(u) => u,
            Err(err) => {
                error!("get usage error: {}, generate it to 0", err.to_string());
                0
            }
        };
        let next_usage = curr_usage.wrapping_add(2);

        let (access, refresh) = {
            let access = self.generator.tag(curr_usage, &grant)?;
            let refresh = self.generator.tag(curr_usage.wrapping_add(1), &grant)?;
            debug_assert!(
                access.len() > 0,
                "An empty access token was generated, this is horribly insecure."
            );
            debug_assert!(
                refresh.len() > 0,
                "An empty refresh token was generated, this is horribly insecure."
            );
            (access, refresh)
        };

        let until = grant.until;
        let token = Token::from_refresh(access.clone(), refresh.clone(), grant);

        // 设置token map access
        let key = format!("oauth2:token_map:access:{}", &access);
        super::set_grant(&mut connection, key.as_str(), &token.grant, 86400)?;
        let _ = connection.hset::<&str, &str, String, usize>(key.as_str(), "access", access.clone())
            .map_err(|err| {
                error!("h_set access error {}", err);
                ()
            })?;

        // 设置token map refresh
        let key = format!("oauth2:token_map:refresh:{}", &refresh);
        super::set_grant(&mut connection, key.as_str(), &token.grant, 86400 * 7)?;
        let _ = connection.hset::<&str, &str, String, usize>(key.as_str(), "access", access.clone())
            .map_err(|err| {
                error!("h_set access error {}", err);
                ()
            })?;
        let _ = connection.hset::<&str, &str, String, usize>(key.as_str(), "refresh", refresh.clone())
            .map_err(|err| {
                error!("h_set refresh error {}", err);
                ()
            })?;

        match connection.set::<_, u64, String>("oauth2:token_map:usage", next_usage) {
            Ok(str) => {
                if str.ne("OK") {
                    error!("set usage error");
                    return Err(())
                }
            }
            Err(err) => {
                error!("set usage error: {}", err.to_string());
                return Err(())
            }
        }
        Ok(IssuedToken {
            token: access,
            refresh: Some(refresh),
            until,
            token_type: TokenType::Bearer,
        })
    }

    fn refresh(&mut self, refresh: &str, mut grant: Grant) -> Result<RefreshedToken, ()> {
        let mut connection = self.redis.get_connection()
            .map_err(|err| {
                error!("get connection error: {}", err.to_string());
                ()
            })?;

        // 获取token对象
        let key = format!("oauth2:token_map:refresh:{}", refresh);
        let redis_grant = super::get_grant(&mut connection, key.as_str())?.unwrap();
        let redis_access = connection.hget::<&str, &str, String>(key.as_str(), "access")
            .map_err(|err| {
                error!("get access error: {}", err);
                ()
            })?;
        let redis_refresh = connection.hget::<&str, &str, String>(key.as_str(), "refresh")
            .map_err(|err| {
                error!("get access error: {}", err);
                ()
            })?;
        let mut token = Token::from_refresh(redis_access, redis_refresh, redis_grant);
        let refresh_key = refresh.to_owned();

        // 删除旧token
        let _ = connection.del(key.as_str())
            .map_err(|err| {
                error!("del refresh error: {}", err);
                ()
            })?;

        // assert!(Arc::ptr_eq(token.refresh.as_ref().unwrap(), &refresh_key));
        assert!(refresh_key.eq(&token.refresh.clone().unwrap()));
        self.set_duration(&mut grant);
        let until = grant.until;

        let curr_usage = match connection.get::<_, u64>("oauth2:token_map:usage") {
            Ok(u) => u,
            Err(err) => {
                error!("get usage error: {}", err.to_string());
                return Err(())
            }
        };

        let next_usage = curr_usage.wrapping_add(1);

        let new_access = self.generator.tag(curr_usage, &grant)?;

        // 对比新旧access并删除缓存,这里有点多余,除非存的时候出错,不然肯定是匹配的
        let key = format!("oauth2:token_map:access:{}", &token.access);
        let redis_access = connection.hget::<&str, &str, String>(key.as_str(), "access")
            .map_err(|err| {
                error!("get access error: {}", err);
                ()
            })?;
        assert!(token.access.eq(&redis_access));
        let _ = connection.del(key.as_str())
            .map_err(|err| {
                error!("del access error: {}", err);
                ()
            })?;

        token.access = new_access.clone();
        token.grant = grant;

        // self.access.insert(new_key, token.clone());
        // self.refresh.insert(refresh_key, token);
        // self.usage = next_usage;
        // 重新设置token map access
        let key = format!("oauth2:token_map:access:{}", &new_access);
        super::set_grant(&mut connection, key.as_str(), &token.grant, 86400)?;
        let _ = connection.hset::<&str, &str, String, usize>(key.as_str(), "access", new_access.clone())
            .map_err(|err| {
                error!("h_set access error {}", err);
                ()
            })?;

        // 重新设置token map refresh
        let key = format!("oauth2:token_map:refresh:{}", refresh);
        super::set_grant(&mut connection, key.as_str(), &token.grant, 86400 * 7)?;
        let _ = connection.hset::<&str, &str, String, usize>(key.as_str(), "access", new_access.clone())
            .map_err(|err| {
                error!("h_set access error {}", err);
                ()
            })?;
        let _ = connection.hset::<&str, &str, String, usize>(key.as_str(), "refresh", refresh_key.clone())
            .map_err(|err| {
                error!("h_set refresh error {}", err);
                ()
            })?;

        match connection.set::<_, u64, String>("oauth2:token_map:usage", next_usage) {
            Ok(str) => {
                if str.ne("OK") {
                    error!("set usage error");
                    return Err(())
                }
            }
            Err(err) => {
                error!("set usage error: {}", err.to_string());
                return Err(())
            }
        }
        Ok(RefreshedToken {
            token: new_access,
            refresh: None,
            until,
            token_type: TokenType::Bearer,
        })
    }

    fn recover_token<'a>(&'a self, token: &'a str) -> Result<Option<Grant>, ()> {
        let mut connection = self.redis.get_connection()
            .map_err(|err| {
                error!("get connection error: {}", err.to_string());
                ()
            })?;
        let key = format!("oauth2:token_map:access:{}", &token);

        super::get_grant(&mut connection, key.as_str())
    }

    fn recover_refresh<'a>(&'a self, token: &'a str) -> Result<Option<Grant>, ()> {
        let mut connection = self.redis.get_connection()
            .map_err(|err| {
                error!("get connection error: {}", err.to_string());
                ()
            })?;
        let key = format!("oauth2:token_map:refresh:{}", &token);

        super::get_grant(&mut connection, key.as_str())
    }
}

/// Signs grants instead of storing them.
///
/// Although this token instance allows preservation of memory it also implies that tokens, once
/// issued, are impossible to revoke.
pub struct TokenSigner {
    duration: Option<Duration>,
    signer: Assertion,
    // FIXME: make this an AtomicU64 once stable.
    counter: AtomicUsize,
    have_refresh: bool,
}

impl TokenSigner {
    /// Construct a signing instance from a private signing key.
    ///
    /// Security notice: Never use a password alone to construct the signing key. Instead, generate
    /// a new key using a utility such as `openssl rand` that you then store away securely.
    pub fn new(secret: Assertion) -> TokenSigner {
        TokenSigner {
            duration: None,
            signer: secret,
            counter: AtomicUsize::new(0),
            have_refresh: false,
        }
    }

    /// Construct a signing instance whose tokens only live for the program execution.
    ///
    /// Useful for rapid prototyping where tokens need not be stored in a persistent database and
    /// can be invalidated at any time. This interface is provided with simplicity in mind, using
    /// the default system random generator (`ring::rand::SystemRandom`).
    pub fn ephemeral() -> TokenSigner {
        TokenSigner::new(Assertion::ephemeral())
    }

    /// Set the validity of all issued grants to the specified duration.
    ///
    /// This only affects tokens issued after this call. The default duration is 1 (ONE) hour for
    /// tokens issued for the authorization code grant method. For many users this may seem to
    /// short but should be secure-by-default. You may want to increase the duration, or instead
    /// use long lived refresh token instead (although you currently need to handle refresh tokens
    /// yourself, coming soonish).
    pub fn valid_for(&mut self, duration: Duration) {
        self.duration = Some(duration);
    }

    /// Set all grants to be valid for their default duration.
    ///
    /// This only affects tokens issued after this call. The default duration is 1 (ONE) hour for
    /// tokens issued for the authorization code grant method.
    pub fn valid_for_default(&mut self) {
        self.duration = None;
    }

    /// Determine whether to generate refresh tokens.
    ///
    /// By default, this option is *off*. Since the `TokenSigner` can on its own not revoke any
    /// tokens it should be considered carefullly whether to issue very long-living and powerful
    /// refresh tokens. On instance where this might be okay is as a component of a grander token
    /// architecture that adds a revocation mechanism.
    pub fn generate_refresh_tokens(&mut self, refresh: bool) {
        self.have_refresh = refresh;
    }

    /// Get the next counter value.
    fn next_counter(&self) -> usize {
        // Acquire+Release is overkill. We only need to ensure that each return value occurs at
        // most once. We would even be content with getting the counter out-of-order in a single
        // thread.
        self.counter.fetch_add(1, Ordering::Relaxed)
    }

    fn refreshable_token(&self, grant: &Grant) -> Result<IssuedToken, ()> {
        let first_ctr = self.next_counter() as u64;
        let second_ctr = self.next_counter() as u64;

        let token = self.as_token().sign(first_ctr, grant)?;
        let refresh = self.as_refresh().sign(second_ctr, grant)?;

        Ok(IssuedToken {
            token,
            refresh: Some(refresh),
            until: grant.until,
            token_type: TokenType::Bearer,
        })
    }

    fn unrefreshable_token(&self, grant: &Grant) -> Result<IssuedToken, ()> {
        let counter = self.next_counter() as u64;

        let token = self.as_token().sign(counter, grant)?;

        Ok(IssuedToken::without_refresh(token, grant.until))
    }

    fn as_token(&self) -> TaggedAssertion {
        self.signer.tag("token")
    }

    fn as_refresh(&self) -> TaggedAssertion {
        self.signer.tag("refresh")
    }
}

impl<'s, I: Issuer + ?Sized> Issuer for &'s mut I {
    fn issue(&mut self, grant: Grant) -> Result<IssuedToken, ()> {
        (**self).issue(grant)
    }

    fn refresh(&mut self, token: &str, grant: Grant) -> Result<RefreshedToken, ()> {
        (**self).refresh(token, grant)
    }

    fn recover_token<'a>(&'a self, token: &'a str) -> Result<Option<Grant>, ()> {
        (**self).recover_token(token)
    }

    fn recover_refresh<'a>(&'a self, token: &'a str) -> Result<Option<Grant>, ()> {
        (**self).recover_refresh(token)
    }
}

impl<I: Issuer + ?Sized> Issuer for Box<I> {
    fn issue(&mut self, grant: Grant) -> Result<IssuedToken, ()> {
        (**self).issue(grant)
    }

    fn refresh(&mut self, token: &str, grant: Grant) -> Result<RefreshedToken, ()> {
        (**self).refresh(token, grant)
    }

    fn recover_token<'a>(&'a self, token: &'a str) -> Result<Option<Grant>, ()> {
        (**self).recover_token(token)
    }

    fn recover_refresh<'a>(&'a self, token: &'a str) -> Result<Option<Grant>, ()> {
        (**self).recover_refresh(token)
    }
}

impl<'s, I: Issuer + ?Sized> Issuer for MutexGuard<'s, I> {
    fn issue(&mut self, grant: Grant) -> Result<IssuedToken, ()> {
        (**self).issue(grant)
    }

    fn refresh(&mut self, token: &str, grant: Grant) -> Result<RefreshedToken, ()> {
        (**self).refresh(token, grant)
    }

    fn recover_token<'a>(&'a self, token: &'a str) -> Result<Option<Grant>, ()> {
        (**self).recover_token(token)
    }

    fn recover_refresh<'a>(&'a self, token: &'a str) -> Result<Option<Grant>, ()> {
        (**self).recover_refresh(token)
    }
}

impl<'s, I: Issuer + ?Sized> Issuer for RwLockWriteGuard<'s, I> {
    fn issue(&mut self, grant: Grant) -> Result<IssuedToken, ()> {
        (**self).issue(grant)
    }

    fn refresh(&mut self, token: &str, grant: Grant) -> Result<RefreshedToken, ()> {
        (**self).refresh(token, grant)
    }

    fn recover_token<'a>(&'a self, token: &'a str) -> Result<Option<Grant>, ()> {
        (**self).recover_token(token)
    }

    fn recover_refresh<'a>(&'a self, token: &'a str) -> Result<Option<Grant>, ()> {
        (**self).recover_refresh(token)
    }
}

impl Issuer for TokenSigner {
    fn issue(&mut self, grant: Grant) -> Result<IssuedToken, ()> {
        (&mut &*self).issue(grant)
    }

    fn refresh(&mut self, _refresh: &str, _grant: Grant) -> Result<RefreshedToken, ()> {
        Err(())
    }

    fn recover_token<'a>(&'a self, token: &'a str) -> Result<Option<Grant>, ()> {
        (&&*self).recover_token(token)
    }

    fn recover_refresh<'a>(&'a self, token: &'a str) -> Result<Option<Grant>, ()> {
        (&&*self).recover_refresh(token)
    }
}

impl<'a> Issuer for &'a TokenSigner {
    fn issue(&mut self, mut grant: Grant) -> Result<IssuedToken, ()> {
        if let Some(duration) = &self.duration {
            grant.until = Utc::now() + *duration;
        }

        if self.have_refresh {
            self.refreshable_token(&grant)
        } else {
            self.unrefreshable_token(&grant)
        }
    }

    fn refresh(&mut self, _refresh: &str, _grant: Grant) -> Result<RefreshedToken, ()> {
        Err(())
    }

    fn recover_token<'t>(&'t self, token: &'t str) -> Result<Option<Grant>, ()> {
        Ok(self.as_token().extract(token).ok())
    }

    fn recover_refresh<'t>(&'t self, token: &'t str) -> Result<Option<Grant>, ()> {
        if !self.have_refresh {
            return Ok(None);
        }

        Ok(self.as_refresh().extract(token).ok())
    }
}

#[cfg(test)]
/// Tests for issuer implementations, including those provided here.
pub mod tests {
    use super::*;
    use crate::primitives::grant::Extensions;
    use crate::primitives::generator::RandomGenerator;
    use chrono::{Duration, Utc};
    use redis::cluster::ClusterClientBuilder;

    fn grant_template() -> Grant {
        Grant {
            client_id: "Client".to_string(),
            owner_id: "Owner".to_string(),
            redirect_uri: "https://example.com".parse().unwrap(),
            scope: "default".parse().unwrap(),
            until: Utc::now() + Duration::hours(1),
            extensions: Extensions::new(),
        }
    }

    /// Tests the simplest invariants that should be upheld by all authorizers.
    ///
    /// This create a token, without any extensions, an lets the issuer generate a issued token.
    /// The uri is `https://example.com` and the token lasts for an hour except if overwritten.
    /// Generation of a valid refresh token is not tested against.
    ///
    /// Custom implementations may want to import and use this in their own tests.
    pub fn simple_test_suite(issuer: &mut dyn Issuer) {
        let request = grant_template();

        let issued = issuer.issue(request.clone()).expect("Issuing failed");
        let from_token = issuer
            .recover_token(&issued.token)
            .expect("Issuer failed during recover")
            .expect("Issued token appears to be invalid");

        assert_ne!(Some(&issued.token), issued.refresh.as_ref());
        assert_eq!(from_token.client_id, "Client");
        assert_eq!(from_token.owner_id, "Owner");
        assert!(Utc::now() < from_token.until);

        let issued_2 = issuer.issue(request).expect("Issuing failed");
        assert_ne!(issued.token, issued_2.token);
        assert_ne!(Some(&issued.token), issued_2.refresh.as_ref());
        assert_ne!(issued.refresh, issued_2.refresh);
        assert_ne!(issued.refresh.as_ref(), Some(&issued_2.token));
    }

    #[test]
    fn signer_test_suite() {
        let mut signer = TokenSigner::ephemeral();
        // Refresh tokens must be unique if generated. If they are not even generated, they are
        // obviously not unique.
        signer.generate_refresh_tokens(true);
        simple_test_suite(&mut signer);
    }

    #[test]
    fn signer_no_default_refresh() {
        let mut signer = TokenSigner::ephemeral();
        let issued = signer.issue(grant_template());

        let token = issued.expect("Issuing without refresh token failed");
        assert!(!token.refreshable());
    }

    #[test]
    fn random_test_suite() {
        let client = get_local_redis();
        let mut token_map = TokenMap::new(RandomGenerator::new(16), Arc::new(client));
        simple_test_suite(&mut token_map);
    }

    #[test]
    fn random_has_refresh() {
        let client = get_local_redis();
        let mut token_map = TokenMap::new(RandomGenerator::new(16), Arc::new(client));
        let issued = token_map.issue(grant_template());

        let token = issued.expect("Issuing without refresh token failed");
        assert!(token.refreshable());
    }

    #[test]
    #[should_panic]
    fn bad_generator() {
        struct BadGenerator;
        impl TagGrant for BadGenerator {
            fn tag(&mut self, _: u64, _: &Grant) -> Result<String, ()> {
                Ok("YOLO.HowBadCanItBeToRepeatTokens?".into())
            }
        }
        let client = get_local_redis();
        let mut token_map = TokenMap::new(BadGenerator, Arc::new(client));
        simple_test_suite(&mut token_map);
    }

    pub fn get_local_redis() -> ClusterClient {
        let builder = ClusterClientBuilder::new(vec!["redis://127.0.0.1:6379"]);
        builder.open().map_err(|err|{
            error!("{}", err.to_string());
            err
        }).unwrap()
    }
}