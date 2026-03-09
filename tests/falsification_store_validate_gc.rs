//! FJ-1306/1329/1325/1326/2301: Purity/repro validation, GC, and run capture falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-1306: validate_purity — pass/fail with min level
//!   - format_purity_report display
//! - FJ-1329: validate_repro_score — pass/fail with min score
//!   - format_repro_report display
//! - FJ-1325/1326: GC roots and mark-and-sweep
//!   - collect_roots from profiles, lockfiles, gc-roots dir
//!   - mark_and_sweep: live/dead classification
//! - FJ-2301: Run log capture
//!   - run_dir path derivation
//!   - ensure_run_dir creates directory and meta.yaml
//!   - capture_output writes log/script/json files
//!   - update_meta_resource records status
//!
//! Usage: cargo test --test falsification_store_validate_gc

use forjar::core::executor::run_capture;
use forjar::core::store::gc::{collect_roots, mark_and_sweep, GcConfig};
use forjar::core::store::purity::{PurityLevel, PuritySignals};
use forjar::core::store::repro_score::ReproInput;
use forjar::core::store::validate::{
    format_purity_report, format_repro_report, validate_purity, validate_repro_score,
};
use forjar::core::types::ResourceRunStatus;
use forjar::transport::ExecOutput;
use std::collections::BTreeSet;

// ============================================================================
// FJ-1306: validate_purity
// ============================================================================

#[test]
fn validate_purity_all_pure_passes() {
    let signals = PuritySignals {
        has_version: true,
        has_store: true,
        has_sandbox: true,
        has_curl_pipe: false,
        dep_levels: vec![],
    };
    let result = validate_purity(&[("nginx", &signals)], None);
    assert!(result.pass);
    assert_eq!(result.recipe_purity, PurityLevel::Pure);
}

#[test]
fn validate_purity_fails_min_level() {
    let signals = PuritySignals {
        has_version: false,
        has_store: false,
        has_sandbox: false,
        has_curl_pipe: false,
        dep_levels: vec![],
    };
    let result = validate_purity(&[("pkg", &signals)], Some(PurityLevel::Pinned));
    assert!(!result.pass);
    assert_eq!(result.recipe_purity, PurityLevel::Constrained);
    assert_eq!(result.required_level, Some(PurityLevel::Pinned));
}

#[test]
fn validate_purity_passes_min_level_exact() {
    let signals = PuritySignals {
        has_version: true,
        has_store: true,
        has_sandbox: false,
        has_curl_pipe: false,
        dep_levels: vec![],
    };
    let result = validate_purity(&[("pkg", &signals)], Some(PurityLevel::Pinned));
    assert!(result.pass);
}

#[test]
fn validate_purity_multiple_resources() {
    let pure_signals = PuritySignals {
        has_version: true,
        has_store: true,
        has_sandbox: true,
        has_curl_pipe: false,
        dep_levels: vec![],
    };
    let impure_signals = PuritySignals {
        has_version: true,
        has_store: true,
        has_sandbox: true,
        has_curl_pipe: true,
        dep_levels: vec![],
    };
    let result = validate_purity(
        &[("nginx", &pure_signals), ("dodgy", &impure_signals)],
        Some(PurityLevel::Pinned),
    );
    assert!(!result.pass);
    assert_eq!(result.recipe_purity, PurityLevel::Impure);
    assert_eq!(result.resources.len(), 2);
}

#[test]
fn format_purity_report_contains_levels() {
    let signals = PuritySignals {
        has_version: true,
        has_store: true,
        has_sandbox: true,
        has_curl_pipe: false,
        dep_levels: vec![],
    };
    let validation = validate_purity(&[("nginx", &signals)], None);
    let report = format_purity_report(&validation);
    assert!(report.contains("Pure (0)"));
    assert!(report.contains("PASS"));
    assert!(report.contains("nginx"));
}

