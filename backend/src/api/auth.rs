use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use axum_extra::extract::CookieJar;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use tracing::instrument;

use crate::{
    api::cookies::{
        access_cookie, clear_access_cookie, clear_refresh_cookie, refresh_cookie, REFRESH_COOKIE,
    },
    auth::{
        dummy_verify, hash_password, should_delete_session_on_refresh, verify_password,
        SessionTokens,
    },
    config::Config,
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

/// Resolve client IP for rate limiting.
/// When the TCP peer is a trusted proxy, prefer `X-Forwarded-For` / `X-Real-IP`.
pub fn client_ip(peer: SocketAddr, headers: &HeaderMap, config: &Config) -> String {
    if config.is_trusted_proxy(peer.ip()) {
        if let Some(ip) = forwarded_client_ip(headers) {
            return ip.to_string();
        }
    }
    peer.ip().to_string()
}

fn forwarded_client_ip(headers: &HeaderMap) -> Option<IpAddr> {
    if let Some(xff) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        // Leftmost is the original client when each hop appends.
        if let Some(first) = xff.split(',').next() {
            if let Ok(ip) = first.trim().parse::<IpAddr>() {
                return Some(ip);
            }
        }
    }
    headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse().ok())
}

fn rate_limited(retry_after: u64) -> Response {
    let mut res = (
        StatusCode::TOO_MANY_REQUESTS,
        Json(serde_json::json!({
            "error": "too_many_requests"
        })),
    )
        .into_response();
    if let Ok(v) = HeaderValue::from_str(&retry_after.to_string()) {
        res.headers_mut().insert("retry-after", v);
    }
    res
}

#[instrument(skip(state, jar, body))]
pub async fn register(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(body): Json<RegisterRequest>,
) -> Response {
    let ip = client_ip(peer, &headers, &state.config);
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

    let user = match state.db.create_user(&email, &password_hash, "user").await {
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
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(body): Json<LoginRequest>,
) -> Response {
    let ip = client_ip(peer, &headers, &state.config);
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
    let secure = state.config.cookie_secure;
    if should_delete_session_on_refresh(now, session.refresh_expires_at) {
        let _ = state.db.delete_session(session.id).await;
        let jar = jar
            .add(clear_access_cookie(secure))
            .add(clear_refresh_cookie(secure));
        return (StatusCode::UNAUTHORIZED, jar).into_response();
    }

    let tokens = SessionTokens::rotate(now);
    match state.db.rotate_session(session.id, &refresh, &tokens).await {
        Ok(true) => {}
        Ok(false) => {
            // Refresh reuse / race: invalidate the whole session (reuse detection).
            let _ = state.db.delete_session(session.id).await;
            let jar = jar
                .add(clear_access_cookie(secure))
                .add(clear_refresh_cookie(secure));
            return (StatusCode::UNAUTHORIZED, jar).into_response();
        }
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }

    let user = match state.db.find_user_by_id(session.user_id).await {
        Ok(Some(u)) => u,
        _ => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use ipnet::IpNet;

    fn cfg_with_proxies(cidrs: &[&str]) -> Config {
        Config {
            database_url: String::new(),
            password_pepper: b"x".to_vec(),
            cookie_secure: false,
            bind_addr: "0.0.0.0:3000".into(),
            frontend_origin: "http://localhost:5173".into(),
            garage_endpoint: String::new(),
            garage_region: "garage".into(),
            garage_access_key: String::new(),
            garage_secret_key: String::new(),
            garage_bucket: String::new(),
            otel_endpoint: String::new(),
            service_name: "test".into(),
            trusted_proxies: cidrs.iter().map(|s| s.parse::<IpNet>().unwrap()).collect(),
        }
    }

    #[test]
    fn uses_peer_ip_when_not_trusted_proxy() {
        let cfg = cfg_with_proxies(&["10.0.0.0/8"]);
        let peer: SocketAddr = "203.0.113.10:443".parse().unwrap();
        let headers = HeaderMap::new();
        assert_eq!(client_ip(peer, &headers, &cfg), "203.0.113.10");
    }

    #[test]
    fn uses_xff_when_peer_is_trusted_proxy() {
        let cfg = cfg_with_proxies(&["10.0.0.0/8"]);
        let peer: SocketAddr = "10.89.0.2:443".parse().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("203.0.113.50, 10.89.0.2"),
        );
        assert_eq!(client_ip(peer, &headers, &cfg), "203.0.113.50");
    }

    #[test]
    fn ignores_xff_from_untrusted_peer() {
        let cfg = cfg_with_proxies(&["10.0.0.0/8"]);
        let peer: SocketAddr = "203.0.113.10:443".parse().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static("1.2.3.4"));
        assert_eq!(client_ip(peer, &headers, &cfg), "203.0.113.10");
    }
}
