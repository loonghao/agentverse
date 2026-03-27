use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum JwtError {
    #[error("invalid token: {0}")]
    Invalid(#[from] jsonwebtoken::errors::Error),
    #[error("token expired")]
    Expired,
}

/// JWT claims embedded in every access token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject = user UUID
    pub sub: Uuid,
    pub username: String,
    /// "human" | "agent" | "system"
    pub kind: String,
    /// Expiry (Unix timestamp)
    pub exp: i64,
    /// Issued-at (Unix timestamp)
    pub iat: i64,
}

pub struct JwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    access_expiry_secs: i64,
}

impl JwtManager {
    pub fn new(secret: &str, access_expiry_secs: i64) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            access_expiry_secs,
        }
    }

    /// Generate a signed JWT access token.
    pub fn generate(&self, user_id: Uuid, username: &str, kind: &str) -> Result<String, JwtError> {
        let now = Utc::now();
        let claims = Claims {
            sub: user_id,
            username: username.to_string(),
            kind: kind.to_string(),
            iat: now.timestamp(),
            exp: (now + Duration::seconds(self.access_expiry_secs)).timestamp(),
        };
        Ok(encode(&Header::default(), &claims, &self.encoding_key)?)
    }

    /// Validate and decode a JWT token.
    pub fn validate(&self, token: &str) -> Result<Claims, JwtError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        // Set leeway to 0 so tokens are rejected immediately on expiry.
        validation.leeway = 0;
        let data = decode::<Claims>(token, &self.decoding_key, &validation)?;
        Ok(data.claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let mgr = JwtManager::new("test-secret", 3600);
        let id = Uuid::new_v4();
        let token = mgr.generate(id, "testuser", "human").unwrap();
        let claims = mgr.validate(&token).unwrap();
        assert_eq!(claims.sub, id);
        assert_eq!(claims.username, "testuser");
    }

    #[test]
    fn expired_token_rejected() {
        let mgr = JwtManager::new("test-secret", -1); // already expired
        let token = mgr.generate(Uuid::new_v4(), "u", "human").unwrap();
        assert!(mgr.validate(&token).is_err());
    }
}