#[test]
fn format_purity_report_shows_required() {
    let signals = PuritySignals::default();
    let validation = validate_purity(&[("pkg", &signals)], Some(PurityLevel::Pure));
    let report = format_purity_report(&validation);
    assert!(report.contains("Required:"));
    assert!(report.contains("FAIL"));
}

// ============================================================================
// FJ-1329: validate_repro_score
// ============================================================================

#[test]
fn validate_repro_passes_no_min() {
    let inputs = vec![ReproInput {
        name: "pkg".into(),
        purity: PurityLevel::Impure,
        has_store: false,
        has_lock_pin: false,
    }];
    let result = validate_repro_score(&inputs, None);
    assert!(result.pass);
    assert_eq!(result.grade, "F");
}

#[test]
fn validate_repro_fails_min_score() {
    let inputs = vec![ReproInput {
        name: "pkg".into(),
        purity: PurityLevel::Impure,
        has_store: false,
        has_lock_pin: false,
    }];
    let result = validate_repro_score(&inputs, Some(50.0));
    assert!(!result.pass);
    assert_eq!(result.required_min, Some(50.0));
}

#[test]
fn validate_repro_passes_min_score() {
    let inputs = vec![ReproInput {
        name: "pkg".into(),
        purity: PurityLevel::Pure,
        has_store: true,
        has_lock_pin: true,
    }];
    let result = validate_repro_score(&inputs, Some(90.0));
    assert!(result.pass);
    assert_eq!(result.grade, "A");
}

#[test]
fn format_repro_report_contains_dimensions() {
    let inputs = vec![ReproInput {
        name: "pkg".into(),
        purity: PurityLevel::Pinned,
        has_store: true,
        has_lock_pin: false,
    }];
    let validation = validate_repro_score(&inputs, None);
    let report = format_repro_report(&validation);
    assert!(report.contains("Reproducibility:"));
    assert!(report.contains("Purity:"));
    assert!(report.contains("Store:"));
    assert!(report.contains("Lock:"));
    assert!(report.contains("PASS"));
}

#[test]
fn format_repro_report_shows_required() {
    let inputs = vec![ReproInput {
        name: "pkg".into(),
        purity: PurityLevel::Impure,
        has_store: false,
        has_lock_pin: false,
    }];
    let validation = validate_repro_score(&inputs, Some(75.0));
    let report = format_repro_report(&validation);
    assert!(report.contains("Required: >= 75.0"));
    assert!(report.contains("FAIL"));
}

// ============================================================================
// FJ-1325: collect_roots
// ============================================================================

#[test]
fn collect_roots_from_profiles() {
    let profiles = vec!["h1".into(), "h2".into()];
    let roots = collect_roots(&profiles, &[], None);
    assert!(roots.contains("h1"));
    assert!(roots.contains("h2"));
    assert_eq!(roots.len(), 2);
}

#[test]
fn collect_roots_from_lockfiles() {
    let locks = vec!["h3".into(), "h4".into()];
    let roots = collect_roots(&[], &locks, None);
    assert!(roots.contains("h3"));
    assert!(roots.contains("h4"));
}

#[test]
fn collect_roots_deduplicates() {
    let profiles = vec!["h1".into()];
    let locks = vec!["h1".into()];
    let roots = collect_roots(&profiles, &locks, None);
    assert_eq!(roots.len(), 1);
}

#[test]
fn collect_roots_empty_sources() {
    let roots = collect_roots(&[], &[], None);
    assert!(roots.is_empty());
}

#[test]
fn collect_roots_with_missing_gc_dir() {
    let roots = collect_roots(
        &[],
        &[],
        Some(std::path::Path::new("/nonexistent/gc-roots")),
    );
    assert!(roots.is_empty());
}

// ============================================================================
// FJ-1326: mark_and_sweep
// ============================================================================

#[test]
fn gc_empty_store() {
    let dir = tempfile::tempdir().unwrap();
    let store_dir = dir.path().join("store");
    std::fs::create_dir_all(&store_dir).unwrap();

    let roots = BTreeSet::new();
    let report = mark_and_sweep(&roots, &store_dir).unwrap();
    assert_eq!(report.total, 0);
    assert!(report.live.is_empty());
    assert!(report.dead.is_empty());
}

