//! FJ-2604/2301: Mutation testing types and observability types.
//! Usage: cargo test --test falsification_mutation_observability

use forjar::core::types::*;

// ── FJ-2604: MutationOperator ──

#[test]
fn mutation_operator_descriptions() {
    assert_eq!(
        MutationOperator::DeleteFile.description(),
        "Remove a managed file"
    );
    assert_eq!(
        MutationOperator::StopService.description(),
        "Stop a managed service"
    );
    assert_eq!(
        MutationOperator::RemovePackage.description(),
        "Remove a managed package"
    );
}

#[test]
fn mutation_operator_applicable_types() {
    assert_eq!(MutationOperator::DeleteFile.applicable_types(), &["file"]);
    assert_eq!(
        MutationOperator::StopService.applicable_types(),
        &["service"]
    );
    assert_eq!(
        MutationOperator::RemovePackage.applicable_types(),
        &["package"]
    );
    assert_eq!(
        MutationOperator::UnmountFilesystem.applicable_types(),
        &["mount"]
    );
}

#[test]
fn mutation_operator_display() {
    assert_eq!(MutationOperator::DeleteFile.to_string(), "delete_file");
    assert_eq!(
        MutationOperator::ModifyContent.to_string(),
        "modify_content"
    );
    assert_eq!(
        MutationOperator::ChangePermissions.to_string(),
        "change_permissions"
    );
    assert_eq!(MutationOperator::KillProcess.to_string(), "kill_process");
}

#[test]
fn mutation_operator_serde() {
    for op in [
        MutationOperator::DeleteFile,
        MutationOperator::ModifyContent,
        MutationOperator::StopService,
        MutationOperator::RemovePackage,
    ] {
        let json = serde_json::to_string(&op).unwrap();
        let parsed: MutationOperator = serde_json::from_str(&json).unwrap();
        assert_eq!(op, parsed);
    }
}

// ── FJ-2604: MutationResult ──

fn mr(id: &str, rtype: &str, op: MutationOperator, detected: bool) -> MutationResult {
    MutationResult {
        resource_id: id.into(),
        resource_type: rtype.into(),
        operator: op,
        detected,
        reconverged: None,
        duration_ms: 100,
        error: None,
    }
}

#[test]
fn mutation_result_killed() {
    let r = mr("f", "file", MutationOperator::DeleteFile, true);
    assert!(r.is_killed());
    assert!(!r.is_survived());
}

#[test]
fn mutation_result_survived() {
    let r = mr("f", "file", MutationOperator::ModifyContent, false);
    assert!(r.is_survived());
    assert!(!r.is_killed());
}

#[test]
fn mutation_result_display() {
    let r = mr("nginx", "file", MutationOperator::DeleteFile, true);
    let s = format!("{r}");
    assert!(s.contains("KILLED"));
    assert!(s.contains("nginx"));
}

// ── FJ-2604: MutationScore ──

#[test]
fn mutation_score_pct() {
    let s = MutationScore {
        total: 20,
        detected: 18,
        survived: 2,
        errored: 0,
    };
    assert!((s.score_pct() - 90.0).abs() < 0.01);
    assert_eq!(s.grade(), 'A');
}

#[test]
fn mutation_score_grades() {
    assert_eq!(
        (MutationScore {
            total: 10,
            detected: 10,
            survived: 0,
            errored: 0
        })
        .grade(),
        'A'
    );
    assert_eq!(
        (MutationScore {
            total: 10,
            detected: 8,
            survived: 2,
            errored: 0
        })
        .grade(),
        'B'
    );
    assert_eq!(
        (MutationScore {
            total: 10,
            detected: 6,
            survived: 4,
            errored: 0
        })
        .grade(),
        'C'
    );
    assert_eq!(
        (MutationScore {
            total: 10,
            detected: 5,
            survived: 5,
            errored: 0
        })
        .grade(),
        'F'
    );
}

#[test]
fn mutation_score_empty() {
    let s = MutationScore::default();
    assert_eq!(s.score_pct(), 100.0); // 0/0 = 100%
    assert_eq!(s.grade(), 'A');
}

#[test]
fn mutation_score_display() {
    let s = MutationScore {
        total: 10,
        detected: 8,
        survived: 2,
        errored: 0,
    };
    let text = format!("{s}");
    assert!(text.contains("80%"));
    assert!(text.contains("Grade B"));
    assert!(text.contains("8/10"));
}

