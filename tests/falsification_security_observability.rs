//! FJ-2300/2301: Popperian falsification for security model and observability.
//!
//! Each test states conditions under which the security or observability
//! system would be rejected as invalid.

use forjar::core::security_scanner::{scan, severity_counts, Severity};
use forjar::core::types::{
    AuthzResult, CoverageLevel, CoverageReport, ForjarConfig, LogFilter, LogGcResult,
    LogTruncation, MutationOperator, MutationReport, MutationResult, MutationScore,
    OperatorIdentity, OperatorSource, PathPolicy, ProgressConfig, Resource, ResourceCoverage,
    ResourceType, RunLogPath, SecretConfig, SecretProvider, SecretRef, SecretScanFinding,
    SecretScanResult, StructuredLogOutput, VerbosityLevel,
};

// ── FJ-2300: Security Model ────────────────────────────────────────

#[test]
fn f_2300_1_path_policy_blocks_denied_paths() {
    let policy = PathPolicy {
        deny_paths: vec![
            "/etc/shadow".into(),
            "/etc/sudoers".into(),
            "/root/.ssh/*".into(),
        ],
    };
    // Falsifier: denied paths MUST be blocked
    assert!(policy.is_denied("/etc/shadow"));
    assert!(policy.is_denied("/etc/sudoers"));
    // Glob match
    assert!(policy.is_denied("/root/.ssh/authorized_keys"));
    assert!(policy.is_denied("/root/.ssh/id_rsa"));
    // Safe paths must NOT be blocked
    assert!(!policy.is_denied("/etc/nginx/nginx.conf"));
    assert!(!policy.is_denied("/var/log/syslog"));
}

#[test]
fn f_2300_2_empty_policy_denies_nothing() {
    let policy = PathPolicy::default();
    assert!(!policy.has_restrictions());
    assert!(!policy.is_denied("/etc/shadow"));
    assert!(!policy.is_denied("/anything"));
}

#[test]
fn f_2300_3_authz_allowed_vs_denied() {
    let allowed = AuthzResult::Allowed;
    assert!(allowed.is_allowed());
    assert_eq!(allowed.to_string(), "allowed");

    let denied = AuthzResult::Denied {
        operator: "eve".into(),
        machine: "production".into(),
    };
    assert!(!denied.is_allowed());
    let msg = denied.to_string();
    assert!(msg.contains("eve"));
    assert!(msg.contains("production"));
}

#[test]
fn f_2300_4_secret_provider_all_backends() {
    // All four provider types must exist and roundtrip
    for (provider, expected) in [
        (SecretProvider::Env, "env"),
        (SecretProvider::File, "file"),
        (SecretProvider::Sops, "sops"),
        (SecretProvider::Op, "op"),
    ] {
        assert_eq!(provider.to_string(), expected);
        let yaml = serde_yaml_ng::to_string(&provider).unwrap();
        let parsed: SecretProvider = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(provider, parsed);
    }
}

#[test]
fn f_2300_5_operator_identity_from_flag_overrides_env() {
    let from_flag = OperatorIdentity::resolve(Some("deploy-bot"));
    assert_eq!(from_flag.name, "deploy-bot");
    assert_eq!(from_flag.source, OperatorSource::CliFlag);

    let from_env = OperatorIdentity::resolve(None);
    assert!(from_env.name.contains('@'));
    assert_eq!(from_env.source, OperatorSource::Environment);
}

#[test]
fn f_2300_6_secret_scan_clean_when_no_findings() {
    let result = SecretScanResult::from_findings(vec![], 10);
    assert!(result.clean);
    assert_eq!(result.scanned_fields, 10);
}

#[test]
fn f_2300_7_secret_scan_dirty_when_findings_present() {
    let findings = vec![SecretScanFinding {
        resource_id: "db".into(),
        field: "content".into(),
        pattern: "password:".into(),
        preview: "password: ***".into(),
    }];
    let result = SecretScanResult::from_findings(findings, 5);
    assert!(!result.clean);
    assert_eq!(result.findings.len(), 1);
}

// ── FJ-1390: Security Scanner ──────────────────────────────────────

