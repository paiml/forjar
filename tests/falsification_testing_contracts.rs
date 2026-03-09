//! FJ-2200/2602/2604/3000-3040: Popperian falsification for testing strategy,
//! design by contract, and defect analysis.
//!
//! Each test states conditions under which the testing framework or
//! defect detection would be rejected as invalid.
#![allow(clippy::field_reassign_with_default)]

use forjar::core::types::{
    BehaviorEntry, BehaviorReport, BehaviorResult, BehaviorSpec, ContractAssertion, ContractKind,
    ConvergenceAssert, HandlerAuditReport, HandlerExemption, HashInvariantCheck, KaniHarness,
    MutationOperator, ProofStatus, VerifyCommand,
};

// ── FJ-2200: Design by Contract ────────────────────────────────────

#[test]
fn f_2200_1_hash_invariant_pass_has_matching_hashes() {
    let check = HashInvariantCheck::pass("nginx-pkg", "package", "blake3:abc123");
    assert!(check.passed);
    assert_eq!(check.expected_hash, check.actual_hash);
    assert!(check.deviation_reason.is_none());
}

#[test]
fn f_2200_2_hash_invariant_fail_records_deviation() {
    let check = HashInvariantCheck::fail(
        "cron-job",
        "cron",
        "blake3:expected",
        "blake3:actual",
        "cron uses schedule hash only",
    );
    assert!(!check.passed);
    assert_ne!(check.expected_hash, check.actual_hash);
    assert_eq!(
        check.deviation_reason.as_deref(),
        Some("cron uses schedule hash only")
    );
}

#[test]
fn f_2200_3_handler_audit_report_counts() {
    let report = HandlerAuditReport {
        checks: vec![
            HashInvariantCheck::pass("file", "file", "h1"),
            HashInvariantCheck::pass("pkg", "package", "h2"),
            HashInvariantCheck::fail("cron", "cron", "h3", "h4", "schedule"),
        ],
        exemptions: vec![HandlerExemption {
            handler: "task".into(),
            reason: "imperative by nature".into(),
            approved_by: Some("spec review".into()),
        }],
    };
    assert_eq!(report.pass_count(), 2);
    assert_eq!(report.fail_count(), 1);
    assert!(!report.all_passed());
    let formatted = report.format_report();
    assert!(formatted.contains("2 passed"));
    assert!(formatted.contains("1 failed"));
    assert!(formatted.contains("[EXEMPT] task"));
}

#[test]
fn f_2200_4_contract_assertion_kinds() {
    for (kind, label) in [
        (ContractKind::Requires, "requires"),
        (ContractKind::Ensures, "ensures"),
        (ContractKind::Invariant, "invariant"),
    ] {
        assert_eq!(kind.to_string(), label);
    }
}

#[test]
fn f_2200_5_contract_assertion_serde_roundtrip() {
    let assertion = ContractAssertion {
        function: "determine_action".into(),
        module: "core::planner".into(),
        kind: ContractKind::Ensures,
        held: true,
        expression: Some("result.is_noop() || result.is_apply()".into()),
    };
    let json = serde_json::to_string(&assertion).unwrap();
    let parsed: ContractAssertion = serde_json::from_str(&json).unwrap();
    assert!(parsed.held);
    assert_eq!(parsed.kind, ContractKind::Ensures);
    assert!(parsed.expression.unwrap().contains("is_noop"));
}

// ── FJ-2201: Kani Proof Metadata ───────────────────────────────────

#[test]
fn f_2201_1_kani_harness_metadata() {
    let harness = KaniHarness {
        name: "proof_mutation_grade_monotonic".into(),
        property: "higher score → higher/equal grade".into(),
        target_function: "MutationScore::grade".into(),
        status: ProofStatus::Verified,
        bound: Some(100),
    };
    assert_eq!(harness.status, ProofStatus::Verified);
    assert_eq!(harness.bound, Some(100));
}

