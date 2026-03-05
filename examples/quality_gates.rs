//! FJ-2702: Quality gate evaluation example.
//!
//! Demonstrates how pipeline quality gates evaluate execution output
//! against exit codes, JSON fields, regex patterns, and numeric thresholds.
//!
//! ```bash
//! cargo run --example quality_gates
//! ```

use forjar::core::task::evaluate_gate;
use forjar::core::types::QualityGate;

fn main() {
    println!("=== FJ-2702: Quality Gate Evaluation ===\n");

    // Gate 1: Exit code gate (simplest)
    let gate = QualityGate::default();
    let result = evaluate_gate(&gate, 0, "");
    println!("Exit code 0: {:?}", result);

    let result = evaluate_gate(&gate, 1, "lint failed");
    println!("Exit code 1: {:?}\n", result);

    // Gate 2: JSON field threshold gate
    let json_gate = QualityGate {
        parse: Some("json".into()),
        field: Some("grade".into()),
        threshold: vec!["A".into(), "B".into()],
        message: Some("Quality score too low".into()),
        ..QualityGate::default()
    };

    let good_output = r#"{"grade":"A","score":95.5}"#;
    let bad_output = r#"{"grade":"D","score":42.0}"#;

    println!("JSON grade=A: {:?}", evaluate_gate(&json_gate, 0, good_output));
    println!("JSON grade=D: {:?}\n", evaluate_gate(&json_gate, 0, bad_output));

    // Gate 3: Numeric minimum gate
    let coverage_gate = QualityGate {
        parse: Some("json".into()),
        field: Some("coverage".into()),
        min: Some(95.0),
        ..QualityGate::default()
    };

    println!(
        "Coverage 96.5%: {:?}",
        evaluate_gate(&coverage_gate, 0, r#"{"coverage":96.5}"#)
    );
    println!(
        "Coverage 90.0%: {:?}\n",
        evaluate_gate(&coverage_gate, 0, r#"{"coverage":90.0}"#)
    );

    // Gate 4: Regex stdout gate
    let regex_gate = QualityGate {
        regex: Some(r"test result: ok\. \d+ passed".into()),
        ..QualityGate::default()
    };

    println!(
        "Regex match: {:?}",
        evaluate_gate(&regex_gate, 0, "test result: ok. 42 passed; 0 failed")
    );
    println!(
        "Regex no match: {:?}\n",
        evaluate_gate(&regex_gate, 0, "COMPILATION FAILED")
    );

    // Gate 5: on_fail=warn (non-blocking)
    let warn_gate = QualityGate {
        on_fail: Some("warn".into()),
        ..QualityGate::default()
    };
    println!("Warn gate (exit 1): {:?}", evaluate_gate(&warn_gate, 1, ""));

    // Summary
    println!("\n=== Gate Actions ===");
    println!("block (default) — halt pipeline");
    println!("warn            — log warning, continue");
    println!("skip_dependents — skip downstream stages");

    // GPU env vars
    println!("\n=== FJ-2703: GPU Device Targeting ===");
    let vars = forjar::core::task::gpu_env_vars(Some(2));
    for (k, v) in &vars {
        println!("{k}={v}");
    }
}
