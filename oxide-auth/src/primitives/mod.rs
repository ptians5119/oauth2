//! A collection of primites useful for more than one authorization method.
//!
//! A primitive is the smallest independent unit of policy used in OAuth related endpoints. For
//! example, an `authorizer` generates and verifies Authorization Codes.  There only is, as you
//! might have noticed, only the OAuth2 code grant method. But abstracting away the underlying
//! primitives makes it possible to provide –e.g.– a independent database based implementation.
//!
//! These should be used to build or instantiate an `Endpoint`, for example [`Generic`] or your
//! own.
//!
//! ```
//! # extern crate oxide_auth;
//! # use oxide_auth::frontends::simple::endpoint::Vacant;
//! use oxide_auth::frontends::simple::endpoint::Generic;
//! use oxide_auth::primitives::{
//!     authorizer::AuthMap,
//!     generator::RandomGenerator,
//!     issuer::TokenMap,
//!     registrar::ClientMap,
//! };
//!
//! Generic {
//!     authorizer: AuthMap::new(RandomGenerator::new(16)),
//!     registrar: ClientMap::new(),
//!     issuer: TokenMap::new(RandomGenerator::new(16)),
//!     // ...
//! #   scopes: Vacant,
//! #   solicitor: Vacant,
//! #   response: Vacant,
//! };
//! ```
//!
//! [`Generic`]: ../frontends/simple/endpoint/struct.Generic.html

use chrono::DateTime;
use chrono::Utc;
use url::Url;
use crate::primitives::grant::Grant;
use redis::{cluster::ClusterConnection, Commands};

pub mod authorizer;
pub mod generator;
pub mod grant;
pub mod issuer;
pub mod registrar;
pub mod scope;

type Time = DateTime<Utc>;

/// Commonly used primitives for frontends and backends.
pub mod prelude {
    pub use super::authorizer::{Authorizer, AuthMap};
    pub use super::issuer::{IssuedToken, Issuer, TokenMap, TokenSigner};
    pub use super::generator::{Assertion, TagGrant, RandomGenerator};
    pub use super::registrar::{Registrar, Client, ClientUrl, ClientMap, PreGrant};
    pub use super::scope::Scope;
}

/// 从redis内获得grant信息
pub fn get_grant(connection: &mut ClusterConnection, key: &str) -> Result<Option<Grant>, ()> {
    let owner_id = connection.hget::<&str, &str, String>(key, "owner_id")
        .map_err(|err| {
            error!("get owner_id error: {}", err.to_string());
            ()
        })?;
    let client_id = connection.hget::<&str, &str, String>(key, "client_id")
        .map_err(|err| {
            error!("get client_id error: {}", err.to_string());
            ()
        })?;
    let scope = connection.hget::<&str, &str, String>(key, "scope")
        .map_err(|err| {
            error!("get scope error: {}", err.to_string());
            ()
        })?;
    let scope = serde_json::from_str::<scope::Scope>(&scope)
        .map_err(|err| {
            error!("parse scope error: {}", err.to_string());
            ()
        })?;
    let redirect_uri = connection.hget::<&str, &str, String>(key, "redirect_uri")
        .map_err(|err| {
            error!("get redirect_uri error: {}", err.to_string());
            ()
        })?;
    let redirect_uri = serde_json::from_str::<Url>(&redirect_uri)
        .map_err(|err| {
            error!("parse redirect_uri error: {}", err.to_string());
            ()
        })?;
    let until = connection.hget::<&str, &str, String>(key, "until")
        .map_err(|err| {
            error!("get until error: {}", err.to_string());
            ()
        })?;
    let until = serde_json::from_str::<Time>(&until)
        .map_err(|err| {
            error!("parse until error: {}", err.to_string());
            ()
        })?;
    let extensions = connection.hget::<&str, &str, String>(key, "extensions")
        .map_err(|err| {
            error!("get extensions error: {}", err.to_string());
            ()
        })?;
    let extensions = serde_json::from_str::<grant::Extensions>(&extensions)
        .map_err(|err| {
            error!("parse extensions error: {}", err.to_string());
            ()
        })?;
    Ok(Some(Grant {
        owner_id,
        client_id,
        scope,
        redirect_uri,
        until,
        extensions
    }))
}

/// 设置grant
pub fn set_grant(connection: &mut ClusterConnection, key: &str, grant: &Grant, expire: usize) -> Result<(), ()>
{
    match connection.hset::<&str, &str, String, usize>(
        key, "owner_id", grant.owner_id.to_owned()) {
        Ok(1) => (),
        _ => {
            error!("set owner_id error");
            return Err(())
        }
    }
    match connection.hset::<&str, &str, String, usize>(
        key, "client_id", grant.client_id.to_owned()) {
        Ok(1) => (),
        _ => {
            error!("set client_id error");
            return Err(())
        }
    }
    match connection.hset::<&str, &str, String, usize>(
        key, "scope", serde_json::to_string(&grant.scope).unwrap()) {
        Ok(1) => (),
        _ => {
            error!("set scope error");
            return Err(())
        }
    }
    match connection.hset::<&str, &str, String, usize>(
        key, "redirect_uri", serde_json::to_string(&grant.redirect_uri).unwrap()) {
        Ok(1) => (),
        _ => {
            error!("set redirect_uri error");
            return Err(())
        }
    }
    match connection.hset::<&str, &str, String, usize>(
        key, "until", serde_json::to_string(&grant.until).unwrap()) {
        Ok(1) => (),
        _ => {
            error!("set until error");
            return Err(())
        }
    }
    match connection.hset::<&str, &str, String, usize>(
        key, "extensions", serde_json::to_string(&grant.extensions).unwrap()) {
        Ok(1) => (),
        _ => {
            error!("set extensions error");
            return Err(())
        }
    }
    // 设置过期时间
    match connection.expire::<&str, usize>(key, expire) {
        Ok(1) => (),
        _ => {
            error!("set expire error");
            return Err(())
        }
    }

    Ok(())
}