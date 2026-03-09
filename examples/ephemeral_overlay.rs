//! FJ-3300/2103: Ephemeral secret redaction and overlay layer conversion.
//!
//! Demonstrates:
//! - BLAKE3 hash redaction of secret values
//! - Ephemeral marker detection and drift verification
//! - Heuristic and forced output redaction
//! - Keyed hashing for state integrity
//! - Overlay directory scanning and whiteout detection
//!
//! Usage: cargo run --example ephemeral_overlay

use forjar::core::state::ephemeral::{
    derive_key, extract_hash, is_ephemeral_marker, keyed_hash, redact_outputs, redact_to_hash,
    verify_drift, verify_keyed_hash,
};
use forjar::core::store::overlay_export::{
    format_overlay_scan, merge_overlay_entries, scan_overlay_upper, whiteouts_to_entries,
};
use forjar::core::types::WhiteoutEntry;

fn main() {
    println!("Forjar: Ephemeral Secrets & Overlay Layers");
    println!("{}", "=".repeat(50));

    // ── Ephemeral Redaction ──
    println!("\n[FJ-3300] Ephemeral Redaction:");
    let secret = "db-password-2026";
    let marker = redact_to_hash(secret);
    println!("  Secret: {secret}");
    println!("  Marker: {marker}");
    assert!(is_ephemeral_marker(&marker));

    let hash = extract_hash(&marker).unwrap();
    println!("  Hash: {hash} ({} chars)", hash.len());

    // ── Drift Detection ──
    println!("\n[FJ-3300] Drift Detection:");
    assert!(verify_drift(secret, &marker));
    println!("  Same secret: no drift");
    assert!(!verify_drift("new-password", &marker));
    println!("  Changed secret: drift detected");

    // ── Output Redaction ──
    println!("\n[FJ-3300] Output Redaction:");
    let mut outputs = indexmap::IndexMap::new();
    outputs.insert("db_password".into(), "s3cret".into());
    outputs.insert("api_token".into(), "tok-123".into());
    outputs.insert("data_dir".into(), "/var/data".into());
    let redacted = redact_outputs(&outputs, false);
    for (k, v) in &redacted {
        let status = if is_ephemeral_marker(v) {
            "REDACTED"
        } else {
            "plain"
        };
        println!("  {k}: [{status}]");
    }

    // ── Keyed Hashing ──
    println!("\n[FJ-3300] Keyed Hashing:");
    let key = derive_key("my-passphrase");
    let hash = keyed_hash(b"encrypted-state-data", &key);
    println!("  HMAC: {hash}");
    assert!(verify_keyed_hash(b"encrypted-state-data", &key, &hash));
    assert!(!verify_keyed_hash(b"tampered", &key, &hash));
    println!("  Integrity verified, tamper detected");

    // ── Overlay Scan ──
    println!("\n[FJ-2103] Overlay Scan:");
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("etc")).unwrap();
    std::fs::write(dir.path().join("etc/app.conf"), "key=value\n").unwrap();
    std::fs::write(dir.path().join("etc/.wh.old.conf"), "").unwrap();
    let scan = scan_overlay_upper(dir.path(), dir.path()).unwrap();
    println!("  {}", format_overlay_scan(&scan));
    assert_eq!(scan.file_count, 1);
    assert_eq!(scan.whiteouts.len(), 1);

    // ── Whiteout Conversion ──
    println!("\n[FJ-2103] Whiteout → Layer Entries:");
    let whiteouts = vec![
        WhiteoutEntry::FileDelete {
            path: "etc/old.conf".into(),
        },
        WhiteoutEntry::OpaqueDir {
            path: "var/cache".into(),
        },
    ];
    let entries = whiteouts_to_entries(&whiteouts);
    for e in &entries {
        println!("  {}", e.path);
    }

    let merged = merge_overlay_entries(&scan);
    println!("  Merged: {} entries (files + whiteouts)", merged.len());

    println!("\n{}", "=".repeat(50));
    println!("All ephemeral/overlay criteria survived.");
}
