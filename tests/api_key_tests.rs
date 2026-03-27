use kronforce::api::{generate_api_key, hash_api_key};

#[test]
fn test_generate_api_key_format() {
    let (raw, prefix) = generate_api_key();
    assert!(raw.starts_with("kf_"));
    assert!(prefix.starts_with("kf_"));
    assert_eq!(prefix.len(), 11);
    assert!(raw.len() > prefix.len());
}

#[test]
fn test_generate_api_key_uniqueness() {
    let (key1, _) = generate_api_key();
    let (key2, _) = generate_api_key();
    assert_ne!(key1, key2);
}

#[test]
fn test_hash_api_key_deterministic() {
    let hash1 = hash_api_key("test_key_123");
    let hash2 = hash_api_key("test_key_123");
    assert_eq!(hash1, hash2);
}

#[test]
fn test_hash_api_key_different_inputs() {
    let hash1 = hash_api_key("key_a");
    let hash2 = hash_api_key("key_b");
    assert_ne!(hash1, hash2);
}

#[test]
fn test_hash_is_hex_string() {
    let hash = hash_api_key("some_key");
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(hash.len(), 64); // SHA256 = 64 hex chars
}
