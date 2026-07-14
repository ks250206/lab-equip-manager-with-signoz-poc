//! Opaque token generation and SHA-256 hashing for session storage.

use rand::RngCore;
use sha2::{Digest, Sha256};

pub fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub fn hash_token(token: &str) -> [u8; 32] {
    let digest = Sha256::digest(token.as_bytes());
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_unique_and_hashed_stably() {
        let a = generate_token();
        let b = generate_token();
        assert_ne!(a, b);
        assert_eq!(hash_token(&a), hash_token(&a));
        assert_ne!(hash_token(&a), hash_token(&b));
        assert_eq!(hash_token(&a).len(), 32);
    }
}
