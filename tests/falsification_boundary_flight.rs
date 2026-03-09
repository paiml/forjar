//! FJ-3209/115/113: Policy boundary testing, flight-grade execution, Ferrocene.
//! Usage: cargo test --test falsification_boundary_flight

use forjar::core::compliance_pack::*;
use forjar::core::ferrocene::*;
use forjar::core::flight_grade::*;
use forjar::core::policy_boundary::*;

// ============================================================================
// Helpers
// ============================================================================

fn assert_pack() -> CompliancePack {
    CompliancePack {
        name: "test-assert".into(),
        version: "1.0".into(),
        framework: "internal".into(),
        description: None,
        rules: vec![ComplianceRule {
            id: "R-001".into(),
            title: "File mode".into(),
            description: None,
            severity: "error".into(),
            controls: vec![],
            check: ComplianceCheck::Assert {
                resource_type: "file".into(),
                field: "mode".into(),
                expected: "0644".into(),
            },
        }],
    }
}

fn deny_pack() -> CompliancePack {
    CompliancePack {
        name: "test-deny".into(),
        version: "1.0".into(),
        framework: "internal".into(),
        description: None,
        rules: vec![ComplianceRule {
            id: "D-001".into(),
            title: "No root password".into(),
            description: None,
            severity: "error".into(),
            controls: vec![],
            check: ComplianceCheck::Deny {
                resource_type: "user".into(),
                field: "password".into(),
                pattern: "root".into(),
            },
        }],
    }
}

fn require_pack() -> CompliancePack {
    CompliancePack {
        name: "test-require".into(),
        version: "1.0".into(),
        framework: "internal".into(),
        description: None,
        rules: vec![ComplianceRule {
            id: "Q-001".into(),
            title: "Owner required".into(),
            description: None,
            severity: "warning".into(),
            controls: vec![],
            check: ComplianceCheck::Require {
                resource_type: "file".into(),
                field: "owner".into(),
            },
        }],
    }
}

fn tag_pack() -> CompliancePack {
    CompliancePack {
        name: "test-tag".into(),
        version: "1.0".into(),
        framework: "internal".into(),
        description: None,
        rules: vec![ComplianceRule {
            id: "T-001".into(),
            title: "Env tag required".into(),
            description: None,
            severity: "warning".into(),
            controls: vec![],
            check: ComplianceCheck::RequireTag { tag: "env".into() },
        }],
    }
}

fn mixed_pack() -> CompliancePack {
    CompliancePack {
        name: "mixed".into(),
        version: "2.0".into(),
        framework: "cis".into(),
        description: Some("Mixed check types".into()),
        rules: vec![
            ComplianceRule {
                id: "M-001".into(),
                title: "Assert mode".into(),
                description: None,
                severity: "error".into(),
                controls: vec!["CIS-1.1".into()],
                check: ComplianceCheck::Assert {
                    resource_type: "file".into(),
                    field: "mode".into(),
                    expected: "0600".into(),
                },
            },
            ComplianceRule {
                id: "M-002".into(),
                title: "Deny root".into(),
                description: None,
                severity: "error".into(),
                controls: vec![],
                check: ComplianceCheck::Deny {
                    resource_type: "user".into(),
                    field: "shell".into(),
                    pattern: "/bin/bash".into(),
                },
            },
            ComplianceRule {
                id: "M-003".into(),
                title: "Script check".into(),
                description: None,
                severity: "info".into(),
                controls: vec![],
                check: ComplianceCheck::Script {
                    script: "echo ok".into(),
                },
            },
        ],
    }
}

// ============================================================================
// FJ-3209: generate_boundary_configs
// ============================================================================

#[test]
fn boundary_assert_generates_two() {
    let configs = generate_boundary_configs(&assert_pack());
    assert_eq!(configs.len(), 2);
    assert!(configs.iter().any(|c| c.expected_pass));
    assert!(configs.iter().any(|c| !c.expected_pass));
}

#[test]
fn boundary_deny_generates_two() {
    let configs = generate_boundary_configs(&deny_pack());
    assert_eq!(configs.len(), 2);
}

#[test]
fn boundary_require_generates_two() {
    let configs = generate_boundary_configs(&require_pack());
    assert_eq!(configs.len(), 2);
}

#[test]
fn boundary_tag_generates_two() {
    let configs = generate_boundary_configs(&tag_pack());
    assert_eq!(configs.len(), 2);
}

#[test]
fn boundary_script_generates_none() {
    let pack = CompliancePack {
        name: "script-only".into(),
        version: "1.0".into(),
        framework: "custom".into(),
        description: None,
        rules: vec![ComplianceRule {
            id: "S-001".into(),
            title: "Script".into(),
            description: None,
            severity: "info".into(),
            controls: vec![],
            check: ComplianceCheck::Script {
                script: "echo ok".into(),
            },
        }],
    };
    let configs = generate_boundary_configs(&pack);
    assert!(configs.is_empty());
}