#[test]
fn f_1390_1_scanner_detects_hardcoded_secret() {
    let mut config = ForjarConfig::default();
    let mut r = Resource::default();
    r.resource_type = ResourceType::File;
    r.content = Some("database_password=supersecret123".into());
    config.resources.insert("db-config".into(), r);

    let findings = scan(&config);
    assert!(
        !findings.is_empty(),
        "scanner must detect hardcoded password"
    );
    assert!(findings.iter().any(|f| f.rule_id == "SS-1"));
    assert!(findings.iter().any(|f| f.severity == Severity::Critical));
}

#[test]
fn f_1390_2_scanner_detects_world_accessible_mode() {
    let mut config = ForjarConfig::default();
    let mut r = Resource::default();
    r.resource_type = ResourceType::File;
    r.mode = Some("0777".into());
    config.resources.insert("open-file".into(), r);

    let findings = scan(&config);
    assert!(
        findings.iter().any(|f| f.rule_id == "SS-3"),
        "scanner must detect world-accessible mode"
    );
}

#[test]
fn f_1390_3_scanner_detects_http_without_tls() {
    let mut config = ForjarConfig::default();
    let mut r = Resource::default();
    r.resource_type = ResourceType::File;
    r.source = Some("http://example.com/file.tar.gz".into());
    config.resources.insert("download".into(), r);

    let findings = scan(&config);
    assert!(
        findings.iter().any(|f| f.rule_id == "SS-2"),
        "scanner must detect HTTP without TLS"
    );
}

#[test]
fn f_1390_4_scanner_clean_on_safe_config() {
    let mut config = ForjarConfig::default();
    let mut r = Resource::default();
    r.resource_type = ResourceType::File;
    r.mode = Some("0600".into());
    r.content = Some("safe content here".into());
    config.resources.insert("safe-file".into(), r);

    let findings = scan(&config);
    // Must not flag SS-1, SS-2, SS-3, SS-8, SS-10
    assert!(
        !findings.iter().any(|f| f.rule_id == "SS-1"),
        "safe content must not trigger secret detection"
    );
    assert!(
        !findings.iter().any(|f| f.rule_id == "SS-3"),
        "mode 0600 must not trigger world-accessible"
    );
}

#[test]
fn f_1390_5_severity_counts_correct() {
    let findings = vec![
        forjar::core::security_scanner::SecurityFinding {
            rule_id: "SS-1".into(),
            category: "hard-coded-secret",
            severity: Severity::Critical,
            resource_id: "a".into(),
            message: "test".into(),
        },
        forjar::core::security_scanner::SecurityFinding {
            rule_id: "SS-2".into(),
            category: "http-without-tls",
            severity: Severity::High,
            resource_id: "b".into(),
            message: "test".into(),
        },
        forjar::core::security_scanner::SecurityFinding {
            rule_id: "SS-9".into(),
            category: "unrestricted-network",
            severity: Severity::Medium,
            resource_id: "c".into(),
            message: "test".into(),
        },
    ];
    let (c, h, m, l) = severity_counts(&findings);
    assert_eq!(c, 1);
    assert_eq!(h, 1);
    assert_eq!(m, 1);
    assert_eq!(l, 0);
}

#[test]
fn f_1390_6_scanner_detects_weak_crypto() {
    let mut config = ForjarConfig::default();
    let mut r = Resource::default();
    r.resource_type = ResourceType::File;
    r.content = Some("cipher: rc4\nprotocol: sslv3".into());
    config.resources.insert("crypto".into(), r);

    let findings = scan(&config);
    assert!(
        findings.iter().any(|f| f.rule_id == "SS-7"),
        "scanner must detect weak cryptography"
    );
}

// ── FJ-2301: Observability ─────────────────────────────────────────

#[test]
fn f_2301_1_verbosity_monotonic_ordering() {
    // Verbosity levels must be strictly ordered
    assert!(VerbosityLevel::Normal < VerbosityLevel::Verbose);
    assert!(VerbosityLevel::Verbose < VerbosityLevel::VeryVerbose);
    assert!(VerbosityLevel::VeryVerbose < VerbosityLevel::Trace);
}

#[test]
fn f_2301_2_verbosity_from_count_saturates() {
    assert_eq!(VerbosityLevel::from_count(0), VerbosityLevel::Normal);
    assert_eq!(VerbosityLevel::from_count(1), VerbosityLevel::Verbose);
    assert_eq!(VerbosityLevel::from_count(2), VerbosityLevel::VeryVerbose);
    assert_eq!(VerbosityLevel::from_count(3), VerbosityLevel::Trace);
    // Saturate at max
    assert_eq!(VerbosityLevel::from_count(100), VerbosityLevel::Trace);
}