#[test]
fn gc_config_defaults() {
    let config = GcConfig::default();
    assert_eq!(config.keep_generations, 5);
    assert!(config.older_than_days.is_none());
}

// ============================================================================
// FJ-2301: run_dir path derivation
// ============================================================================

#[test]
fn run_dir_path() {
    let path = run_capture::run_dir(std::path::Path::new("/state"), "web", "run-123");
    assert_eq!(path, std::path::PathBuf::from("/state/web/runs/run-123"));
}

// ============================================================================
// FJ-2301: ensure_run_dir creates directory and meta.yaml
// ============================================================================

#[test]
fn ensure_run_dir_creates_dir() {
    let dir = tempfile::tempdir().unwrap();
    let run_dir = dir.path().join("state/web/runs/run-001");
    run_capture::ensure_run_dir(&run_dir, "run-001", "web", "apply");
    assert!(run_dir.exists());
    assert!(run_dir.join("meta.yaml").exists());
}

#[test]
fn ensure_run_dir_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let run_dir = dir.path().join("state/web/runs/run-001");
    run_capture::ensure_run_dir(&run_dir, "run-001", "web", "apply");
    let first_meta = std::fs::read_to_string(run_dir.join("meta.yaml")).unwrap();
    run_capture::ensure_run_dir(&run_dir, "run-001", "web", "apply");
    let second_meta = std::fs::read_to_string(run_dir.join("meta.yaml")).unwrap();
    assert_eq!(first_meta, second_meta, "should not overwrite on re-run");
}

// ============================================================================
// FJ-2301: capture_output writes log/script/json files
// ============================================================================

#[test]
fn capture_output_creates_files() {
    let dir = tempfile::tempdir().unwrap();
    let run_dir = dir.path().join("run");
    std::fs::create_dir_all(&run_dir).unwrap();

    let output = ExecOutput {
        stdout: "installed nginx".into(),
        stderr: String::new(),
        exit_code: 0,
    };
    run_capture::capture_output(
        &run_dir,
        "pkg-nginx",
        "package",
        "apply",
        "web",
        "local",
        "apt-get install -y nginx",
        &output,
        1.5,
    );

    assert!(run_dir.join("pkg-nginx.apply.log").exists());
    assert!(run_dir.join("pkg-nginx.apply.json").exists());
    assert!(run_dir.join("pkg-nginx.script").exists());

    let script = std::fs::read_to_string(run_dir.join("pkg-nginx.script")).unwrap();
    assert_eq!(script, "apt-get install -y nginx");

    let log = std::fs::read_to_string(run_dir.join("pkg-nginx.apply.log")).unwrap();
    assert!(log.contains("installed nginx"));
}

#[test]
fn capture_output_no_dir_no_crash() {
    let output = ExecOutput {
        stdout: "ok".into(),
        stderr: String::new(),
        exit_code: 0,
    };
    // Should silently return without writing
    run_capture::capture_output(
        std::path::Path::new("/nonexistent/run"),
        "pkg",
        "package",
        "apply",
        "web",
        "local",
        "echo ok",
        &output,
        0.1,
    );
}

// ============================================================================
// FJ-2301: update_meta_resource records status
// ============================================================================

#[test]
fn update_meta_resource_records_status() {
    let dir = tempfile::tempdir().unwrap();
    let run_dir = dir.path().join("run");
    run_capture::ensure_run_dir(&run_dir, "run-001", "web", "apply");

    run_capture::update_meta_resource(
        &run_dir,
        "pkg-nginx",
        ResourceRunStatus::Converged {
            exit_code: Some(0),
            duration_secs: Some(1.2),
            failed: false,
        },
    );

    let meta = std::fs::read_to_string(run_dir.join("meta.yaml")).unwrap();
    assert!(meta.contains("pkg-nginx"));

    // Should also write meta.json
    assert!(run_dir.join("meta.json").exists());
}
