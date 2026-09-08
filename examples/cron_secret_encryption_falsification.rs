//! FJ-3103/3302/3303/3307: Cron, ephemeral, encryption, secret lint falsification.
//!
//! Demonstrates Popperian rejection criteria for:
//! - Cron expression parsing and schedule matching
//! - Script secret leakage detection (13 patterns)
//! - State encryption metadata (BLAKE3 hash + HMAC)
//! - Ephemeral value pipeline (resolve → hash → discard)
//!
//! Usage: cargo run --example cron_secret_encryption_falsification

use forjar::core::cron_source::{matches, parse_cron, schedule_summary, CronTime};
use forjar::core::ephemeral::{
    check_drift, substitute_ephemerals, to_records, DriftStatus, ResolvedEphemeral,
};
use forjar::core::script_secret_lint::{scan_script, validate_no_leaks};
use forjar::core::state_encryption::{
    create_metadata, derive_key, hash_data, keyed_hash, verify_keyed_hash, verify_metadata,
};

/// One falsification criterion: print the verdict, then refuse to continue if
/// it did not survive. The five-line `if ok { "yes" } else { "no" }` block this
/// replaces appeared once per criterion and was the whole of these functions'
/// branching (CB-200 counted `main` at complexity 38 because of it, PMAT-206).
fn criterion(label: &str, ok: bool) {
    println!(
        "  {label}: {} {}",
        if ok { "yes" } else { "no" },
        if ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(ok, "{label}");
}

fn main() {
    println!("Forjar Cron / Secret Lint / Encryption / Ephemeral Falsification");
    println!("{}", "=".repeat(60));

    cron_parsing();
    script_secret_lint();
    state_encryption();
    ephemeral_values();
}

/// FJ-3103: Cron Parsing
fn cron_parsing() {
    // ── FJ-3103: Cron Parsing ──
    println!("\n[FJ-3103] Cron Expression Parsing:");

    let every_15 = parse_cron("*/15 * * * *").unwrap();
    let ok_15 = every_15.minutes.len() == 4
        && every_15.minutes.contains(&0)
        && every_15.minutes.contains(&45);
    criterion("*/15 → 4 values (0,15,30,45)", ok_15);

    let business = parse_cron("0 9 * * 1-5").unwrap();
    let t_mon = CronTime {
        minute: 0,
        hour: 9,
        day: 1,
        month: 3,
        weekday: 1,
    };
    let t_sun = CronTime {
        minute: 0,
        hour: 9,
        day: 7,
        month: 3,
        weekday: 0,
    };
    let biz_ok = matches(&business, &t_mon) && !matches(&business, &t_sun);
    criterion("Mon 9:00 matches, Sun 9:00 doesn't", biz_ok);

    let summary = schedule_summary(&every_15);
    println!("  Summary: {summary}");

    let err = parse_cron("* *");
    let err_ok = err.is_err();
    criterion("Invalid '* *' rejected", err_ok);
}

/// FJ-3307: Script Secret Lint
fn script_secret_lint() {
    // ── FJ-3307: Script Secret Lint ──
    println!("\n[FJ-3307] Script Secret Leakage Detection:");

    let clean = scan_script("#!/bin/bash\napt-get install nginx\n");
    let clean_ok = clean.clean();
    criterion("Clean script passes", clean_ok);

    let leak = scan_script("echo $PASSWORD > /tmp/log\ncurl -u admin:pass https://api.com\n");
    let leak_ok = !leak.clean() && leak.findings.len() >= 2;
    criterion(
        &format!(
            "echo $PASSWORD + curl -u detected ({} findings)",
            leak.findings.len()
        ),
        leak_ok,
    );

    let comments = scan_script("# echo $PASSWORD\n# sshpass -p secret ssh x\n");
    let comment_ok = comments.clean();
    criterion("Comments skipped", comment_ok);

    let v = validate_no_leaks("echo $TOKEN\n");
    let v_ok = v.is_err();
    criterion("validate_no_leaks rejects leaked token", v_ok);
}

/// FJ-3303: State Encryption
fn state_encryption() {
    // ── FJ-3303: State Encryption (BLAKE3) ──
    println!("\n[FJ-3303] State Encryption (BLAKE3):");
    let key = derive_key("my-passphrase");
    criterion(
        "BLAKE3 hash deterministic (64 hex)",
        hash_is_deterministic(),
    );
    criterion(
        "HMAC verify + tamper detection",
        hmac_detects_tampering(&key),
    );
    criterion("Metadata v1 + verified", metadata_verifies(&key));
}

fn hash_is_deterministic() -> bool {
    let h1 = hash_data(b"state data");
    h1 == hash_data(b"state data") && h1.len() == 64
}

fn hmac_detects_tampering(key: &[u8; 32]) -> bool {
    let hmac = keyed_hash(b"ciphertext", key);
    verify_keyed_hash(b"ciphertext", key, &hmac) && !verify_keyed_hash(b"tampered", key, &hmac)
}

fn metadata_verifies(key: &[u8; 32]) -> bool {
    let meta = create_metadata(b"plaintext state", b"encrypted bytes", key);
    meta.version == 1
        && verify_metadata(&meta, b"encrypted bytes", key)
        && !meta.encrypted_at.is_empty()
}

/// FJ-3302: Ephemeral Values
fn ephemeral_values() {
    // ── FJ-3302: Ephemeral Values ──
    println!("\n[FJ-3302] Ephemeral Value Pipeline:");

    let resolved = vec![
        ResolvedEphemeral {
            key: "db_pass".into(),
            value: "s3cret".into(),
            hash: blake3::hash(b"s3cret").to_hex().to_string(),
        },
        ResolvedEphemeral {
            key: "api_key".into(),
            value: "abc123".into(),
            hash: blake3::hash(b"abc123").to_hex().to_string(),
        },
    ];

    let config = substitute_ephemerals(
        "postgres://u:{{ephemeral.db_pass}}@h/db?k={{ephemeral.api_key}}",
        &resolved,
    );
    let sub_ok = config == "postgres://u:s3cret@h/db?k=abc123";
    criterion("Template substitution", sub_ok);

    let records = to_records(&resolved);
    let rec_ok = records.len() == 2 && records[0].hash.len() == 64;
    criterion("to_records strips plaintext", rec_ok);

    let drift = check_drift(&resolved, &records);
    let drift_ok = drift.iter().all(|d| d.status == DriftStatus::Unchanged);
    criterion("Drift unchanged after same resolve", drift_ok);

    let changed = vec![ResolvedEphemeral {
        key: "db_pass".into(),
        value: "rotated".into(),
        hash: blake3::hash(b"rotated").to_hex().to_string(),
    }];
    let change_drift = check_drift(&changed, &records);
    let change_ok = change_drift[0].status == DriftStatus::Changed;
    criterion("Drift detected after rotation", change_ok);

    println!("\n{}", "=".repeat(60));
    println!("All cron/secret/encryption/ephemeral criteria survived.");
}