#[test]
fn f_2301_3_streams_raw_only_at_trace() {
    assert!(!VerbosityLevel::Normal.streams_raw());
    assert!(!VerbosityLevel::Verbose.streams_raw());
    assert!(!VerbosityLevel::VeryVerbose.streams_raw());
    assert!(VerbosityLevel::Trace.streams_raw());
}

#[test]
fn f_2301_4_shows_scripts_at_very_verbose_and_above() {
    assert!(!VerbosityLevel::Normal.shows_scripts());
    assert!(!VerbosityLevel::Verbose.shows_scripts());
    assert!(VerbosityLevel::VeryVerbose.shows_scripts());
    assert!(VerbosityLevel::Trace.shows_scripts());
}

#[test]
fn f_2301_5_log_truncation_preserves_small_logs() {
    let trunc = LogTruncation {
        first_bytes: 100,
        last_bytes: 100,
    };
    let small = "small log content";
    assert!(!trunc.should_truncate(small.len()));
    assert_eq!(trunc.truncate(small), small);
}

#[test]
fn f_2301_6_log_truncation_marker_on_large_logs() {
    let trunc = LogTruncation {
        first_bytes: 5,
        last_bytes: 5,
    };
    let big = "XXXXX_middle_section_YYYYY";
    assert!(trunc.should_truncate(big.len()));
    let result = trunc.truncate(big);
    assert!(result.starts_with("XXXXX"));
    assert!(result.ends_with("YYYYY"));
    assert!(result.contains("TRUNCATED"));
}

#[test]
fn f_2301_7_log_filter_criteria_detection() {
    let empty = LogFilter::default();
    assert!(!empty.has_criteria());

    let machine = LogFilter::for_machine("web");
    assert!(machine.has_criteria());
    assert_eq!(machine.machine.as_deref(), Some("web"));

    let failures = LogFilter::failures();
    assert!(failures.has_criteria());
    assert!(failures.failures_only);
}

#[test]
fn f_2301_8_run_log_path_format() {
    let p = RunLogPath::new("state", "gpu-01", "r-abc123");
    assert_eq!(p.run_dir(), "state/gpu-01/runs/r-abc123");
    assert_eq!(
        p.resource_log("nginx-pkg", "apply"),
        "state/gpu-01/runs/r-abc123/nginx-pkg.apply.log"
    );
    assert_eq!(p.meta_path(), "state/gpu-01/runs/r-abc123/meta.yaml");
}

#[test]
fn f_2301_9_log_gc_result_mb_conversion() {
    let gc = LogGcResult {
        runs_removed: 10,
        bytes_freed: 100 * 1024 * 1024,
        runs_kept: 5,
    };
    assert!((gc.mb_freed() - 100.0).abs() < 0.01);
    let display = gc.to_string();
    assert!(display.contains("removed 10 runs"));
    assert!(display.contains("100.0 MB"));
}

#[test]
fn f_2301_10_structured_log_output_serde() {
    let out = StructuredLogOutput {
        run_id: "r-1".into(),
        machine: "web".into(),
        resource_id: "nginx".into(),
        log_path: "state/web/runs/r-1/nginx.apply.log".into(),
        exit_code: 0,
        duration_secs: 2.5,
        truncated: false,
    };
    let json = serde_json::to_string(&out).unwrap();
    let parsed: StructuredLogOutput = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.run_id, "r-1");
    assert_eq!(parsed.exit_code, 0);
    assert!(!parsed.truncated);
}

#[test]
fn f_2301_11_progress_config_defaults() {
    let pc = ProgressConfig::default();
    assert!(pc.show_progress);
    assert_eq!(pc.update_interval_ms, 100);
}

// ── FJ-2604: Mutation Testing Model ────────────────────────────────

#[test]
fn f_2604_1_mutation_score_zero_total_is_100_pct() {
    let score = MutationScore::default();
    assert!((score.score_pct() - 100.0).abs() < 0.01);
    assert_eq!(score.grade(), 'A');
}

