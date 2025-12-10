//! Request extractors.

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use misskey_db::entities::user;

/// Authenticated user extractor.
#[derive(Debug, Clone)]
pub struct AuthUser(pub user::Model);

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Get user from request extensions (set by auth middleware)
        parts
            .extensions
            .get::<user::Model>()
            .cloned()
            .map(AuthUser)
            .ok_or((StatusCode::UNAUTHORIZED, "Unauthorized"))
    }
}

/// Optional authenticated user extractor.
#[derive(Debug, Clone)]
pub struct MaybeAuthUser(pub Option<user::Model>);

impl<S> FromRequestParts<S> for MaybeAuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(parts.extensions.get::<user::Model>().cloned()))
    }
}