// ── FJ-2604: TypeMutationSummary ──

#[test]
fn type_mutation_summary_pct() {
    let s = TypeMutationSummary {
        resource_type: "file".into(),
        total: 10,
        detected: 7,
    };
    assert!((s.detection_pct() - 70.0).abs() < 0.01);
}

#[test]
fn type_mutation_summary_empty() {
    let s = TypeMutationSummary {
        resource_type: "file".into(),
        total: 0,
        detected: 0,
    };
    assert_eq!(s.detection_pct(), 100.0);
}

#[test]
fn type_mutation_summary_display() {
    let s = TypeMutationSummary {
        resource_type: "service".into(),
        total: 5,
        detected: 5,
    };
    let text = format!("{s}");
    assert!(text.contains("service"));
    assert!(text.contains("5/5"));
    assert!(text.contains("100%"));
}

// ── FJ-2604: MutationReport ──

#[test]
fn mutation_report_from_results() {
    let results = vec![
        mr("f1", "file", MutationOperator::DeleteFile, true),
        mr("f2", "file", MutationOperator::ModifyContent, false),
        mr("s1", "service", MutationOperator::StopService, true),
    ];
    let report = MutationReport::from_results(results);
    assert_eq!(report.score.total, 3);
    assert_eq!(report.score.detected, 2);
    assert_eq!(report.score.survived, 1);
    assert_eq!(report.undetected.len(), 1);
    assert_eq!(report.undetected[0].resource_id, "f2");
    assert_eq!(report.by_type.len(), 2); // file, service
}

#[test]
fn mutation_report_format_summary() {
    let results = vec![
        mr("f1", "file", MutationOperator::DeleteFile, true),
        mr("f2", "file", MutationOperator::ModifyContent, false),
    ];
    let report = MutationReport::from_results(results);
    let s = report.format_summary();
    assert!(s.contains("Mutation Score"));
    assert!(s.contains("file"));
    assert!(s.contains("Undetected"));
}

#[test]
fn mutation_report_all_detected() {
    let results = vec![mr("a", "file", MutationOperator::DeleteFile, true)];
    let report = MutationReport::from_results(results);
    assert!(report.undetected.is_empty());
    assert_eq!(report.score.grade(), 'A');
}

#[test]
fn mutation_report_errored() {
    let mut r = mr("x", "file", MutationOperator::DeleteFile, false);
    r.error = Some("sandbox failed".into());
    let report = MutationReport::from_results(vec![r]);
    assert_eq!(report.score.errored, 1);
    assert_eq!(report.score.survived, 0); // errored != survived
}

// ── FJ-2301: VerbosityLevel ──

#[test]
fn verbosity_from_count() {
    assert_eq!(VerbosityLevel::from_count(0), VerbosityLevel::Normal);
    assert_eq!(VerbosityLevel::from_count(1), VerbosityLevel::Verbose);
    assert_eq!(VerbosityLevel::from_count(2), VerbosityLevel::VeryVerbose);
    assert_eq!(VerbosityLevel::from_count(3), VerbosityLevel::Trace);
    assert_eq!(VerbosityLevel::from_count(10), VerbosityLevel::Trace);
}

#[test]
fn verbosity_streams_raw() {
    assert!(!VerbosityLevel::Normal.streams_raw());
    assert!(!VerbosityLevel::VeryVerbose.streams_raw());
    assert!(VerbosityLevel::Trace.streams_raw());
}

#[test]
fn verbosity_shows_scripts() {
    assert!(!VerbosityLevel::Normal.shows_scripts());
    assert!(!VerbosityLevel::Verbose.shows_scripts());
    assert!(VerbosityLevel::VeryVerbose.shows_scripts());
    assert!(VerbosityLevel::Trace.shows_scripts());
}

#[test]
fn verbosity_display() {
    assert_eq!(VerbosityLevel::Normal.to_string(), "normal");
    assert_eq!(VerbosityLevel::Trace.to_string(), "trace");
}

#[test]
fn verbosity_serde() {
    let json = serde_json::to_string(&VerbosityLevel::VeryVerbose).unwrap();
    let parsed: VerbosityLevel = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, VerbosityLevel::VeryVerbose);
}

