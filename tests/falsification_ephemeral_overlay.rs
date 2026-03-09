//! FJ-3300/2103: Ephemeral value redaction and overlay layer conversion.
//!
//! Popperian rejection criteria for:
//! - FJ-3300: redact_to_hash (deterministic, different values differ, format)
//! - FJ-3300: is_ephemeral_marker (valid, invalid, boundary)
//! - FJ-3300: extract_hash (roundtrip, invalid, length)
//! - FJ-3300: verify_drift (match, mismatch, invalid marker)
//! - FJ-3300: redact_outputs (force_all, heuristic, empty)
//! - FJ-3300: keyed_hash (deterministic, different keys, different data)
//! - FJ-3300: derive_key (deterministic, different passphrases)
//! - FJ-3300: verify_keyed_hash (pass, tampered)
//! - FJ-2103: whiteouts_to_entries (file delete, opaque dir, empty)
//! - FJ-2103: merge_overlay_entries (combines entries + whiteouts)
//! - FJ-2103: format_overlay_scan (human output)
//! - FJ-2103: scan_overlay_upper (regular files, whiteouts, nested)
//!
//! Usage: cargo test --test falsification_ephemeral_overlay

use forjar::core::state::ephemeral::{
    derive_key, extract_hash, is_ephemeral_marker, keyed_hash, redact_outputs, redact_to_hash,
    verify_drift, verify_keyed_hash, EPHEMERAL_PREFIX, EPHEMERAL_SUFFIX,
};
use forjar::core::store::overlay_export::{
    format_overlay_scan, merge_overlay_entries, scan_overlay_upper, whiteouts_to_entries,
    OverlayScan,
};
use forjar::core::types::WhiteoutEntry;

// ============================================================================
// FJ-3300: redact_to_hash
// ============================================================================

#[test]
fn redact_deterministic() {
    let h1 = redact_to_hash("my-secret");
    let h2 = redact_to_hash("my-secret");
    assert_eq!(h1, h2);
}

#[test]
fn redact_format() {
    let h = redact_to_hash("test-value");
    assert!(h.starts_with(EPHEMERAL_PREFIX));
    assert!(h.ends_with(EPHEMERAL_SUFFIX));
}

#[test]
fn redact_different_values_differ() {
    assert_ne!(redact_to_hash("secret-A"), redact_to_hash("secret-B"));
}

#[test]
fn redact_empty_string() {
    let h = redact_to_hash("");
    assert!(is_ephemeral_marker(&h));
}

// ============================================================================
// FJ-3300: is_ephemeral_marker
// ============================================================================

#[test]
fn marker_valid() {
    let marker = redact_to_hash("value");
    assert!(is_ephemeral_marker(&marker));
}

#[test]
fn marker_invalid_plaintext() {
    assert!(!is_ephemeral_marker("plaintext-value"));
}

#[test]
fn marker_invalid_partial_prefix() {
    assert!(!is_ephemeral_marker("EPHEMERAL[blake3:"));
    assert!(!is_ephemeral_marker("EPHEMERAL["));
}

#[test]
fn marker_invalid_no_suffix() {
    let no_suffix = format!("{EPHEMERAL_PREFIX}abcdef1234567890");
    assert!(!is_ephemeral_marker(&no_suffix));
}

// ============================================================================
// FJ-3300: extract_hash
// ============================================================================

#[test]
fn extract_hash_roundtrip() {
    let original = "password-123";
    let marker = redact_to_hash(original);
    let hash = extract_hash(&marker).unwrap();
    assert_eq!(hash.len(), 64); // BLAKE3 hex = 64 chars
    let expected = blake3::hash(original.as_bytes()).to_hex().to_string();
    assert_eq!(hash, expected);
}

#[test]
fn extract_hash_invalid() {
    assert!(extract_hash("not-a-marker").is_none());
    assert!(extract_hash("").is_none());
}

// ============================================================================
// FJ-3300: verify_drift
// ============================================================================

#[test]
fn drift_no_change() {
    let secret = "db-password-2026";
    let marker = redact_to_hash(secret);
    assert!(verify_drift(secret, &marker));
}

#[test]
fn drift_value_changed() {
    let marker = redact_to_hash("old-password");
    assert!(!verify_drift("new-password", &marker));
}

#[test]
fn drift_invalid_marker() {
    assert!(!verify_drift("anything", "not-a-marker"));
}

