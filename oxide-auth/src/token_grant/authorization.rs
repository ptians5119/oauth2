//! Provides the handling for Authorization Code Requests
use std::borrow::Cow;
use std::result::Result as StdResult;

use url::Url;
use chrono::{Duration, Utc};

use crate::primitives::issuer::{IssuedToken, Issuer};
use crate::primitives::authorizer::Authorizer;
use crate::primitives::registrar::{ClientUrl, ExactUrl, Registrar, RegistrarError, PreGrant};
use crate::primitives::grant::{Extensions, Grant};
use crate::{endpoint::Scope, endpoint::Solicitation, primitives::registrar::BoundClient};
use crate::code_grant::accesstoken::{Error, BearerToken};
use crate::code_grant::authorization::{Endpoint, Request};
use crate::code_grant::error::*;
use std::ops::Add;

type Result<T> = std::result::Result<T, Error>;


pub fn authorization_token(handler: &mut dyn Endpoint, request: &dyn Request) -> self::Result<BearerToken> {

    // 校验client_id 和 redirect url
    let client_id = request.client_id().ok_or(Error::invalid_with(AccessTokenErrorType::InvalidRequest))?;
    let redirect_uri: Option<Cow<ExactUrl>> = match request.redirect_uri() {
        None => None,
        Some(ref uri) => {
            let parsed = uri.parse().map_err(|_| Error::invalid_with(AccessTokenErrorType::InvalidRequest))?;
            Some(Cow::Owned(parsed))
        }
    };
    let client_url = ClientUrl {
        client_id: client_id,
        redirect_uri: redirect_uri,
    };
    let bound_client = match handler.registrar().bound_redirect(client_url.clone()) {
        Err(RegistrarError::Unspecified) => {
            error!("unspecified");
            return Err(Error::invalid_with(AccessTokenErrorType::UnauthorizedClient));
        }
        Err(RegistrarError::PrimitiveError) => return Err(Error::invalid_with(AccessTokenErrorType::UnauthorizedClient)),
        Ok(client) => client,
    };

    // 解析scope
    let scope = request.scope();
    let scope = if let Some(scope) = scope {
        let scope = scope.replace("%20", " ");
        Some(Cow::from(scope))
    } else {
        None
    };
    let scope = match scope.map(|scope| scope.as_ref().parse()) {
        None => None,
        Some(Err(_)) => {
            return Err(Error::invalid_with(AccessTokenErrorType::InvalidScope));
        }
        Some(Ok(scope)) => Some(scope),
    };

    debug!("scope = {:?}", scope);

    let pre_grant = handler
        .registrar()
        .negotiate(bound_client, scope)
        .map_err(|err| match err {
            RegistrarError::PrimitiveError => Error::invalid_with(AccessTokenErrorType::InvalidGrant),
            RegistrarError::Unspecified => {
                Error::invalid_with(AccessTokenErrorType::InvalidRequest)
            }
        })?;

    debug!("pre_grant={:?}", pre_grant);

    let grant = Grant {
        owner_id: request.user_id().unwrap_or_default().to_string(),
        client_id: pre_grant.client_id,
        scope: pre_grant.scope,
        redirect_uri: pre_grant.redirect_uri.into_url(),
        until: Utc::now() + Duration::seconds(3600),
        extensions: Extensions::new(),
    };

    // 获取token
    let token = handler.issuer().issue(grant.clone()).map_err(|_| {
        error!("err on issuer issue token by grant");
        Error::invalid_with(AccessTokenErrorType::ServerError)
    })?;

    let mut token = IssuedToken::without_refresh(token.token, token.until);

    info!("client={:?} token={:?}", client_url.clone(), token.clone());

    Ok(BearerToken(token, grant.scope.to_string()))
}