#[test]
fn f_2201_2_proof_status_all_variants() {
    for (status, label) in [
        (ProofStatus::Verified, "verified"),
        (ProofStatus::Failed, "failed"),
        (ProofStatus::Pending, "pending"),
        (ProofStatus::Timeout, "timeout"),
        (ProofStatus::Deprecated, "deprecated"),
    ] {
        assert_eq!(status.to_string(), label);
    }
}

// ── FJ-2602: Behavior-Driven Infrastructure Specs ──────────────────

#[test]
fn f_2602_1_behavior_spec_yaml_roundtrip() {
    let yaml = r#"
name: nginx web server
config: examples/nginx.yaml
machine: web-1
behaviors:
  - name: nginx is installed
    resource: nginx-pkg
    state: present
    verify:
      command: "dpkg -l nginx | grep -q '^ii'"
      exit_code: 0
  - name: idempotent apply
    type: convergence
    convergence:
      second_apply: noop
      state_unchanged: true
"#;
    let spec: BehaviorSpec = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(spec.name, "nginx web server");
    assert_eq!(spec.machine.as_deref(), Some("web-1"));
    assert_eq!(spec.behavior_count(), 2);

    // First entry: resource assertion
    assert!(!spec.behaviors[0].is_convergence());
    assert!(spec.behaviors[0].has_verify());
    assert_eq!(spec.behaviors[0].assert_state.as_deref(), Some("present"));

    // Second entry: convergence assertion
    assert!(spec.behaviors[1].is_convergence());
    assert!(!spec.behaviors[1].has_verify());
}

#[test]
fn f_2602_2_behavior_spec_referenced_resources() {
    let spec = BehaviorSpec {
        name: "test".into(),
        config: "c.yaml".into(),
        machine: None,
        behaviors: vec![
            BehaviorEntry {
                name: "pkg check".into(),
                resource: Some("nginx-pkg".into()),
                behavior_type: None,
                assert_state: Some("present".into()),
                verify: None,
                convergence: None,
            },
            BehaviorEntry {
                name: "convergence".into(),
                resource: None,
                behavior_type: Some("convergence".into()),
                assert_state: None,
                verify: None,
                convergence: Some(ConvergenceAssert::default()),
            },
            BehaviorEntry {
                name: "svc check".into(),
                resource: Some("nginx-svc".into()),
                behavior_type: None,
                assert_state: Some("running".into()),
                verify: None,
                convergence: None,
            },
        ],
    };
    let refs = spec.referenced_resources();
    assert_eq!(refs, vec!["nginx-pkg", "nginx-svc"]);
}

#[test]
fn f_2602_3_verify_command_all_fields() {
    let yaml = r#"
command: "curl -sf http://localhost:8080/health"
exit_code: 0
stdout: "ok"
stderr_contains: "warn"
file_exists: "/tmp/marker"
file_content: "blake3:abc123"
port_open: 8080
retries: 5
retry_delay_secs: 2
"#;
    let vc: VerifyCommand = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(vc.exit_code, Some(0));
    assert_eq!(vc.stdout.as_deref(), Some("ok"));
    assert_eq!(vc.port_open, Some(8080));
    assert_eq!(vc.retries, Some(5));
    assert_eq!(vc.retry_delay_secs, Some(2));
    assert_eq!(vc.file_exists.as_deref(), Some("/tmp/marker"));
}

#[test]
fn f_2602_4_behavior_report_all_pass() {
    let results = vec![
        BehaviorResult {
            name: "pkg installed".into(),
            passed: true,
            failure: None,
            actual_exit_code: Some(0),
            actual_stdout: None,
            duration_ms: 50,
        },
        BehaviorResult {
            name: "svc running".into(),
            passed: true,
            failure: None,
            actual_exit_code: Some(0),
            actual_stdout: None,
            duration_ms: 30,
        },
    ];
    let report = BehaviorReport::from_results("nginx".into(), results);
    assert!(report.all_passed());
    assert_eq!(report.total, 2);
    assert_eq!(report.passed, 2);
    assert_eq!(report.failed, 0);
}