#[test]
fn f_2604_2_mutation_score_grade_boundaries() {
    let make = |detected, total| MutationScore {
        total,
        detected,
        survived: total - detected,
        errored: 0,
    };
    assert_eq!(make(90, 100).grade(), 'A'); // 90%
    assert_eq!(make(89, 100).grade(), 'B'); // 89%
    assert_eq!(make(80, 100).grade(), 'B'); // 80%
    assert_eq!(make(79, 100).grade(), 'C'); // 79%
    assert_eq!(make(60, 100).grade(), 'C'); // 60%
    assert_eq!(make(59, 100).grade(), 'F'); // 59%
}

#[test]
fn f_2604_3_mutation_score_monotonic() {
    // Higher detection rate → higher or equal grade (A > B > C > F)
    let grade_rank = |g: char| -> u8 {
        match g {
            'A' => 4,
            'B' => 3,
            'C' => 2,
            'F' => 1,
            _ => 0,
        }
    };
    let ranks: Vec<u8> = (0..=100)
        .map(|d| {
            grade_rank(
                MutationScore {
                    total: 100,
                    detected: d,
                    survived: 100 - d,
                    errored: 0,
                }
                .grade(),
            )
        })
        .collect();
    for i in 1..ranks.len() {
        assert!(
            ranks[i] >= ranks[i - 1],
            "grade must be monotonically non-decreasing with detection rate"
        );
    }
}

#[test]
fn f_2604_4_mutation_operator_applicable_types() {
    // File operators apply to files
    assert!(MutationOperator::DeleteFile
        .applicable_types()
        .contains(&"file"));
    assert!(MutationOperator::ModifyContent
        .applicable_types()
        .contains(&"file"));
    // Service operators apply to services
    assert!(MutationOperator::StopService
        .applicable_types()
        .contains(&"service"));
    // Package operators apply to packages
    assert!(MutationOperator::RemovePackage
        .applicable_types()
        .contains(&"package"));
}

#[test]
fn f_2604_5_mutation_report_from_results() {
    let results = vec![
        MutationResult {
            resource_id: "config".into(),
            resource_type: "file".into(),
            operator: MutationOperator::DeleteFile,
            detected: true,
            reconverged: Some(true),
            duration_ms: 100,
            error: None,
        },
        MutationResult {
            resource_id: "config".into(),
            resource_type: "file".into(),
            operator: MutationOperator::ModifyContent,
            detected: false,
            reconverged: None,
            duration_ms: 50,
            error: None,
        },
    ];
    let report = MutationReport::from_results(results);
    assert_eq!(report.score.total, 2);
    assert_eq!(report.score.detected, 1);
    assert_eq!(report.score.survived, 1);
    assert_eq!(report.undetected.len(), 1);
}

// ── FJ-2605: Coverage Model ────────────────────────────────────────

#[test]
fn f_2605_1_coverage_levels_strictly_ordered() {
    assert!(CoverageLevel::L0 < CoverageLevel::L1);
    assert!(CoverageLevel::L1 < CoverageLevel::L2);
    assert!(CoverageLevel::L2 < CoverageLevel::L3);
    assert!(CoverageLevel::L3 < CoverageLevel::L4);
    assert!(CoverageLevel::L4 < CoverageLevel::L5);
}

#[test]
fn f_2605_2_coverage_report_threshold() {
    let entries = vec![
        ResourceCoverage {
            resource_id: "a".into(),
            level: CoverageLevel::L3,
            resource_type: "file".into(),
        },
        ResourceCoverage {
            resource_id: "b".into(),
            level: CoverageLevel::L1,
            resource_type: "package".into(),
        },
    ];
    let report = CoverageReport::from_entries(entries);
    assert_eq!(report.min_level, CoverageLevel::L1);
    assert!(report.meets_threshold(CoverageLevel::L1));
    assert!(!report.meets_threshold(CoverageLevel::L2));
}

#[test]
fn f_2605_3_coverage_report_histogram() {
    let entries = vec![
        ResourceCoverage {
            resource_id: "a".into(),
            level: CoverageLevel::L5,
            resource_type: "file".into(),
        },
        ResourceCoverage {
            resource_id: "b".into(),
            level: CoverageLevel::L5,
            resource_type: "file".into(),
        },
        ResourceCoverage {
            resource_id: "c".into(),
            level: CoverageLevel::L0,
            resource_type: "task".into(),
        },
    ];
    let report = CoverageReport::from_entries(entries);
    assert_eq!(report.histogram[0], 1); // L0
    assert_eq!(report.histogram[5], 2); // L5
    assert_eq!(report.min_level, CoverageLevel::L0);
}
