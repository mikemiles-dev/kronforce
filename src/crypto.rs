//! Application-level field encryption using AES-256-GCM.
//!
//! Encrypts sensitive fields (secret variable values, OIDC tokens) before storing
//! in SQLite. The encryption key is derived from `KRONFORCE_ENCRYPTION_KEY` env var.
//! If not set, encryption is disabled and values are stored in plaintext.

use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, KeyInit},
};
use base64::Engine;

/// Encryption context. None if encryption is disabled.
static ENCRYPTION_KEY: std::sync::OnceLock<Option<[u8; 32]>> = std::sync::OnceLock::new();

/// Initializes the encryption key from the environment. Call once at startup.
pub fn init() {
    ENCRYPTION_KEY.get_or_init(|| {
        std::env::var("KRONFORCE_ENCRYPTION_KEY")
            .ok()
            .filter(|s| !s.is_empty())
            .map(|s| {
                // SHA-256 hash the key to get exactly 32 bytes
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(s.as_bytes());
                let result = hasher.finalize();
                let mut key = [0u8; 32];
                key.copy_from_slice(&result);
                key
            })
    });
}

fn get_key() -> Option<&'static [u8; 32]> {
    ENCRYPTION_KEY.get().and_then(|k| k.as_ref())
}

/// Returns true if encryption is enabled.
pub fn is_enabled() -> bool {
    get_key().is_some()
}

/// Encrypts a plaintext string. Returns base64-encoded ciphertext with prepended nonce.
/// If encryption is disabled, returns the plaintext unchanged.
pub fn encrypt(plaintext: &str) -> String {
    let Some(key_bytes) = get_key() else {
        return plaintext.to_string();
    };

    let key = Key::<Aes256Gcm>::from_slice(key_bytes);
    let cipher = Aes256Gcm::new(key);

    // Generate random 12-byte nonce
    let mut nonce_bytes = [0u8; 12];
    use rand::Rng;
    rand::rng().fill(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .expect("encryption failed");

    // Prepend nonce to ciphertext and base64 encode
    let mut combined = Vec::with_capacity(12 + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    format!(
        "enc:{}",
        base64::engine::general_purpose::STANDARD.encode(&combined)
    )
}

/// Decrypts a string. If it starts with "enc:", decrypts it.
/// Otherwise returns it unchanged (plaintext or encryption disabled).
pub fn decrypt(stored: &str) -> Result<String, String> {
    let Some(encoded) = stored.strip_prefix("enc:") else {
        return Ok(stored.to_string()); // Not encrypted
    };

    let Some(key_bytes) = get_key() else {
        return Err("encrypted value but no KRONFORCE_ENCRYPTION_KEY set".to_string());
    };

    let combined = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|e| format!("base64 decode failed: {e}"))?;

    if combined.len() < 13 {
        return Err("ciphertext too short".to_string());
    }

    let key = Key::<Aes256Gcm>::from_slice(key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&combined[..12]);
    let ciphertext = &combined[12..];

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "decryption failed (wrong key?)".to_string())?;

    String::from_utf8(plaintext).map_err(|e| format!("invalid UTF-8: {e}"))
}
