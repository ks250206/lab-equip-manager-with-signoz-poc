use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use axum_extra::extract::CookieJar;
use chrono::Utc;

use crate::{
    api::cookies::ACCESS_COOKIE,
    models::User,
    state::AppState,
};

#[derive(Debug)]
pub struct AuthUser(pub User);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let Some(token) = jar.get(ACCESS_COOKIE).map(|c| c.value().to_string()) else {
            return Err(StatusCode::UNAUTHORIZED);
        };

        let Some((session, user)) = state
            .db
            .find_session_user_by_access(&token)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        else {
            return Err(StatusCode::UNAUTHORIZED);
        };

        if Utc::now() >= session.access_expires_at {
            // Access expired: do not delete session if refresh may still be valid.
            return Err(StatusCode::UNAUTHORIZED);
        }

        Ok(AuthUser(user))
    }
}