#[test]
fn f_2602_5_behavior_report_with_failures() {
    let results = vec![
        BehaviorResult {
            name: "installed".into(),
            passed: true,
            failure: None,
            actual_exit_code: Some(0),
            actual_stdout: None,
            duration_ms: 50,
        },
        BehaviorResult {
            name: "running".into(),
            passed: false,
            failure: Some("exit code 1, expected 0".into()),
            actual_exit_code: Some(1),
            actual_stdout: Some("inactive".into()),
            duration_ms: 100,
        },
    ];
    let report = BehaviorReport::from_results("nginx".into(), results);
    assert!(!report.all_passed());
    assert_eq!(report.passed, 1);
    assert_eq!(report.failed, 1);

    let summary = report.format_summary();
    assert!(summary.contains("[PASS] installed"));
    assert!(summary.contains("[FAIL] running"));
    assert!(summary.contains("exit code 1"));
    assert!(summary.contains("1/2 passed"));
}

// ── FJ-2604: Mutation Operator Applicability ───────────────────────

#[test]
fn f_2604_6_all_mutation_operators_have_descriptions() {
    let operators = [
        MutationOperator::DeleteFile,
        MutationOperator::ModifyContent,
        MutationOperator::ChangePermissions,
        MutationOperator::StopService,
        MutationOperator::RemovePackage,
        MutationOperator::KillProcess,
        MutationOperator::UnmountFilesystem,
        MutationOperator::CorruptConfig,
    ];
    for op in &operators {
        assert!(
            !op.description().is_empty(),
            "operator {} must have a description",
            op
        );
        assert!(
            !op.applicable_types().is_empty(),
            "operator {} must apply to at least one type",
            op
        );
    }
}

#[test]
fn f_2604_7_file_operators_only_apply_to_files() {
    let file_ops = [
        MutationOperator::DeleteFile,
        MutationOperator::ModifyContent,
        MutationOperator::ChangePermissions,
        MutationOperator::CorruptConfig,
    ];
    for op in &file_ops {
        assert!(
            op.applicable_types().contains(&"file"),
            "{} must apply to file type",
            op
        );
    }
}

#[test]
fn f_2604_8_service_operators_apply_to_services() {
    assert!(MutationOperator::StopService
        .applicable_types()
        .contains(&"service"));
    assert!(MutationOperator::KillProcess
        .applicable_types()
        .contains(&"service"));
}

// ── FJ-3000: Exit Code Safety Lint ─────────────────────────────────

#[test]
fn f_3000_1_semicolon_chain_detected() {
    use forjar::core::types::{ForjarConfig, Resource, ResourceType};

    let mut config = ForjarConfig::default();
    let mut task = Resource::default();
    task.resource_type = ResourceType::Task;
    task.command = Some("cd /app ; make install".into());
    config.resources.insert("build".into(), task);

    let warnings = forjar::cli::lint::lint_semicolon_chains(&config);
    assert!(
        !warnings.is_empty(),
        "must detect semicolon chain in task command"
    );
    assert!(warnings[0].contains("';'"));
}

#[test]
fn f_3000_2_semicolon_in_quotes_not_flagged() {
    use forjar::cli::lint::has_bare_semicolon;

    // Bare semicolon should be detected
    assert!(has_bare_semicolon("cmd1 ; cmd2"));
    // Semicolon inside single quotes should not
    assert!(!has_bare_semicolon("echo 'a;b'"));
    // Semicolon inside double quotes should not
    assert!(!has_bare_semicolon("echo \"a;b\""));
    // No semicolon at all
    assert!(!has_bare_semicolon("cmd1 && cmd2"));
}

#[test]
fn f_3000_3_multiline_commands_not_flagged() {
    use forjar::core::types::{ForjarConfig, Resource, ResourceType};

    let mut config = ForjarConfig::default();
    let mut task = Resource::default();
    task.resource_type = ResourceType::Task;
    task.command = Some("cd /app\nmake install".into());
    config.resources.insert("build".into(), task);

    let warnings = forjar::cli::lint::lint_semicolon_chains(&config);
    assert!(
        warnings.is_empty(),
        "multiline commands should not trigger semicolon lint"
    );
}

