use agentverse_auth::{JwtManager, PasswordManager, SigningManager};
use uuid::Uuid;

// ── JWT tests ────────────────────────────────────────────────────────────────

#[test]
fn jwt_round_trip_human() {
    let mgr = JwtManager::new("test-secret-32-chars-minimum!!", 3600);
    let id = Uuid::new_v4();
    let token = mgr.generate(id, "alice", "human").unwrap();
    let claims = mgr.validate(&token).unwrap();
    assert_eq!(claims.sub, id);
    assert_eq!(claims.username, "alice");
    assert_eq!(claims.kind, "human");
}

#[test]
fn jwt_round_trip_agent() {
    let mgr = JwtManager::new("agent-secret-key-long-enough!!", 3600);
    let id = Uuid::new_v4();
    let token = mgr.generate(id, "bot-1", "agent").unwrap();
    let claims = mgr.validate(&token).unwrap();
    assert_eq!(claims.kind, "agent");
}

#[test]
fn jwt_expired_is_rejected() {
    let mgr = JwtManager::new("test-secret-32-chars-minimum!!", -10);
    let token = mgr.generate(Uuid::new_v4(), "u", "human").unwrap();
    assert!(mgr.validate(&token).is_err());
}

#[test]
fn jwt_tampered_signature_rejected() {
    let mgr = JwtManager::new("secret-a-is-different-secret!!!", 3600);
    let mgr2 = JwtManager::new("secret-b-is-different-secret!!!", 3600);
    let token = mgr.generate(Uuid::new_v4(), "u", "human").unwrap();
    assert!(mgr2.validate(&token).is_err());
}

// ── Password tests ───────────────────────────────────────────────────────────

#[test]
fn password_hash_and_verify_ok() {
    let pw = "correct-horse-battery-staple";
    let hash = PasswordManager::hash(pw).unwrap();
    assert!(PasswordManager::verify(pw, &hash).is_ok());
}

#[test]
fn wrong_password_rejected() {
    let hash = PasswordManager::hash("correct").unwrap();
    assert!(PasswordManager::verify("wrong", &hash).is_err());
}

// ── Ed25519 signing tests ────────────────────────────────────────────────────

#[test]
fn sign_and_verify_ok() {
    let mgr = SigningManager::generate();
    let pub_key = mgr.public_key_hex();
    let data = b"artifact-content-to-sign";
    let sig = mgr.sign(data);
    assert!(SigningManager::verify(&pub_key, data, &sig).is_ok());
}

#[test]
fn tampered_data_fails_verification() {
    let mgr = SigningManager::generate();
    let pub_key = mgr.public_key_hex();
    let sig = mgr.sign(b"original");
    assert!(SigningManager::verify(&pub_key, b"tampered", &sig).is_err());
}

#[test]
fn wrong_public_key_fails_verification() {
    let mgr1 = SigningManager::generate();
    let mgr2 = SigningManager::generate();
    let data = b"data";
    let sig = mgr1.sign(data);
    assert!(SigningManager::verify(&mgr2.public_key_hex(), data, &sig).is_err());
}

#[test]
fn private_key_hex_roundtrip() {
    let mgr = SigningManager::generate();
    let priv_hex = mgr.private_key_hex();
    let mgr2 = SigningManager::from_hex(&priv_hex).unwrap();
    assert_eq!(mgr.public_key_hex(), mgr2.public_key_hex());
    // Both should produce verifiable signatures
    let sig = mgr2.sign(b"hello");
    assert!(SigningManager::verify(&mgr.public_key_hex(), b"hello", &sig).is_ok());
}