#[test]
fn boundary_mixed_pack_counts() {
    let configs = generate_boundary_configs(&mixed_pack());
    // 2 rules with boundaries (assert + deny), 1 script (no boundary)
    assert_eq!(configs.len(), 4);
}

#[test]
fn boundary_config_has_target_rule_id() {
    let configs = generate_boundary_configs(&assert_pack());
    for c in &configs {
        assert_eq!(c.target_rule_id, "R-001");
    }
}

#[test]
fn boundary_config_golden_has_correct_field() {
    let configs = generate_boundary_configs(&assert_pack());
    let golden = configs.iter().find(|c| c.expected_pass).unwrap();
    let fields = &golden.resources["boundary-file"];
    assert_eq!(fields["mode"], "0644");
}

// ============================================================================
// FJ-3209: test_boundaries
// ============================================================================

#[test]
fn test_boundaries_assert_pack() {
    let result = test_boundaries(&assert_pack());
    assert_eq!(result.pack_name, "test-assert");
    assert_eq!(result.rules_tested, 1);
    assert!(result.outcomes.len() >= 2);
}

#[test]
fn test_boundaries_mixed_pack() {
    let result = test_boundaries(&mixed_pack());
    assert_eq!(result.pack_name, "mixed");
    assert!(result.rules_with_boundary >= 2);
}

#[test]
fn boundary_result_all_passed_empty() {
    let result = BoundaryTestResult {
        pack_name: "empty".into(),
        rules_tested: 0,
        rules_with_boundary: 0,
        outcomes: vec![],
    };
    assert!(result.all_passed());
    assert_eq!(result.failure_count(), 0);
}

#[test]
fn boundary_result_with_failure() {
    let result = BoundaryTestResult {
        pack_name: "test".into(),
        rules_tested: 1,
        rules_with_boundary: 1,
        outcomes: vec![BoundaryOutcome {
            rule_id: "R-1".into(),
            passed: false,
            expected: "pass".into(),
            actual: "fail".into(),
            description: "test".into(),
        }],
    };
    assert!(!result.all_passed());
    assert_eq!(result.failure_count(), 1);
}

// ============================================================================
// FJ-3209: format_boundary_results
// ============================================================================

#[test]
fn format_boundary_contains_pack_name() {
    let result = test_boundaries(&assert_pack());
    let text = format_boundary_results(&result);
    assert!(text.contains("test-assert"));
    assert!(text.contains("Result:"));
}

#[test]
fn format_boundary_shows_pass_fail() {
    let result = test_boundaries(&deny_pack());
    let text = format_boundary_results(&result);
    assert!(text.contains("PASS") || text.contains("FAIL"));
}

// ============================================================================
// FJ-115: Flight-grade compliance
// ============================================================================

#[test]
fn fg_compliance_within_limits() {
    let report = check_compliance(100, 10);
    assert!(report.compliant);
    assert!(report.no_dynamic_alloc);
    assert!(report.bounded_loops);
    assert!(report.no_panic_paths);
    assert!(report.deterministic_memory);
    assert_eq!(report.max_resources, MAX_RESOURCES);
}

#[test]
fn fg_compliance_exceeds_resources() {
    let report = check_compliance(MAX_RESOURCES + 1, 10);
    assert!(!report.compliant);
    assert!(!report.deterministic_memory);
}

#[test]
fn fg_compliance_exceeds_depth() {
    let report = check_compliance(10, MAX_DEPTH + 1);
    assert!(!report.compliant);
}

#[test]
fn fg_compliance_at_exact_limits() {
    let report = check_compliance(MAX_RESOURCES, MAX_DEPTH);
    assert!(report.compliant);
}

#[test]
fn fg_compliance_serde() {
    let report = check_compliance(10, 5);
    let json = serde_json::to_string(&report).unwrap();
    assert!(json.contains("\"compliant\":true"));
}

// ============================================================================
// FJ-115: Topological sort
// ============================================================================

#[test]
fn fg_topo_empty() {
    let mut plan = FgPlan::empty();
    assert!(fg_topo_sort(&mut plan).is_ok());
    assert_eq!(plan.order_len, 0);
}

#[test]
fn fg_topo_single() {
    let mut plan = FgPlan::empty();
    plan.resources[0].id = 0;
    plan.count = 1;
    assert!(fg_topo_sort(&mut plan).is_ok());
    assert_eq!(plan.order_len, 1);
    assert_eq!(plan.order[0], 0);
}

#[test]
fn fg_topo_chain_two() {
    let mut plan = FgPlan::empty();
    plan.resources[0].id = 0;
    plan.resources[1].id = 1;
    plan.resources[1].deps[0] = 0;
    plan.resources[1].dep_count = 1;
    plan.count = 2;
    assert!(fg_topo_sort(&mut plan).is_ok());
    assert_eq!(plan.order_len, 2);
    assert_eq!(plan.order[0], 0); // dependency first
    assert_eq!(plan.order[1], 1);
}

