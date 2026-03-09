//! FJ-2700/2701: Pipeline state building and I/O cache tracking.
//!
//! Popperian rejection criteria for:
//! - FJ-2700: build_pipeline_state (pass, fail, cached, empty)
//! - FJ-2700: stage_command (with/without command)
//! - FJ-2700: format_pipeline_summary (passed, failed, skipped)
//! - FJ-2701: should_skip_cached (cache enabled/disabled, hash match/mismatch)
//!
//! Usage: cargo test --test falsification_pipeline_io_tracking

use forjar::core::task::pipeline::{
    build_pipeline_state, format_pipeline_summary, stage_command, StageExecResult,
};
use forjar::core::task::should_skip_cached;
use forjar::core::types::{PipelineStage, PipelineState, StageState, StageStatus};

// ============================================================================
// FJ-2700: build_pipeline_state
// ============================================================================

#[test]
fn pipeline_state_all_pass() {
    let results = vec![
        StageExecResult {
            name: "lint".into(),
            cached: false,
            exit_code: 0,
            duration_ms: 200,
            input_hash: None,
        },
        StageExecResult {
            name: "test".into(),
            cached: false,
            exit_code: 0,
            duration_ms: 1500,
            input_hash: None,
        },
    ];
    let state = build_pipeline_state(&results);
    assert_eq!(state.status, StageStatus::Passed);
    assert_eq!(state.last_completed, Some(1));
    assert_eq!(state.stages.len(), 2);
    assert_eq!(state.stages[0].status, StageStatus::Passed);
    assert_eq!(state.stages[1].status, StageStatus::Passed);
}

#[test]
fn pipeline_state_with_failure() {
    let results = vec![
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
    let state = build_pipeline_state(&results);
    assert_eq!(state.status, StageStatus::Failed);
    assert_eq!(state.last_completed, Some(0));
    assert_eq!(state.stages[1].status, StageStatus::Failed);
    assert_eq!(state.stages[1].exit_code, Some(1));
}

#[test]
fn pipeline_state_with_cache_skip() {
    let results = vec![
        StageExecResult {
            name: "pull".into(),
            cached: true,
            exit_code: 0,
            duration_ms: 0,
            input_hash: Some("blake3:abc".into()),
        },
        StageExecResult {
            name: "convert".into(),
            cached: false,
            exit_code: 0,
            duration_ms: 2000,
            input_hash: None,
        },
    ];
    let state = build_pipeline_state(&results);
    assert_eq!(state.status, StageStatus::Passed);
    assert_eq!(state.stages[0].status, StageStatus::Skipped);
    assert_eq!(state.stages[1].status, StageStatus::Passed);
    assert_eq!(state.last_completed, Some(1));
}

#[test]
fn pipeline_state_empty() {
    let state = build_pipeline_state(&[]);
    assert_eq!(state.status, StageStatus::Passed);
    assert!(state.stages.is_empty());
    assert!(state.last_completed.is_none());
}

#[test]
fn pipeline_state_preserves_duration_and_hash() {
    let results = vec![StageExecResult {
        name: "build".into(),
        cached: false,
        exit_code: 0,
        duration_ms: 42000,
        input_hash: Some("blake3:xyz".into()),
    }];
    let state = build_pipeline_state(&results);
    assert_eq!(state.stages[0].duration_ms, Some(42000));
    assert_eq!(state.stages[0].input_hash, Some("blake3:xyz".into()));
}

// ============================================================================
// FJ-2700: stage_command
// ============================================================================

#[test]
fn stage_command_with_cmd() {
    let stage = PipelineStage {
        name: "build".into(),
        command: Some("cargo build --release".into()),
        ..Default::default()
    };
    let cmd = stage_command(&stage);
    assert!(cmd.starts_with("set -euo pipefail"));
    assert!(cmd.contains("cargo build --release"));
    assert!(cmd.ends_with('\n'));
}

#[test]
fn stage_command_none() {
    let stage = PipelineStage {
        name: "noop".into(),
        command: None,
        ..Default::default()
    };
    assert_eq!(stage_command(&stage), "true\n");
}

// ============================================================================
// FJ-2700: format_pipeline_summary
// ============================================================================

#[test]
fn summary_passed() {
    let state = PipelineState {
        stages: vec![
            StageState {
                name: "lint".into(),
                status: StageStatus::Passed,
                exit_code: Some(0),
                duration_ms: Some(200),
                input_hash: None,
            },
            StageState {
                name: "test".into(),
                status: StageStatus::Passed,
                exit_code: Some(0),
                duration_ms: Some(1500),
                input_hash: None,
            },
        ],
        status: StageStatus::Passed,
        last_completed: Some(1),
    };
    let summary = format_pipeline_summary(&state);
    assert!(summary.contains("PASS"));
    assert!(summary.contains("lint"));
    assert!(summary.contains("test"));
    assert!(summary.contains("Pipeline: PASSED"));
    assert!(summary.contains("(200ms)"));
}

#[test]
fn summary_failed() {
    let state = PipelineState {
        stages: vec![StageState {
            name: "deploy".into(),
            status: StageStatus::Failed,
            exit_code: Some(1),
            duration_ms: Some(100),
            input_hash: None,
        }],
        status: StageStatus::Failed,
        last_completed: None,
    };
    let summary = format_pipeline_summary(&state);
    assert!(summary.contains("FAIL"));
    assert!(summary.contains("deploy"));
    assert!(summary.contains("Pipeline: FAILED"));
}

#[test]
fn summary_with_skip() {
    let state = PipelineState {
        stages: vec![StageState {
            name: "cached".into(),
            status: StageStatus::Skipped,
            exit_code: Some(0),
            duration_ms: Some(0),
            input_hash: Some("blake3:hash".into()),
        }],
        status: StageStatus::Passed,
        last_completed: Some(0),
    };
    let summary = format_pipeline_summary(&state);
    assert!(summary.contains("SKIP"));
    assert!(summary.contains("cached"));
}

#[test]
fn summary_pending_and_running() {
    let state = PipelineState {
        stages: vec![
            StageState {
                name: "waiting".into(),
                status: StageStatus::Pending,
                exit_code: None,
                duration_ms: None,
                input_hash: None,
            },
            StageState {
                name: "active".into(),
                status: StageStatus::Running,
                exit_code: None,
                duration_ms: None,
                input_hash: None,
            },
        ],
        status: StageStatus::Running,
        last_completed: None,
    };
    let summary = format_pipeline_summary(&state);
    assert!(summary.contains("PENDING"));
    assert!(summary.contains("RUNNING"));
    assert!(summary.contains("INCOMPLETE"));
}

// ============================================================================
// FJ-2701: should_skip_cached
// ============================================================================

#[test]
fn skip_cache_disabled() {
    assert!(!should_skip_cached(false, None, None));
    assert!(!should_skip_cached(
        false,
        Some("blake3:a"),
        Some("blake3:a")
    ));
}

#[test]
fn skip_cache_enabled_match() {
    assert!(should_skip_cached(
        true,
        Some("blake3:abc"),
        Some("blake3:abc")
    ));
}

#[test]
fn skip_cache_enabled_mismatch() {
    assert!(!should_skip_cached(
        true,
        Some("blake3:abc"),
        Some("blake3:def")
    ));
}

#[test]
fn skip_cache_enabled_missing_hashes() {
    assert!(!should_skip_cached(true, None, Some("blake3:abc")));
    assert!(!should_skip_cached(true, Some("blake3:abc"), None));
    assert!(!should_skip_cached(true, None, None));
}
