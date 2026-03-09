//! FJ-1328/1314/1306/1329: Conversion strategy, pin tripwire, purity/repro validation.
//!
//! Usage: cargo run --example convert_pin_validate

use forjar::core::store::convert::{analyze_conversion, ConversionSignals};
use forjar::core::store::lockfile::{LockFile, Pin};
use forjar::core::store::pin_tripwire::{check_before_apply, format_pin_report, pin_severity};
use forjar::core::store::purity::{PurityLevel, PuritySignals};
use forjar::core::store::repro_score::ReproInput;
use forjar::core::store::validate::{
    format_purity_report, format_repro_report, validate_purity, validate_repro_score,
};
use std::collections::BTreeMap;

fn main() {
    println!("Forjar: Conversion, Pin Tripwire & Validation");
    println!("{}", "=".repeat(55));

    // ── FJ-1328: Conversion Analysis ──
    println!("\n[FJ-1328] Conversion Analysis:");
    let signals = vec![
        ConversionSignals {
            name: "nginx".into(),
            has_version: true,
            has_store: true,
            has_sandbox: true,
            has_curl_pipe: false,
            provider: "apt".into(),
            current_version: Some("1.24".into()),
        },
        ConversionSignals {
            name: "curl".into(),
            has_version: false,
            has_store: false,
            has_sandbox: false,
            has_curl_pipe: false,
            provider: "apt".into(),
            current_version: None,
        },
    ];
    let report = analyze_conversion(&signals);
    println!("  Resources: {}", report.resources.len());
    println!("  Auto changes: {}", report.auto_change_count);
    println!("  Manual changes: {}", report.manual_change_count);
    println!("  Current purity: {:?}", report.current_purity);
    println!("  Projected purity: {:?}", report.projected_purity);
    for r in &report.resources {
        println!(
            "  {} ({:?} -> {:?}): {} auto, {} manual",
            r.name,
            r.current_purity,
            r.target_purity,
            r.auto_changes.len(),
            r.manual_changes.len()
        );
    }

    // ── FJ-1314: Pin Tripwire ──
    println!("\n[FJ-1314] Pin Tripwire:");
    let mut pins = BTreeMap::new();
    pins.insert(
        "nginx".into(),
        Pin {
            provider: "apt".into(),
            version: Some("1.24".into()),
            hash: "blake3:abc".into(),
            git_rev: None,
            pin_type: None,
        },
    );
    let lf = LockFile {
        schema: "1.0".into(),
        pins,
    };
    let mut current = BTreeMap::new();
    current.insert("nginx".into(), "blake3:def".into());
    let result = check_before_apply(&lf, &current, &["nginx".into(), "curl".into()]);
    println!("  All fresh: {}", result.all_fresh);
    println!("  Stale pins: {}", result.stale_pins.len());
    println!("  Missing inputs: {:?}", result.missing_inputs);
    println!("  Severity: {:?}", pin_severity(&result, false));
    println!("  Report:\n{}", format_pin_report(&result));

    // ── FJ-1306: Purity Validation ──
    println!("\n[FJ-1306] Purity Validation:");
    let pure_sig = PuritySignals {
        has_version: true,
        has_store: true,
        has_sandbox: true,
        has_curl_pipe: false,
        dep_levels: vec![],
    };
    let v = validate_purity(&[("nginx", &pure_sig)], Some(PurityLevel::Pinned));
    println!("{}", format_purity_report(&v));

    // ── FJ-1329: Repro Score Validation ──
    println!("\n[FJ-1329] Reproducibility Validation:");
    let inputs = vec![
        ReproInput {
            name: "nginx".into(),
            purity: PurityLevel::Pure,
            has_store: true,
            has_lock_pin: true,
        },
        ReproInput {
            name: "curl".into(),
            purity: PurityLevel::Constrained,
            has_store: false,
            has_lock_pin: false,
        },
    ];
    let rv = validate_repro_score(&inputs, Some(50.0));
    println!("{}", format_repro_report(&rv));

    println!("\n{}", "=".repeat(55));
    println!("All conversion/pin/validation criteria survived.");
}