#[test]
fn fg_topo_diamond() {
    let mut plan = FgPlan::empty();
    // A(0) → B(1), A(0) → C(2), B+C → D(3)
    plan.resources[0].id = 0;
    plan.resources[1].id = 1;
    plan.resources[1].deps[0] = 0;
    plan.resources[1].dep_count = 1;
    plan.resources[2].id = 2;
    plan.resources[2].deps[0] = 0;
    plan.resources[2].dep_count = 1;
    plan.resources[3].id = 3;
    plan.resources[3].deps[0] = 1;
    plan.resources[3].deps[1] = 2;
    plan.resources[3].dep_count = 2;
    plan.count = 4;
    assert!(fg_topo_sort(&mut plan).is_ok());
    assert_eq!(plan.order_len, 4);
    assert_eq!(plan.order[0], 0); // A must be first
}

#[test]
fn fg_resource_empty_defaults() {
    let r = FgResource::empty();
    assert_eq!(r.status, FgStatus::Pending);
    assert_eq!(r.dep_count, 0);
    assert_eq!(r.hash, [0u8; 32]);
}

#[test]
fn fg_status_equality() {
    assert_eq!(FgStatus::Converged, FgStatus::Converged);
    assert_ne!(FgStatus::Failed, FgStatus::Skipped);
}

// ============================================================================
// FJ-113: Ferrocene source compliance
// ============================================================================

#[test]
fn ferrocene_clean_source() {
    let source = "fn main() {\n    println!(\"safe\");\n}";
    let violations = check_source_compliance(source);
    assert!(violations.is_empty());
}

#[test]
fn ferrocene_forbidden_attr() {
    let attr = concat!("#![allow(", "unsafe_code", ")]");
    let source = format!("{attr}\nfn main() {{}}\n");
    let violations = check_source_compliance(&source);
    assert!(!violations.is_empty());
    assert_eq!(violations[0].severity, ViolationSeverity::Error);
    assert_eq!(violations[0].line, 1);
}

#[test]
fn ferrocene_unsafe_block() {
    let source = concat!("fn main() {\n    ", "unsa", "fe { };\n}\n");
    let violations = check_source_compliance(source);
    assert!(!violations.is_empty());
}

#[test]
fn ferrocene_feature_gate() {
    let source = "#![feature(nightly_only)]\nfn main() {}";
    let violations = check_source_compliance(source);
    assert!(!violations.is_empty());
    assert!(violations[0].message.contains("Forbidden attribute"));
}

#[test]
fn ferrocene_safety_standard_serde() {
    for std in [
        SafetyStandard::Iso26262,
        SafetyStandard::Do178c,
        SafetyStandard::Iec61508,
        SafetyStandard::En50128,
    ] {
        let json = serde_json::to_string(&std).unwrap();
        let parsed: SafetyStandard = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, std);
    }
}

#[test]
fn ferrocene_asil_levels() {
    assert_ne!(AsilLevel::QM, AsilLevel::D);
    assert_eq!(AsilLevel::A, AsilLevel::A);
}

#[test]
fn ferrocene_dal_levels() {
    assert_ne!(DalLevel::E, DalLevel::A);
    let json = serde_json::to_string(&DalLevel::B).unwrap();
    let parsed: DalLevel = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, DalLevel::B);
}

#[test]
fn ferrocene_ci_config_content() {
    let cfg = ferrocene_ci_config();
    assert!(cfg.contains("ferrocene"));
    assert!(cfg.contains("panic=abort"));
    assert!(cfg.contains("overflow-checks=on"));
}

#[test]
fn ferrocene_evidence_generation() {
    let ev = generate_evidence(SafetyStandard::Iso26262, "binhash", "srchash");
    assert_eq!(ev.standard, SafetyStandard::Iso26262);
    assert_eq!(ev.binary_hash, "binhash");
    assert_eq!(ev.source_hash, "srchash");
    assert!(!ev.build_flags.is_empty());
    assert!(!ev.forbidden_features.is_empty());
}

#[test]
fn ferrocene_evidence_serde() {
    let ev = generate_evidence(SafetyStandard::Do178c, "h1", "h2");
    let json = serde_json::to_string(&ev).unwrap();
    assert!(json.contains("Do178c"));
}

#[test]
fn ferrocene_detect_toolchain() {
    let info = detect_toolchain();
    assert!(!info.is_ferrocene); // CI runs standard rustc
    assert!(!info.channel.is_empty());
}

#[test]
fn ferrocene_violation_severity() {
    let v = ComplianceViolation {
        line: 1,
        message: "test".into(),
        severity: ViolationSeverity::Warning,
    };
    let json = serde_json::to_string(&v).unwrap();
    assert!(json.contains("Warning"));
}
