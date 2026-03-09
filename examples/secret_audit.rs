//! Example: Secret access audit trail (FJ-3308)
//!
//! Demonstrates logging secret access events to a JSONL audit trail
//! for compliance and forensic analysis.
//!
//! ```bash
//! cargo run --example secret_audit
//! ```

use forjar::core::secret_audit::{
    append_audit, audit_summary, filter_by_key, filter_by_type, format_audit_summary,
    make_discard_event, make_inject_event, make_resolve_event, make_rotate_event, read_audit,
    SecretEventType,
};
use tempfile::TempDir;

fn main() {
    println!("=== Secret Access Audit Trail (FJ-3308) ===\n");

    let dir = TempDir::new().unwrap();
    let state_dir = dir.path();

    // 1. Simulate secret lifecycle: resolve → inject → discard
    println!("1. Secret lifecycle events:");

    let hash1 = blake3::hash(b"super-secret-db-password")
        .to_hex()
        .to_string();
    let hash2 = blake3::hash(b"api-key-prod-1234").to_hex().to_string();

    // Resolve secrets from providers
    let e1 = make_resolve_event("db_password", "env", &hash1, Some("web-01"));
    append_audit(state_dir, &e1).unwrap();
    println!("   [resolve] db_password from env provider (machine: web-01)");

    let e2 = make_resolve_event("api_key", "file", &hash2, Some("web-01"));
    append_audit(state_dir, &e2).unwrap();
    println!("   [resolve] api_key from file provider (machine: web-01)");

    // Inject into namespace
    let e3 = make_inject_event("db_password", "env", &hash1, "ns-forjar-apply-1");
    append_audit(state_dir, &e3).unwrap();
    println!("   [inject]  db_password into ns-forjar-apply-1");

    let e4 = make_inject_event("api_key", "file", &hash2, "ns-forjar-apply-1");
    append_audit(state_dir, &e4).unwrap();
    println!("   [inject]  api_key into ns-forjar-apply-1");

    // Discard after use
    let e5 = make_discard_event("db_password", &hash1);
    append_audit(state_dir, &e5).unwrap();
    println!("   [discard] db_password (namespace torn down)");

    let e6 = make_discard_event("api_key", &hash2);
    append_audit(state_dir, &e6).unwrap();
    println!("   [discard] api_key (namespace torn down)");

    // Rotate a secret
    let new_hash = blake3::hash(b"rotated-db-password-v2").to_hex().to_string();
    let e7 = make_rotate_event("db_password", "env", &hash1, &new_hash);
    append_audit(state_dir, &e7).unwrap();
    println!("   [rotate]  db_password (key rotated)");

    // 2. Read and analyze
    println!("\n2. Reading audit log:");
    let events = read_audit(state_dir).unwrap();
    println!("   Total events: {}", events.len());

    // 3. Filter by key
    println!("\n3. Filter by key 'db_password':");
    let db_events = filter_by_key(&events, "db_password");
    for e in &db_events {
        println!(
            "   [{:>8}] pid={} hash={}...",
            e.event_type,
            e.pid,
            &e.value_hash[..16]
        );
    }

    // 4. Filter by type
    println!("\n4. All inject events:");
    let injects = filter_by_type(&events, &SecretEventType::Inject);
    for e in &injects {
        println!(
            "   {} → {} (ns: {})",
            e.key,
            e.provider,
            e.namespace.as_deref().unwrap_or("-")
        );
    }

    // 5. Summary
    println!("\n5. Audit Summary:");
    let summary = audit_summary(&events);
    println!("{}", format_audit_summary(&summary));

    println!("\nDone.");
}
