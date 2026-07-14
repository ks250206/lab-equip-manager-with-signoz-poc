//! Session token lifetimes and rotation helpers.

use chrono::{DateTime, Duration, Utc};

use super::token::{generate_token, hash_token};

pub const ACCESS_TTL: Duration = Duration::minutes(15);
pub const REFRESH_TTL: Duration = Duration::days(30);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub access_token_hash: [u8; 32],
    pub refresh_token_hash: [u8; 32],
    pub access_expires_at: DateTime<Utc>,
    pub refresh_expires_at: DateTime<Utc>,
}

impl SessionTokens {
    pub fn issue(now: DateTime<Utc>) -> Self {
        let access_token = generate_token();
        let refresh_token = generate_token();
        Self {
            access_token_hash: hash_token(&access_token),
            refresh_token_hash: hash_token(&refresh_token),
            access_token,
            refresh_token,
            access_expires_at: now + ACCESS_TTL,
            refresh_expires_at: now + REFRESH_TTL,
        }
    }

    /// Rotate both access and refresh tokens (called on refresh).
    pub fn rotate(now: DateTime<Utc>) -> Self {
        Self::issue(now)
    }
}

/// Access expired but refresh still valid → keep session, only refresh path renews.
pub fn should_delete_session_on_refresh(
    now: DateTime<Utc>,
    refresh_expires_at: DateTime<Utc>,
) -> bool {
    now >= refresh_expires_at
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_sets_expected_ttls() {
        let now = Utc::now();
        let tokens = SessionTokens::issue(now);
        assert_eq!(tokens.access_expires_at, now + ACCESS_TTL);
        assert_eq!(tokens.refresh_expires_at, now + REFRESH_TTL);
        assert_ne!(tokens.access_token, tokens.refresh_token);
    }

    #[test]
    fn rotate_issues_new_pair() {
        let now = Utc::now();
        let a = SessionTokens::issue(now);
        let b = SessionTokens::rotate(now);
        assert_ne!(a.access_token, b.access_token);
        assert_ne!(a.refresh_token, b.refresh_token);
    }

    #[test]
    fn delete_only_when_refresh_expired() {
        let now = Utc::now();
        assert!(!should_delete_session_on_refresh(now, now + Duration::hours(1)));
        assert!(should_delete_session_on_refresh(now, now - Duration::seconds(1)));
    }
}