#[test]
fn drift_empty_values() {
    let marker = redact_to_hash("");
    assert!(verify_drift("", &marker));
    assert!(!verify_drift("non-empty", &marker));
}

// ============================================================================
// FJ-3300: redact_outputs
// ============================================================================

#[test]
fn redact_outputs_force_all() {
    let mut outputs = indexmap::IndexMap::new();
    outputs.insert("data_dir".into(), "/var/data".into());
    outputs.insert("app_port".into(), "8080".into());
    let redacted = redact_outputs(&outputs, true);
    assert!(is_ephemeral_marker(redacted.get("data_dir").unwrap()));
    assert!(is_ephemeral_marker(redacted.get("app_port").unwrap()));
}

#[test]
fn redact_outputs_heuristic_secrets() {
    let mut outputs = indexmap::IndexMap::new();
    outputs.insert("db_password".into(), "s3cret".into());
    outputs.insert("api_token".into(), "tok-123".into());
    outputs.insert("ssh_key".into(), "key-data".into());
    outputs.insert("aws_credential".into(), "cred-data".into());
    outputs.insert("app_secret".into(), "hidden".into());
    let redacted = redact_outputs(&outputs, false);
    for key in [
        "db_password",
        "api_token",
        "ssh_key",
        "aws_credential",
        "app_secret",
    ] {
        assert!(
            is_ephemeral_marker(redacted.get(key).unwrap()),
            "expected {key} to be redacted"
        );
    }
}

#[test]
fn redact_outputs_heuristic_non_secrets_preserved() {
    let mut outputs = indexmap::IndexMap::new();
    outputs.insert("data_dir".into(), "/var/data".into());
    outputs.insert("app_port".into(), "8080".into());
    outputs.insert("hostname".into(), "web-01".into());
    let redacted = redact_outputs(&outputs, false);
    assert_eq!(redacted.get("data_dir").unwrap(), "/var/data");
    assert_eq!(redacted.get("app_port").unwrap(), "8080");
    assert_eq!(redacted.get("hostname").unwrap(), "web-01");
}

#[test]
fn redact_outputs_empty() {
    let outputs = indexmap::IndexMap::new();
    let redacted = redact_outputs(&outputs, true);
    assert!(redacted.is_empty());
}

// ============================================================================
// FJ-3300: keyed_hash / derive_key / verify_keyed_hash
// ============================================================================

#[test]
fn keyed_hash_deterministic() {
    let key = derive_key("my-passphrase");
    let h1 = keyed_hash(b"state data", &key);
    let h2 = keyed_hash(b"state data", &key);
    assert_eq!(h1, h2);
    assert_eq!(h1.len(), 64);
}

#[test]
fn keyed_hash_different_keys_differ() {
    let k1 = derive_key("key-A");
    let k2 = derive_key("key-B");
    assert_ne!(keyed_hash(b"same", &k1), keyed_hash(b"same", &k2));
}

#[test]
fn keyed_hash_different_data_differ() {
    let key = derive_key("k");
    assert_ne!(keyed_hash(b"data-A", &key), keyed_hash(b"data-B", &key));
}

#[test]
fn derive_key_deterministic() {
    assert_eq!(derive_key("phrase"), derive_key("phrase"));
}

#[test]
fn derive_key_different_passphrases() {
    assert_ne!(derive_key("a"), derive_key("b"));
}

#[test]
fn verify_keyed_hash_pass() {
    let key = derive_key("test-key");
    let hash = keyed_hash(b"important data", &key);
    assert!(verify_keyed_hash(b"important data", &key, &hash));
}

#[test]
fn verify_keyed_hash_tampered_data() {
    let key = derive_key("test-key");
    let hash = keyed_hash(b"original", &key);
    assert!(!verify_keyed_hash(b"tampered", &key, &hash));
}

#[test]
fn verify_keyed_hash_wrong_key() {
    let k1 = derive_key("key-1");
    let k2 = derive_key("key-2");
    let hash = keyed_hash(b"data", &k1);
    assert!(!verify_keyed_hash(b"data", &k2, &hash));
}

// ============================================================================
// FJ-2103: whiteouts_to_entries
// ============================================================================

#[test]
fn whiteouts_file_delete() {
    let whiteouts = vec![WhiteoutEntry::FileDelete {
        path: "etc/old.conf".into(),
    }];
    let entries = whiteouts_to_entries(&whiteouts);
    assert_eq!(entries.len(), 1);
    assert!(entries[0].path.contains(".wh."));
}

