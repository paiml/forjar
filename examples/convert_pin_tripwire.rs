//! FJ-1328/1314/1364: Recipe conversion, pin tripwire, and pin resolution.
//!
//! Demonstrates:
//! - 5-step conversion ladder: unpinned → version pin → store → lock → sandbox
//! - Pin tripwire: staleness detection with severity levels
//! - Pin resolution: provider-specific CLI commands and output parsing
//! - Pin hashing: deterministic BLAKE3 composite hashes
//!
//! Usage: cargo run --example convert_pin_tripwire

use forjar::core::store::convert::{analyze_conversion, ConversionSignals};
use forjar::core::store::lockfile::{LockFile, Pin};
use forjar::core::store::pin_resolve::{parse_resolved_version, pin_hash, resolution_command};
use forjar::core::store::pin_tripwire::{
    check_before_apply, format_pin_report, needs_pin_update, pin_severity,
};
use std::collections::BTreeMap;

fn main() {
    println!("Forjar: Recipe Conversion, Pin Tripwire & Resolution");
    println!("{}", "=".repeat(55));

    // ── FJ-1328: Conversion Ladder ──
    println!("\n[FJ-1328] Conversion Ladder:");
    let signals = vec![
        ConversionSignals {
            name: "curl".into(),
            has_version: false,
            has_store: false,
            has_sandbox: false,
            has_curl_pipe: false,
            provider: "apt".into(),
            current_version: None,
        },
        ConversionSignals {
            name: "serde".into(),
            has_version: true,
            has_store: true,
            has_sandbox: true,
            has_curl_pipe: false,
            provider: "cargo".into(),
            current_version: Some("1.0.215".into()),
        },
    ];
    let report = analyze_conversion(&signals);
    for r in &report.resources {
        println!(
            "  {} : {:?} → {:?} ({} auto, {} manual)",
            r.name,
            r.current_purity,
            r.target_purity,
            r.auto_changes.len(),
            r.manual_changes.len()
        );
    }
    println!(
        "  Overall: {:?} ({} auto changes)",
        report.current_purity, report.auto_change_count
    );
    assert!(report.auto_change_count > 0);

    // ── FJ-1314: Pin Tripwire ──
    println!("\n[FJ-1314] Pin Tripwire:");
    let mut pins = BTreeMap::new();
    pins.insert(
        "curl".to_string(),
        Pin {
            provider: "apt".into(),
            version: Some("7.88.1".into()),
            hash: "blake3:aaa".into(),
            git_rev: None,
            pin_type: None,
        },
    );
    pins.insert(
        "jq".to_string(),
        Pin {
            provider: "apt".into(),
            version: Some("1.6".into()),
            hash: "blake3:bbb".into(),
            git_rev: None,
            pin_type: None,
        },
    );
    let lock = LockFile {
        schema: "1.0".into(),
        pins,
    };

    // Stale scenario: curl hash changed
    let mut current = BTreeMap::new();
    current.insert("curl".into(), "blake3:new_hash".into());
    current.insert("jq".into(), "blake3:bbb".into());
    let inputs = vec!["curl".into(), "jq".into(), "missing_pkg".into()];

    let result = check_before_apply(&lock, &current, &inputs);
    let severity = pin_severity(&result, false);
    println!("  All fresh: {}", result.all_fresh);
    println!("  Stale pins: {}", result.stale_pins.len());
    println!("  Missing inputs: {:?}", result.missing_inputs);
    println!("  Severity (non-strict): {:?}", severity);
    println!("  Report:\n{}", format_pin_report(&result));
    assert!(needs_pin_update(&lock, &current, &inputs));

    // ── FJ-1364: Pin Resolution ──
    println!("[FJ-1364] Pin Resolution Commands:");
    for (provider, pkg) in &[
        ("apt", "curl"),
        ("cargo", "serde"),
        ("nix", "hello"),
        ("pip", "requests"),
        ("docker", "nginx"),
    ] {
        if let Some(cmd) = resolution_command(provider, pkg) {
            println!("  {provider}/{pkg}: {}", &cmd[..cmd.len().min(60)]);
        }
    }

    // Version parsing
    println!("\n[FJ-1364] Version Parsing:");
    let apt_out = "curl:\n  Installed: 7.88.1-10\n  Candidate: 7.88.1-10+deb12u7\n  Version table:";
    println!("  apt: {:?}", parse_resolved_version("apt", apt_out));
    let cargo_out = r#"serde = "1.0.215"    # A serialization framework"#;
    println!("  cargo: {:?}", parse_resolved_version("cargo", cargo_out));

    // Pin hashing
    println!("\n[FJ-1364] Pin Hashing:");
    let h1 = pin_hash("apt", "curl", "7.88.1");
    let h2 = pin_hash("apt", "curl", "7.88.1");
    let h3 = pin_hash("apt", "curl", "7.99.0");
    println!("  apt/curl/7.88.1 = {}", &h1[..20]);
    println!("  (same again)    = {}", &h2[..20]);
    println!("  apt/curl/7.99.0 = {}", &h3[..20]);
    assert_eq!(h1, h2, "deterministic");
    assert_ne!(h1, h3, "version-sensitive");

    println!("\n{}", "=".repeat(55));
    println!("All conversion/pin/tripwire criteria survived.");
}
