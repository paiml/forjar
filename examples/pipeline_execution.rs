//! FJ-2700: Pipeline execution example.
//!
//! Demonstrates multi-stage pipeline execution with caching,
//! gate stages, and state tracking.
//!
//! ```bash
//! cargo run --example pipeline_execution
//! ```

use forjar::core::task::pipeline::{
    build_pipeline_state, format_pipeline_summary, plan_pipeline, stage_command, StageExecResult,
};
use forjar::core::types::{PipelineStage, PipelineState, StageState, StageStatus};

fn main() {
    demo_pipeline_plan();
    demo_pipeline_execution();
    demo_cached_rerun();
}

fn demo_pipeline_plan() {
    println!("=== FJ-2700: Pipeline Plan ===\n");

    let stages = vec![
        PipelineStage {
            name: "pull".into(),
            command: Some("apr pull model.gguf".into()),
            outputs: vec!["/opt/models/raw.gguf".into()],
            ..Default::default()
        },
        PipelineStage {
            name: "convert".into(),
            command: Some("apr convert /opt/models/raw.gguf --quantization q4_k_m".into()),
            inputs: vec!["/opt/models/raw.gguf".into()],
            outputs: vec!["/opt/models/model.apr".into()],
            ..Default::default()
        },
        PipelineStage {
            name: "verify".into(),
            command: Some("apr qa /opt/models/model.apr --gates G0,G1,G2".into()),
            inputs: vec!["/opt/models/model.apr".into()],
            gate: true,
            ..Default::default()
        },
    ];

    let plan = plan_pipeline(
        &stages,
        &PipelineState::default(),
        true,
        std::path::Path::new("."),
    );
    for entry in &plan {
        let gate = if entry.is_gate { " [GATE]" } else { "" };
        let skip = if entry.skip { " (cached)" } else { "" };
        println!("  Stage: {}{gate}{skip}", entry.name);
    }

    println!("\n  Stage commands:");
    for stage in &stages {
        let cmd = stage_command(stage);
        println!("    {}: {}", stage.name, cmd.trim());
    }
    println!();
}

fn demo_pipeline_execution() {
    println!("=== FJ-2700: Pipeline Execution ===\n");

    let results = vec![
        StageExecResult {
            name: "pull".into(),
            cached: false,
            exit_code: 0,
            duration_ms: 15000,
            input_hash: None,
        },
        StageExecResult {
            name: "convert".into(),
            cached: false,
            exit_code: 0,
            duration_ms: 45000,
            input_hash: Some("blake3:a1b2c3d4e5f6".into()),
        },
        StageExecResult {
            name: "verify".into(),
            cached: false,
            exit_code: 0,
            duration_ms: 5000,
            input_hash: Some("blake3:f6e5d4c3b2a1".into()),
        },
    ];

    let state = build_pipeline_state(&results);
    print!("{}", format_pipeline_summary(&state));
    println!();
}

fn demo_cached_rerun() {
    println!("=== FJ-2700: Cached Re-run ===\n");

    let previous = PipelineState {
        stages: vec![
            StageState {
                name: "pull".into(),
                status: StageStatus::Passed,
                exit_code: Some(0),
                duration_ms: Some(15000),
                input_hash: Some("blake3:aaa".into()),
            },
            StageState {
                name: "convert".into(),
                status: StageStatus::Passed,
                exit_code: Some(0),
                duration_ms: Some(45000),
                input_hash: Some("blake3:bbb".into()),
            },
        ],
        status: StageStatus::Passed,
        last_completed: Some(1),
    };

    // Second run: pull was cached, convert had new inputs → re-ran
    let results = vec![
        StageExecResult {
            name: "pull".into(),
            cached: true,
            exit_code: 0,
            duration_ms: 0,
            input_hash: Some("blake3:aaa".into()),
        },
        StageExecResult {
            name: "convert".into(),
            cached: false,
            exit_code: 0,
            duration_ms: 42000,
            input_hash: Some("blake3:ccc".into()),
        },
        StageExecResult {
            name: "verify".into(),
            cached: false,
            exit_code: 1,
            duration_ms: 3000,
            input_hash: None,
        },
    ];

    let state = build_pipeline_state(&results);
    print!("{}", format_pipeline_summary(&state));

    println!("\n  Previous pipeline state:");
    print!("{}", format_pipeline_summary(&previous));
}
