//! Tests for FJ-1300: store path derivation.

use super::path::{store_entry_path, store_path, STORE_BASE};

#[test]
fn test_fj1300_store_path_deterministic() {
    let h1 = store_path("recipe:abc", &["in:1", "in:2"], "x86_64", "apt");
    let h2 = store_path("recipe:abc", &["in:1", "in:2"], "x86_64", "apt");
    assert_eq!(h1, h2, "identical inputs must produce identical hash");
}

#[test]
fn test_fj1300_store_path_different_recipe() {
    let h1 = store_path("recipe:abc", &["in:1"], "x86_64", "apt");
    let h2 = store_path("recipe:xyz", &["in:1"], "x86_64", "apt");
    assert_ne!(h1, h2, "different recipe hashes must differ");
}

#[test]
fn test_fj1300_store_path_different_inputs() {
    let h1 = store_path("recipe:abc", &["in:1"], "x86_64", "apt");
    let h2 = store_path("recipe:abc", &["in:2"], "x86_64", "apt");
    assert_ne!(h1, h2, "different input hashes must differ");
}

#[test]
fn test_fj1300_store_path_different_arch() {
    let h1 = store_path("recipe:abc", &["in:1"], "x86_64", "apt");
    let h2 = store_path("recipe:abc", &["in:1"], "aarch64", "apt");
    assert_ne!(h1, h2, "different arches must differ");
}

#[test]
fn test_fj1300_store_path_different_provider() {
    let h1 = store_path("recipe:abc", &["in:1"], "x86_64", "apt");
    let h2 = store_path("recipe:abc", &["in:1"], "x86_64", "cargo");
    assert_ne!(h1, h2, "different providers must differ");
}

#[test]
fn test_fj1300_store_path_order_independence() {
    let h1 = store_path("recipe:abc", &["in:1", "in:2", "in:3"], "x86_64", "apt");
    let h2 = store_path("recipe:abc", &["in:3", "in:1", "in:2"], "x86_64", "apt");
    assert_eq!(h1, h2, "input hash order must not matter");
}

#[test]
fn test_fj1300_store_path_blake3_format() {
    let h = store_path("recipe:abc", &["in:1"], "x86_64", "apt");
    assert!(h.starts_with("blake3:"), "hash must have blake3: prefix");
    let hex = h.strip_prefix("blake3:").unwrap();
    assert_eq!(hex.len(), 64, "blake3 hex must be 64 chars");
    assert!(
        hex.chars().all(|c| c.is_ascii_hexdigit()),
        "hash must be valid hex"
    );
}

#[test]
fn test_fj1300_store_path_empty_inputs() {
    let h = store_path("recipe:abc", &[], "x86_64", "apt");
    assert!(h.starts_with("blake3:"));
}

#[test]
fn test_fj1300_store_entry_path_basic() {
    let hash = "blake3:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
    let path = store_entry_path(hash);
    assert_eq!(
        path,
        format!("{STORE_BASE}/abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890")
    );
}

#[test]
fn test_fj1300_store_entry_path_strips_prefix() {
    let hash = "blake3:aabb";
    let path = store_entry_path(hash);
    assert!(path.ends_with("/aabb"));
    assert!(!path.contains("blake3:"));
}

#[test]
fn test_fj1300_store_entry_path_no_prefix() {
    let path = store_entry_path("rawdeadbeef");
    assert!(path.ends_with("/rawdeadbeef"));
}

#[test]
fn test_fj1300_store_base_constant() {
    assert_eq!(STORE_BASE, "/var/lib/forjar/store");
}
