use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use axum_extra::extract::CookieJar;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{
    api::cookies::{
        access_cookie, clear_access_cookie, clear_refresh_cookie, refresh_cookie, REFRESH_COOKIE,
    },
    auth::{
        dummy_verify, hash_password, should_delete_session_on_refresh, verify_password,
        SessionTokens,
    },
    models::User,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub role: String,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self {
            id: u.id.to_string(),
            email: u.email,
            role: u.role,
        }
    }
}

fn client_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "unknown".into())
}

fn rate_limited(retry_after: u64) -> Response {
    let mut res = (StatusCode::TOO_MANY_REQUESTS, Json(serde_json::json!({
        "error": "too_many_requests"
    }))).into_response();
    if let Ok(v) = HeaderValue::from_str(&retry_after.to_string()) {
        res.headers_mut().insert("retry-after", v);
    }
    res
}

#[instrument(skip(state, jar, body))]
pub async fn register(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<RegisterRequest>,
) -> Response {
    let ip = client_ip(&headers);
    let email = body.email.trim().to_lowercase();

    if let Err(e) = state.register_ip_limiter.check_and_hit(&format!("ip:{ip}")) {
        return rate_limited(e.retry_after_secs);
    }
    if let Err(e) = state
        .register_account_limiter
        .check_and_hit(&format!("acct:{email}"))
    {
        return rate_limited(e.retry_after_secs);
    }

    let password_hash = match hash_password(&body.password, &state.config.password_pepper) {
        Ok(h) => h,
        Err(crate::auth::PasswordError::TooShort) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "password_too_short"})),
            )
                .into_response();
        }
        Err(_) => {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let user = match state
        .db
        .create_user(&email, &password_hash, "user")
        .await
    {
        Ok(u) => u,
        Err(sqlx::Error::Database(db)) if db.constraint() == Some("users_email_key") => {
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({"error": "email_taken"})),
            )
                .into_response();
        }
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    issue_session_response(&state, jar, user).await
}

#[instrument(skip(state, jar, body))]
pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<LoginRequest>,
) -> Response {
    let ip = client_ip(&headers);
    let email = body.email.trim().to_lowercase();

    if let Err(e) = state.login_ip_limiter.check_and_hit(&format!("ip:{ip}")) {
        return rate_limited(e.retry_after_secs);
    }
    if let Err(e) = state
        .login_account_limiter
        .check_and_hit(&format!("acct:{email}"))
    {
        return rate_limited(e.retry_after_secs);
    }

    let user = match state.db.find_user_by_email(&email).await {
        Ok(u) => u,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let Some(user) = user else {
        dummy_verify(&state.config.password_pepper);
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "invalid_credentials"})),
        )
            .into_response();
    };

    if verify_password(
        &body.password,
        &state.config.password_pepper,
        &user.password_hash,
    )
    .is_err()
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "invalid_credentials"})),
        )
            .into_response();
    }

    issue_session_response(&state, jar, user).await
}

async fn issue_session_response(state: &AppState, jar: CookieJar, user: User) -> Response {
    let tokens = SessionTokens::issue(Utc::now());
    if state.db.create_session(user.id, &tokens).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let secure = state.config.cookie_secure;
    let jar = jar
        .add(access_cookie(&tokens.access_token, secure))
        .add(refresh_cookie(&tokens.refresh_token, secure));
    (jar, Json(UserResponse::from(user))).into_response()
}

#[instrument(skip(state, jar))]
pub async fn refresh(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(refresh) = jar.get(REFRESH_COOKIE).map(|c| c.value().to_string()) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let Some(session) = (match state.db.find_session_by_refresh(&refresh).await {
        Ok(s) => s,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let now = Utc::now();
    if should_delete_session_on_refresh(now, session.refresh_expires_at) {
        let _ = state.db.delete_session(session.id).await;
        let secure = state.config.cookie_secure;
        let jar = jar
            .add(clear_access_cookie(secure))
            .add(clear_refresh_cookie(secure));
        return (StatusCode::UNAUTHORIZED, jar).into_response();
    }

    let tokens = SessionTokens::rotate(now);
    if state.db.rotate_session(session.id, &tokens).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let user = match state.db.find_user_by_id(session.user_id).await {
        Ok(Some(u)) => u,
        _ => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let secure = state.config.cookie_secure;
    let jar = jar
        .add(access_cookie(&tokens.access_token, secure))
        .add(refresh_cookie(&tokens.refresh_token, secure));
    (jar, Json(UserResponse::from(user))).into_response()
}

#[instrument(skip(state, jar))]
pub async fn logout(State(state): State<AppState>, jar: CookieJar) -> Response {
    if let Some(refresh) = jar.get(REFRESH_COOKIE).map(|c| c.value().to_string()) {
        let _ = state.db.delete_session_by_refresh(&refresh).await;
    }
    let secure = state.config.cookie_secure;
    let jar = jar
        .add(clear_access_cookie(secure))
        .add(clear_refresh_cookie(secure));
    (StatusCode::NO_CONTENT, jar).into_response()
}

#[instrument(skip(user))]
pub async fn me(user: crate::api::extractors::AuthUser) -> Json<UserResponse> {
    Json(UserResponse::from(user.0))
}