// ── FJ-2301: LogFilter ──

#[test]
fn log_filter_for_machine() {
    let f = LogFilter::for_machine("intel");
    assert_eq!(f.machine.as_deref(), Some("intel"));
    assert!(f.has_criteria());
}

#[test]
fn log_filter_for_run() {
    let f = LogFilter::for_run("r-123");
    assert_eq!(f.run_id.as_deref(), Some("r-123"));
    assert!(f.has_criteria());
}

#[test]
fn log_filter_failures() {
    let f = LogFilter::failures();
    assert!(f.failures_only);
    assert!(f.has_criteria());
}

#[test]
fn log_filter_default_no_criteria() {
    assert!(!LogFilter::default().has_criteria());
}

// ── FJ-2301: LogTruncation ──

#[test]
fn log_truncation_defaults() {
    let t = LogTruncation::default();
    assert_eq!(t.first_bytes, 8192);
    assert_eq!(t.last_bytes, 8192);
}

#[test]
fn log_truncation_should_truncate() {
    let t = LogTruncation::default();
    assert!(!t.should_truncate(100));
    assert!(!t.should_truncate(16384));
    assert!(t.should_truncate(16385));
}

#[test]
fn log_truncation_small_passthrough() {
    let t = LogTruncation {
        first_bytes: 5,
        last_bytes: 5,
    };
    assert_eq!(t.truncate("short"), "short");
}

#[test]
fn log_truncation_large() {
    let t = LogTruncation {
        first_bytes: 5,
        last_bytes: 5,
    };
    let input = "ABCDE__middle__FGHIJ";
    let result = t.truncate(input);
    assert!(result.starts_with("ABCDE"));
    assert!(result.ends_with("FGHIJ"));
    assert!(result.contains("TRUNCATED"));
    assert!(result.contains("10 bytes omitted"));
}

// ── FJ-2301: LogGcResult ──

#[test]
fn log_gc_mb_freed() {
    let gc = LogGcResult {
        runs_removed: 3,
        bytes_freed: 10 * 1024 * 1024,
        runs_kept: 7,
    };
    assert!((gc.mb_freed() - 10.0).abs() < 0.01);
}

#[test]
fn log_gc_display() {
    let gc = LogGcResult {
        runs_removed: 2,
        bytes_freed: 5 * 1024 * 1024,
        runs_kept: 8,
    };
    let s = gc.to_string();
    assert!(s.contains("removed 2 runs"));
    assert!(s.contains("5.0 MB"));
    assert!(s.contains("8 runs kept"));
}

// ── FJ-2301: RunLogPath ──

#[test]
fn run_log_path_builder() {
    let p = RunLogPath::new("state", "intel", "r-abc");
    assert_eq!(p.run_dir(), "state/intel/runs/r-abc");
    assert_eq!(
        p.resource_log("nginx", "apply"),
        "state/intel/runs/r-abc/nginx.apply.log"
    );
    assert_eq!(p.meta_path(), "state/intel/runs/r-abc/meta.yaml");
    assert_eq!(p.runs_dir(), "state/intel/runs");
}

#[test]
fn run_log_path_actions() {
    let p = RunLogPath::new("s", "m", "r-1");
    assert_eq!(p.resource_log("pkg", "check"), "s/m/runs/r-1/pkg.check.log");
    assert_eq!(
        p.resource_log("svc", "destroy"),
        "s/m/runs/r-1/svc.destroy.log"
    );
}

// ── FJ-2301: ProgressConfig ──

#[test]
fn progress_config_defaults() {
    let pc = ProgressConfig::default();
    assert!(pc.show_progress);
    assert_eq!(pc.update_interval_ms, 100);
}

// ── FJ-2301: StructuredLogOutput serde ──

#[test]
fn structured_log_output_serde() {
    let out = StructuredLogOutput {
        run_id: "r-1".into(),
        machine: "m".into(),
        resource_id: "pkg".into(),
        log_path: "state/m/runs/r-1/pkg.apply.log".into(),
        exit_code: 0,
        duration_secs: 1.0,
        truncated: false,
    };
    let json = serde_json::to_string(&out).unwrap();
    let parsed: StructuredLogOutput = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.run_id, "r-1");
    assert!(!parsed.truncated);
}
