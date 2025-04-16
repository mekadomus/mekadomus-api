use crate::{
    error::app_error::{unauthorized, AppError},
    helper::token::AUTH_TOKEN_LEN,
    AppState,
};

use async_trait::async_trait;
use axum::{
    body::Body,
    extract::State,
    http::{header::AUTHORIZATION, Request},
    middleware::Next,
    response::Response,
};
use http::method::Method;
use mockall::automock;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use tracing::error;

static PUBLIC_PATHS: Lazy<HashMap<&str, HashSet<Method>>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("/health", HashSet::from([Method::GET, Method::HEAD]));
    m.insert("/v1/alert", HashSet::from([Method::POST]));
    m.insert("/v1/email-verification", HashSet::from([Method::GET]));
    m.insert("/v1/log-in", HashSet::from([Method::POST]));
    m.insert("/v1/measurement", HashSet::from([Method::POST]));
    m.insert("/v1/new-password", HashSet::from([Method::POST]));
    m.insert("/v1/recover-password", HashSet::from([Method::GET]));
    m.insert("/v1/sign-up", HashSet::from([Method::POST]));
    return m;
});

#[automock]
#[async_trait]
pub trait Authorizer: Send + Sync {
    async fn authorize(&self, state: AppState, request: &mut Request<Body>) -> bool;
}

pub struct DefaultAuthorizer;

#[async_trait]
impl Authorizer for DefaultAuthorizer {
    async fn authorize(&self, state: AppState, request: &mut Request<Body>) -> bool {
        let method = request.method();
        let path = request.uri().path();
        if PUBLIC_PATHS.contains_key(path) && PUBLIC_PATHS.get(path).unwrap().contains(method) {
            return true;
        }

        let auth_header = match request.headers().get(AUTHORIZATION) {
            Some(h) => h.to_str().unwrap(),
            None => return false,
        };
        let parts: Vec<&str> = auth_header.split_whitespace().collect();
        if parts.len() != 2 {
            return false;
        }

        match parts[0] {
            "Bearer" => {
                let token = parts[1];
                if token.len() < *AUTH_TOKEN_LEN {
                    return false;
                }
                match state.storage.user_by_token(token).await {
                    Ok(u) => {
                        if u.is_none() {
                            return false;
                        }
                        let mut user = u.unwrap();
                        user.password = None;
                        request.extensions_mut().insert(user);
                        return true;
                    }
                    Err(e) => {
                        error!("Failed to authorize user: {}", e);
                        return false;
                    }
                }
            }
            _ => {
                return false;
            }
        }
    }
}

pub async fn auth(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    if state
        .authorizer
        .clone()
        .authorize(state, &mut request)
        .await
    {
        return Ok(next.run(request).await);
    } else {
        return unauthorized();
    }
}