#[test]
fn whiteouts_opaque_dir() {
    let whiteouts = vec![WhiteoutEntry::OpaqueDir {
        path: "var/cache".into(),
    }];
    let entries = whiteouts_to_entries(&whiteouts);
    assert_eq!(entries.len(), 1);
    assert!(entries[0].path.contains(".wh."));
}

#[test]
fn whiteouts_empty() {
    let entries = whiteouts_to_entries(&[]);
    assert!(entries.is_empty());
}

#[test]
fn whiteouts_mixed() {
    let whiteouts = vec![
        WhiteoutEntry::FileDelete {
            path: "a/b.txt".into(),
        },
        WhiteoutEntry::OpaqueDir { path: "c/d".into() },
        WhiteoutEntry::FileDelete {
            path: "e.log".into(),
        },
    ];
    let entries = whiteouts_to_entries(&whiteouts);
    assert_eq!(entries.len(), 3);
}

// ============================================================================
// FJ-2103: merge_overlay_entries
// ============================================================================

#[test]
fn merge_combines_entries_and_whiteouts() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("app")).unwrap();
    std::fs::write(dir.path().join("app/new.txt"), "hello").unwrap();
    std::fs::write(dir.path().join("app/.wh.old.txt"), "").unwrap();

    let scan = scan_overlay_upper(dir.path(), dir.path()).unwrap();
    assert_eq!(scan.entries.len(), 1);
    assert_eq!(scan.whiteouts.len(), 1);
    let merged = merge_overlay_entries(&scan);
    assert_eq!(merged.len(), 2);
}

// ============================================================================
// FJ-2103: format_overlay_scan
// ============================================================================

#[test]
fn format_scan_output() {
    let scan = OverlayScan {
        entries: vec![],
        whiteouts: vec![WhiteoutEntry::FileDelete { path: "x".into() }],
        total_bytes: 2048,
        file_count: 5,
    };
    let s = format_overlay_scan(&scan);
    assert!(s.contains("5 files"));
    assert!(s.contains("2.0 KB"));
    assert!(s.contains("1 whiteout"));
}

#[test]
fn format_scan_zero() {
    let scan = OverlayScan {
        entries: vec![],
        whiteouts: vec![],
        total_bytes: 0,
        file_count: 0,
    };
    let s = format_overlay_scan(&scan);
    assert!(s.contains("0 files"));
    assert!(s.contains("0 whiteout"));
}

// ============================================================================
// FJ-2103: scan_overlay_upper
// ============================================================================

#[test]
fn scan_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let scan = scan_overlay_upper(dir.path(), dir.path()).unwrap();
    assert_eq!(scan.file_count, 0);
    assert!(scan.entries.is_empty());
}

#[test]
fn scan_regular_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("etc")).unwrap();
    std::fs::write(dir.path().join("etc/app.conf"), "key=value\n").unwrap();
    std::fs::write(dir.path().join("etc/other.conf"), "x=1\n").unwrap();
    let scan = scan_overlay_upper(dir.path(), dir.path()).unwrap();
    assert_eq!(scan.file_count, 2);
    assert!(scan.total_bytes > 0);
}

#[test]
fn scan_nested_deep() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("a/b/c")).unwrap();
    std::fs::write(dir.path().join("a/b/c/deep.txt"), "deep").unwrap();
    let scan = scan_overlay_upper(dir.path(), dir.path()).unwrap();
    assert_eq!(scan.file_count, 1);
}

#[test]
fn scan_missing_dir_errors() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nonexistent");
    assert!(scan_overlay_upper(&missing, &missing).is_err());
}

#[test]
fn scan_file_whiteout() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("etc")).unwrap();
    std::fs::write(dir.path().join("etc/.wh.removed.conf"), "").unwrap();
    let scan = scan_overlay_upper(dir.path(), dir.path()).unwrap();
    assert_eq!(scan.whiteouts.len(), 1);
    assert!(
        matches!(&scan.whiteouts[0], WhiteoutEntry::FileDelete { path } if path == "etc/removed.conf")
    );
}

#[test]
fn scan_opaque_whiteout() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("var/cache")).unwrap();
    std::fs::write(dir.path().join("var/cache/.wh..wh..opq"), "").unwrap();
    let scan = scan_overlay_upper(dir.path(), dir.path()).unwrap();
    assert_eq!(scan.whiteouts.len(), 1);
    assert!(matches!(&scan.whiteouts[0], WhiteoutEntry::OpaqueDir { path } if path == "var/cache"));
}
