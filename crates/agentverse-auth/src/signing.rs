use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SigningError {
    #[error("invalid key material: {0}")]
    InvalidKey(String),
    #[error("signature verification failed")]
    VerificationFailed,
    #[error("hex decode error: {0}")]
    HexDecode(String),
}

/// Ed25519 signing manager for manifest integrity.
pub struct SigningManager {
    signing_key: SigningKey,
}

impl SigningManager {
    /// Generate a new random keypair.
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        Self { signing_key }
    }

    /// Load from 32-byte hex-encoded private key seed.
    pub fn from_hex(private_key_hex: &str) -> Result<Self, SigningError> {
        let bytes =
            hex::decode(private_key_hex).map_err(|e| SigningError::HexDecode(e.to_string()))?;
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| SigningError::InvalidKey("must be 32 bytes".into()))?;
        Ok(Self {
            signing_key: SigningKey::from_bytes(&arr),
        })
    }

    /// Sign arbitrary bytes; returns lowercase hex signature.
    pub fn sign(&self, data: &[u8]) -> String {
        let sig: Signature = self.signing_key.sign(data);
        hex::encode(sig.to_bytes())
    }

    /// Export the public key as lowercase hex.
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.signing_key.verifying_key().to_bytes())
    }

    /// Export private key seed as hex (for storage).
    pub fn private_key_hex(&self) -> String {
        hex::encode(self.signing_key.to_bytes())
    }

    /// Verify a hex-encoded signature against the given public key hex.
    pub fn verify(
        public_key_hex: &str,
        data: &[u8],
        signature_hex: &str,
    ) -> Result<(), SigningError> {
        let pk_bytes =
            hex::decode(public_key_hex).map_err(|e| SigningError::HexDecode(e.to_string()))?;
        let pk_arr: [u8; 32] = pk_bytes
            .try_into()
            .map_err(|_| SigningError::InvalidKey("public key must be 32 bytes".into()))?;
        let verifying_key = VerifyingKey::from_bytes(&pk_arr)
            .map_err(|e| SigningError::InvalidKey(e.to_string()))?;

        let sig_bytes =
            hex::decode(signature_hex).map_err(|e| SigningError::HexDecode(e.to_string()))?;
        let sig_arr: [u8; 64] = sig_bytes
            .try_into()
            .map_err(|_| SigningError::InvalidKey("signature must be 64 bytes".into()))?;
        let sig = Signature::from_bytes(&sig_arr);

        verifying_key
            .verify(data, &sig)
            .map_err(|_| SigningError::VerificationFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_and_verify() {
        let mgr = SigningManager::generate();
        let pub_key = mgr.public_key_hex();
        let data = b"hello agentverse";
        let sig = mgr.sign(data);
        assert!(SigningManager::verify(&pub_key, data, &sig).is_ok());
        assert!(SigningManager::verify(&pub_key, b"tampered", &sig).is_err());
    }

    #[test]
    fn hex_roundtrip() {
        let mgr = SigningManager::generate();
        let priv_hex = mgr.private_key_hex();
        let mgr2 = SigningManager::from_hex(&priv_hex).unwrap();
        assert_eq!(mgr.public_key_hex(), mgr2.public_key_hex());
    }
}
