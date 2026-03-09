//! FJ-1390/3308/3103/3303: Security scanner, secret audit, cron, encryption.
//!
//! Demonstrates:
//! - Static IaC security scanning (10 rule categories)
//! - Secret access audit trail with JSONL persistence
//! - Cron expression parsing and schedule matching
//! - State encryption with BLAKE3 HMAC integrity
//!
//! Usage: cargo run --example security_audit_cron

use forjar::core::cron_source::{matches, parse_cron, schedule_summary, CronTime};
use forjar::core::secret_audit::{
    audit_summary, filter_by_key, format_audit_summary, make_discard_event, make_inject_event,
    make_resolve_event, make_rotate_event,
};
use forjar::core::security_scanner::{scan, severity_counts};
use forjar::core::state_encryption::{
    create_metadata, derive_key, hash_data, keyed_hash, verify_keyed_hash, verify_metadata,
};
use forjar::core::types::*;
use indexmap::IndexMap;

fn main() {
    println!("Forjar: Security, Audit, Cron & Encryption");
    println!("{}", "=".repeat(50));

    // ── FJ-1390: Security Scanner ──
    println!("\n[FJ-1390] Static Security Scanner:");
    let mut resources = IndexMap::new();
    resources.insert(
        "db-config".into(),
        Resource {
            resource_type: ResourceType::File,
            content: Some("password=hunter2\nbind 0.0.0.0".into()),
            mode: Some("0644".into()),
            ..Default::default()
        },
    );
    resources.insert(
        "app-download".into(),
        Resource {
            resource_type: ResourceType::File,
            source: Some("http://releases.example.com/v1.tar.gz".into()),
            ..Default::default()
        },
    );
    resources.insert(
        "clean-svc".into(),
        Resource {
            resource_type: ResourceType::Service,
            ..Default::default()
        },
    );
    let config = ForjarConfig {
        name: "insecure-demo".into(),
        resources,
        ..Default::default()
    };
    let findings = scan(&config);
    let (c, h, m, l) = severity_counts(&findings);
    println!("  Findings: {} total", findings.len());
    println!("  Critical: {c}, High: {h}, Medium: {m}, Low: {l}");
    for f in &findings {
        println!(
            "  [{:?}] {}: {} ({})",
            f.severity, f.rule_id, f.message, f.resource_id
        );
    }
    assert!(c >= 1, "should detect hardcoded password");
    assert!(h >= 1, "should detect HTTP without TLS");

    // ── FJ-3308: Secret Audit Trail ──
    println!("\n[FJ-3308] Secret Access Audit:");
    let events = vec![
        make_resolve_event("db_pass", "env", "blake3:abc123", Some("web-01")),
        make_inject_event("db_pass", "env", "blake3:abc123", "ns-apply-1"),
        make_resolve_event("api_key", "file", "blake3:def456", None),
        make_discard_event("db_pass", "blake3:abc123"),
        make_rotate_event("api_key", "file", "blake3:def456", "blake3:ghi789"),
    ];
    let summary = audit_summary(&events);
    println!("{}", format_audit_summary(&summary));
    let db_events = filter_by_key(&events, "db_pass");
    println!("  db_pass accesses: {}", db_events.len());
    assert_eq!(summary.total, 5);
    assert_eq!(summary.unique_keys, 2);

    // ── FJ-3103: Cron Source ──
    println!("\n[FJ-3103] Cron Expression Parsing:");
    for (expr, desc) in [
        ("*/15 * * * *", "every 15 min"),
        ("0 9 * * 1-5", "9am weekdays"),
        ("0 0 1 * *", "midnight first of month"),
    ] {
        let schedule = parse_cron(expr).unwrap();
        println!("  {expr:20} ({desc}): {}", schedule_summary(&schedule));
    }

    let work_hours = parse_cron("0 9 * * 1-5").unwrap();
    let monday_9am = CronTime {
        minute: 0,
        hour: 9,
        day: 10,
        month: 3,
        weekday: 1,
    };
    let sunday_9am = CronTime {
        minute: 0,
        hour: 9,
        day: 9,
        month: 3,
        weekday: 0,
    };
    assert!(matches(&work_hours, &monday_9am));
    assert!(!matches(&work_hours, &sunday_9am));
    println!("  Monday 9am matches work schedule: true");
    println!("  Sunday 9am matches work schedule: false");

    // ── FJ-3303: State Encryption ──
    println!("\n[FJ-3303] State Encryption:");
    let key = derive_key("my-passphrase");
    let plaintext = b"schema: 1.0\nmachine: web-01\nresources: {}";
    let ciphertext = b"<encrypted-blob>";

    let h = hash_data(plaintext);
    println!("  Plaintext BLAKE3: {}", &h[..16]);

    let hmac = keyed_hash(ciphertext, &key);
    println!("  Ciphertext HMAC:  {}", &hmac[..16]);
    assert!(verify_keyed_hash(ciphertext, &key, &hmac));
    assert!(!verify_keyed_hash(b"tampered", &key, &hmac));

    let meta = create_metadata(plaintext, ciphertext, &key);
    assert!(verify_metadata(&meta, ciphertext, &key));
    assert!(!verify_metadata(&meta, b"tampered", &key));
    println!("  Metadata integrity verified");

    println!("\n{}", "=".repeat(50));
    println!("All security/audit/cron/encryption criteria survived.");
}
