use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PasswordError {
    #[error("hashing failed: {0}")]
    Hash(String),
    #[error("invalid password")]
    Invalid,
}

pub struct PasswordManager;

impl PasswordManager {
    /// Hash a plaintext password using Argon2id.
    pub fn hash(password: &str) -> Result<String, PasswordError> {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| PasswordError::Hash(e.to_string()))
    }

    /// Verify a plaintext password against a stored Argon2 hash.
    pub fn verify(password: &str, hash: &str) -> Result<(), PasswordError> {
        let parsed = PasswordHash::new(hash).map_err(|e| PasswordError::Hash(e.to_string()))?;
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .map_err(|_| PasswordError::Invalid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify() {
        let pw = "super-secret-password";
        let hash = PasswordManager::hash(pw).unwrap();
        assert!(PasswordManager::verify(pw, &hash).is_ok());
        assert!(PasswordManager::verify("wrong-password", &hash).is_err());
    }
}