// ── FJ-3030: Nohup LD_LIBRARY_PATH Lint ───────────────────────────

#[test]
fn f_3030_1_nohup_without_ld_library_path_flagged() {
    use forjar::core::types::{ForjarConfig, Resource, ResourceType};

    let mut config = ForjarConfig::default();
    let mut task = Resource::default();
    task.resource_type = ResourceType::Task;
    task.command = Some("nohup /opt/cuda/bin/train &".into());
    config.resources.insert("train".into(), task);

    let warnings = forjar::cli::lint::lint_nohup_ld_path(&config);
    assert!(
        !warnings.is_empty(),
        "must flag nohup with absolute binary without LD_LIBRARY_PATH"
    );
}

#[test]
fn f_3030_2_nohup_with_ld_library_path_not_flagged() {
    use forjar::core::types::{ForjarConfig, Resource, ResourceType};

    let mut config = ForjarConfig::default();
    let mut task = Resource::default();
    task.resource_type = ResourceType::Task;
    task.command = Some("LD_LIBRARY_PATH=/opt/cuda/lib nohup /opt/cuda/bin/train &".into());
    config.resources.insert("train".into(), task);

    let warnings = forjar::cli::lint::lint_nohup_ld_path(&config);
    assert!(
        warnings.is_empty(),
        "nohup with LD_LIBRARY_PATH set should not be flagged"
    );
}

// ── FJ-3040: Nohup Sleep Health Check Anti-Pattern ─────────────────

#[test]
fn f_3040_1_nohup_sleep_curl_flagged() {
    use forjar::core::types::{ForjarConfig, Resource, ResourceType};

    let mut config = ForjarConfig::default();
    let mut task = Resource::default();
    task.resource_type = ResourceType::Task;
    task.command =
        Some("nohup /opt/bin/server & sleep 10; curl -sf http://localhost:8080/health".into());
    config.resources.insert("server".into(), task);

    let warnings = forjar::cli::lint::lint_nohup_sleep_health(&config);
    assert!(
        !warnings.is_empty(),
        "must flag nohup + sleep + health probe pattern"
    );
    assert!(warnings[0].contains("fragile"));
}

#[test]
fn f_3040_2_nohup_without_sleep_not_flagged() {
    use forjar::core::types::{ForjarConfig, Resource, ResourceType};

    let mut config = ForjarConfig::default();
    let mut task = Resource::default();
    task.resource_type = ResourceType::Task;
    task.command = Some("nohup /opt/bin/server &".into());
    config.resources.insert("server".into(), task);

    let warnings = forjar::cli::lint::lint_nohup_sleep_health(&config);
    assert!(
        warnings.is_empty(),
        "nohup without sleep+health should not be flagged"
    );
}

// ── Cross-cutting: Serde Roundtrips ────────────────────────────────

#[test]
fn f_cross_1_behavior_spec_json_roundtrip() {
    let spec = BehaviorSpec {
        name: "test spec".into(),
        config: "config.yaml".into(),
        machine: Some("web-01".into()),
        behaviors: vec![BehaviorEntry {
            name: "check".into(),
            resource: Some("pkg".into()),
            behavior_type: None,
            assert_state: Some("present".into()),
            verify: Some(VerifyCommand {
                command: "dpkg -l".into(),
                exit_code: Some(0),
                ..Default::default()
            }),
            convergence: None,
        }],
    };
    let json = serde_json::to_string(&spec).unwrap();
    let parsed: BehaviorSpec = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.name, "test spec");
    assert_eq!(parsed.behaviors.len(), 1);
}

#[test]
fn f_cross_2_kani_harness_json_roundtrip() {
    let harness = KaniHarness {
        name: "proof_convergence".into(),
        property: "apply is idempotent".into(),
        target_function: "reconcile".into(),
        status: ProofStatus::Verified,
        bound: Some(256),
    };
    let json = serde_json::to_string(&harness).unwrap();
    let parsed: KaniHarness = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.status, ProofStatus::Verified);
    assert_eq!(parsed.bound, Some(256));
}
