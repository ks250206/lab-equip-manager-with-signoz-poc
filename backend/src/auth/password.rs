//! Password hashing: HMAC-SHA-256(pepper) then Argon2id.

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use thiserror::Error;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PasswordError {
    #[error("password must be at least 12 characters")]
    TooShort,
    #[error("password hashing failed")]
    HashFailed,
    #[error("password verification failed")]
    VerifyFailed,
}

/// Mix password with pepper via HMAC-SHA-256, then Argon2id-hash the result.
pub fn hash_password(password: &str, pepper: &[u8]) -> Result<String, PasswordError> {
    if password.chars().count() < 12 {
        return Err(PasswordError::TooShort);
    }
    let mixed = mix_with_pepper(password, pepper)?;
    let salt = SaltString::generate(&mut rand::thread_rng());
    let argon2 = Argon2::default();
    argon2
        .hash_password(&mixed, &salt)
        .map(|h| h.to_string())
        .map_err(|_| PasswordError::HashFailed)
}

pub fn verify_password(password: &str, pepper: &[u8], encoded: &str) -> Result<(), PasswordError> {
    let mixed = mix_with_pepper(password, pepper)?;
    let parsed = PasswordHash::new(encoded).map_err(|_| PasswordError::VerifyFailed)?;
    Argon2::default()
        .verify_password(&mixed, &parsed)
        .map_err(|_| PasswordError::VerifyFailed)
}

/// Run a dummy Argon2id verify against a fixed hash to reduce timing leaks
/// when the account does not exist.
pub fn dummy_verify(pepper: &[u8]) {
    let dummy_password = "timing-safe-dummy-password-xx";
    let Ok(hash) = hash_password(dummy_password, pepper) else {
        return;
    };
    let _ = verify_password("wrong-password-xxxxxxxxxxxxxxx", pepper, &hash);
}

fn mix_with_pepper(password: &str, pepper: &[u8]) -> Result<Vec<u8>, PasswordError> {
    let mut mac = HmacSha256::new_from_slice(pepper).map_err(|_| PasswordError::HashFailed)?;
    mac.update(password.as_bytes());
    Ok(mac.finalize().into_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PEPPER: &[u8] = b"test-pepper-key-for-unit-tests";

    #[test]
    fn rejects_short_password() {
        let err = hash_password("short", PEPPER).unwrap_err();
        assert_eq!(err, PasswordError::TooShort);
    }

    #[test]
    fn hashes_and_verifies_password() {
        let hash = hash_password("correct-horse-battery", PEPPER).unwrap();
        assert!(verify_password("correct-horse-battery", PEPPER, &hash).is_ok());
        assert!(verify_password("wrong-password-xxxx", PEPPER, &hash).is_err());
    }

    #[test]
    fn pepper_changes_hash_input() {
        let hash = hash_password("correct-horse-battery", PEPPER).unwrap();
        assert!(verify_password("correct-horse-battery", b"other-pepper", &hash).is_err());
    }

    #[test]
    fn dummy_verify_runs_without_panic() {
        dummy_verify(PEPPER);
    }
}
