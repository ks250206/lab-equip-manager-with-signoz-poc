use axum_extra::extract::cookie::{Cookie, SameSite};
use time::Duration as TimeDuration;

use crate::auth::{ACCESS_TTL, REFRESH_TTL};

pub const ACCESS_COOKIE: &str = "access_token";
pub const REFRESH_COOKIE: &str = "refresh_token";

pub fn access_cookie(token: &str, secure: bool) -> Cookie<'static> {
    Cookie::build((ACCESS_COOKIE, token.to_string()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(secure)
        .max_age(TimeDuration::seconds(ACCESS_TTL.num_seconds()))
        .build()
}

pub fn refresh_cookie(token: &str, secure: bool) -> Cookie<'static> {
    Cookie::build((REFRESH_COOKIE, token.to_string()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(secure)
        .max_age(TimeDuration::seconds(REFRESH_TTL.num_seconds()))
        .build()
}

pub fn clear_access_cookie(secure: bool) -> Cookie<'static> {
    Cookie::build((ACCESS_COOKIE, ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(secure)
        .max_age(TimeDuration::seconds(0))
        .build()
}

pub fn clear_refresh_cookie(secure: bool) -> Cookie<'static> {
    Cookie::build((REFRESH_COOKIE, ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(secure)
        .max_age(TimeDuration::seconds(0))
        .build()
}
