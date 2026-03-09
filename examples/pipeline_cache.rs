//! FJ-2700/2701: Pipeline state building and I/O cache tracking.
//!
//! Demonstrates:
//! - Building pipeline state from stage execution results
//! - Pipeline summary formatting with status labels
//! - Content-addressed cache skip logic
//!
//! Usage: cargo run --example pipeline_cache

use forjar::core::task::pipeline::{
    build_pipeline_state, format_pipeline_summary, stage_command, StageExecResult,
};
use forjar::core::task::should_skip_cached;
use forjar::core::types::{PipelineStage, StageStatus};

fn main() {
    println!("Forjar: Pipeline State & I/O Cache Tracking");
    println!("{}", "=".repeat(50));

    // ── Pipeline State Building ──
    println!("\n[FJ-2700] Pipeline State (all pass):");
    let results = vec![
        StageExecResult {
            name: "lint".into(),
            cached: false,
            exit_code: 0,
            duration_ms: 200,
            input_hash: Some("blake3:aaa".into()),
        },
        StageExecResult {
            name: "test".into(),
            cached: false,
            exit_code: 0,
            duration_ms: 1500,
            input_hash: None,
        },
        StageExecResult {
            name: "deploy".into(),
            cached: true,
            exit_code: 0,
            duration_ms: 0,
            input_hash: Some("blake3:bbb".into()),
        },
    ];
    let state = build_pipeline_state(&results);
    println!("{}", format_pipeline_summary(&state));
    assert_eq!(state.status, StageStatus::Passed);

    // With failure
    println!("[FJ-2700] Pipeline State (with failure):");
    let results_fail = vec![
        StageExecResult {
            name: "build".into(),
            cached: false,
            exit_code: 0,
            duration_ms: 5000,
            input_hash: None,
        },
        StageExecResult {
            name: "test".into(),
            cached: false,
            exit_code: 1,
            duration_ms: 3000,
            input_hash: None,
        },
    ];
    let state_fail = build_pipeline_state(&results_fail);
    println!("{}", format_pipeline_summary(&state_fail));
    assert_eq!(state_fail.status, StageStatus::Failed);

    // Stage command generation
    println!("[FJ-2700] Stage Commands:");
    let stage = PipelineStage {
        name: "build".into(),
        command: Some("cargo build --release".into()),
        ..Default::default()
    };
    let cmd = stage_command(&stage);
    println!("  build: {}", cmd.trim());

    // ── Cache Skip Logic ──
    println!("\n[FJ-2701] Cache Skip Decisions:");
    let cases = [
        (false, None, None, "no cache"),
        (true, Some("blake3:a"), Some("blake3:a"), "match"),
        (true, Some("blake3:a"), Some("blake3:b"), "mismatch"),
        (true, None, Some("blake3:a"), "missing current"),
    ];
    for (cache, cur, stored, label) in &cases {
        let skip = should_skip_cached(*cache, *cur, *stored);
        println!("  {label}: skip={skip}");
    }
    assert!(should_skip_cached(true, Some("blake3:x"), Some("blake3:x")));
    assert!(!should_skip_cached(
        true,
        Some("blake3:x"),
        Some("blake3:y")
    ));

    println!("\n{}", "=".repeat(50));
    println!("All pipeline/cache criteria survived.");
}
