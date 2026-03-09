//! FJ-1387/2702: Compliance benchmarks and quality gate evaluation.
//!
//! Popperian rejection criteria for:
//! - FJ-1387: evaluate_benchmark (CIS, NIST-800-53, SOC2, HIPAA, unknown),
//!   supported_benchmarks, count_by_severity, per-rule triggering
//! - FJ-2702: evaluate_gate (exit code, JSON parse, field, threshold, min,
//!   regex), GateAction (block/warn/skip), gpu_env_vars
//!
//! Usage: cargo test --test falsification_compliance_gate

use forjar::core::compliance::{
    self, evaluate_benchmark, supported_benchmarks, ComplianceFinding, FindingSeverity,
};
use forjar::core::task::{evaluate_gate, gpu_env_vars, GateAction, GateResult};
use forjar::core::types::*;
use indexmap::IndexMap;

// ── Config builder ──

fn make_config(resources: Vec<(&str, Resource)>) -> ForjarConfig {
    let mut res = IndexMap::new();
    for (id, r) in resources {
        res.insert(id.to_string(), r);
    }
    ForjarConfig {
        version: "1.0".into(),
        name: "test".into(),
        resources: res,
        description: None,
        params: Default::default(),
        machines: Default::default(),
        policy: Default::default(),
        outputs: Default::default(),
        policies: Default::default(),
        data: Default::default(),
        includes: Default::default(),
        include_provenance: Default::default(),
        checks: Default::default(),
        moved: Default::default(),
        secrets: Default::default(),
        environments: Default::default(),
    }
}

fn make_resource(rtype: ResourceType) -> Resource {
    Resource {
        resource_type: rtype,
        ..Default::default()
    }
}

// ============================================================================
// FJ-1387: supported_benchmarks
// ============================================================================

#[test]
fn benchmarks_list() {
    let b = supported_benchmarks();
    assert!(b.contains(&"cis"));
    assert!(b.contains(&"nist-800-53"));
    assert!(b.contains(&"soc2"));
    assert!(b.contains(&"hipaa"));
    assert_eq!(b.len(), 4);
}

// ============================================================================
// FJ-1387: CIS benchmark rules
// ============================================================================

#[test]
fn cis_world_writable() {
    let mut r = make_resource(ResourceType::File);
    r.mode = Some("0777".into());
    let config = make_config(vec![("web", r)]);
    let findings = evaluate_benchmark("cis", &config);
    assert!(findings.iter().any(|f| f.rule_id == "CIS-6.1.1"));
}

#[test]
fn cis_root_tmp() {
    let mut r = make_resource(ResourceType::File);
    r.path = Some("/tmp/script.sh".into());
    r.owner = Some("root".into());
    let config = make_config(vec![("tmp", r)]);
    let findings = evaluate_benchmark("cis", &config);
    assert!(findings.iter().any(|f| f.rule_id == "CIS-1.1.5"));
}

#[test]
fn cis_service_no_restart() {
    let r = make_resource(ResourceType::Service);
    let config = make_config(vec![("svc", r)]);
    let findings = evaluate_benchmark("cis", &config);
    assert!(findings.iter().any(|f| f.rule_id == "CIS-5.2.1"));
}

#[test]
fn cis_package_no_version() {
    let r = make_resource(ResourceType::Package);
    let config = make_config(vec![("pkg", r)]);
    let findings = evaluate_benchmark("cis", &config);
    assert!(findings.iter().any(|f| f.rule_id == "CIS-6.2.1"));
}

#[test]
fn cis_clean_config() {
    let mut r = make_resource(ResourceType::File);
    r.mode = Some("0600".into());
    r.owner = Some("app".into());
    r.path = Some("/etc/app.conf".into());
    let config = make_config(vec![("f", r)]);
    let findings = evaluate_benchmark("cis", &config);
    assert!(findings.is_empty());
}

// ============================================================================
// FJ-1387: NIST 800-53 benchmark rules
// ============================================================================

#[test]
fn nist_ac3_missing_owner() {
    let mut r = make_resource(ResourceType::File);
    r.path = Some("/etc/app.conf".into());
    let config = make_config(vec![("f", r)]);
    let findings = evaluate_benchmark("nist-800-53", &config);
    assert!(findings.iter().any(|f| f.rule_id == "NIST-AC-3.1"));
}

#[test]
fn nist_ac3_missing_mode() {
    let mut r = make_resource(ResourceType::File);
    r.owner = Some("root".into());
    let config = make_config(vec![("f", r)]);
    let findings = evaluate_benchmark("nist-800-53", &config);
    assert!(findings.iter().any(|f| f.rule_id == "NIST-AC-3.2"));
}

#[test]
fn nist_ac6_root_service() {
    let mut r = make_resource(ResourceType::Service);
    r.owner = Some("root".into());
    let config = make_config(vec![("svc", r)]);
    let findings = evaluate_benchmark("nist-800-53", &config);
    assert!(findings.iter().any(|f| f.rule_id == "NIST-AC-6"));
}

#[test]
fn nist_cm6_docker_no_ports() {
    let r = make_resource(ResourceType::Docker);
    let config = make_config(vec![("app", r)]);
    let findings = evaluate_benchmark("nist-800-53", &config);
    assert!(findings.iter().any(|f| f.rule_id == "NIST-CM-6"));
}

#[test]
fn nist_sc28_sensitive_path_no_mode() {
    let mut r = make_resource(ResourceType::File);
    r.path = Some("/etc/ssh/sshd_config".into());
    let config = make_config(vec![("ssh", r)]);
    let findings = evaluate_benchmark("nist-800-53", &config);
    assert!(findings.iter().any(|f| f.rule_id == "NIST-SC-28"));
}

#[test]
fn nist_si7_external_source_no_check() {
    let mut r = make_resource(ResourceType::File);
    r.source = Some("https://releases.example.com/bin".into());
    let config = make_config(vec![("dl", r)]);
    let findings = evaluate_benchmark("nist-800-53", &config);
    assert!(findings.iter().any(|f| f.rule_id == "NIST-SI-7"));
}

#[test]
fn nist_si7_with_check_clean() {
    let mut r = make_resource(ResourceType::File);
    r.source = Some("https://releases.example.com/bin".into());
    let mut config = make_config(vec![("dl", r)]);
    config.checks.insert(
        "dl".into(),
        CheckBlock {
            machine: "local".into(),
            command: "sha256sum --check".into(),
            expect_exit: None,
            description: None,
        },
    );
    let findings = evaluate_benchmark("nist-800-53", &config);
    assert!(!findings.iter().any(|f| f.rule_id == "NIST-SI-7"));
}

// ============================================================================
// FJ-1387: SOC2 benchmark rules
// ============================================================================

#[test]
fn soc2_file_no_owner() {
    let r = make_resource(ResourceType::File);
    let config = make_config(vec![("f", r)]);
    let findings = evaluate_benchmark("soc2", &config);
    assert!(findings.iter().any(|f| f.rule_id == "SOC2-CC6.1"));
}

#[test]
fn soc2_service_no_restart_on() {
    let r = make_resource(ResourceType::Service);
    let config = make_config(vec![("svc", r)]);
    let findings = evaluate_benchmark("soc2", &config);
    assert!(findings.iter().any(|f| f.rule_id == "SOC2-CC7.2"));
}

// ============================================================================
// FJ-1387: HIPAA benchmark rules
// ============================================================================

#[test]
fn hipaa_other_access() {
    let mut r = make_resource(ResourceType::File);
    r.mode = Some("0644".into());
    let config = make_config(vec![("f", r)]);
    let findings = evaluate_benchmark("hipaa", &config);
    assert!(findings.iter().any(|f| f.rule_id == "HIPAA-164.312a"));
}

#[test]
fn hipaa_unencrypted_port() {
    let mut r = make_resource(ResourceType::Network);
    r.port = Some("80".into());
    let config = make_config(vec![("web", r)]);
    let findings = evaluate_benchmark("hipaa", &config);
    assert!(findings
        .iter()
        .any(|f| f.rule_id == "HIPAA-164.312e" && f.severity == FindingSeverity::Critical));
}

#[test]
fn hipaa_clean_config() {
    let mut r = make_resource(ResourceType::File);
    r.mode = Some("0600".into());
    let config = make_config(vec![("f", r)]);
    let findings = evaluate_benchmark("hipaa", &config);
    assert!(findings.is_empty());
}

// ============================================================================
// FJ-1387: Unknown benchmark / count_by_severity
// ============================================================================

#[test]
fn unknown_benchmark() {
    let config = make_config(vec![]);
    let findings = evaluate_benchmark("pci-dss", &config);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].message.contains("unknown benchmark"));
}

#[test]
fn benchmark_case_insensitive() {
    let config = make_config(vec![]);
    let findings = evaluate_benchmark("CIS", &config);
    // Should work — empty config = no findings
    assert!(findings.is_empty());
}

#[test]
fn count_by_severity() {
    let findings = vec![
        ComplianceFinding {
            rule_id: "A".into(),
            benchmark: "x".into(),
            severity: FindingSeverity::Critical,
            resource_id: "r".into(),
            message: String::new(),
        },
        ComplianceFinding {
            rule_id: "B".into(),
            benchmark: "x".into(),
            severity: FindingSeverity::High,
            resource_id: "r".into(),
            message: String::new(),
        },
        ComplianceFinding {
            rule_id: "C".into(),
            benchmark: "x".into(),
            severity: FindingSeverity::Medium,
            resource_id: "r".into(),
            message: String::new(),
        },
        ComplianceFinding {
            rule_id: "D".into(),
            benchmark: "x".into(),
            severity: FindingSeverity::Low,
            resource_id: "r".into(),
            message: String::new(),
        },
        ComplianceFinding {
            rule_id: "E".into(),
            benchmark: "x".into(),
            severity: FindingSeverity::Info,
            resource_id: "r".into(),
            message: String::new(),
        },
    ];
    let (c, h, m, l) = compliance::count_by_severity(&findings);
    assert_eq!((c, h, m), (1, 1, 1));
    assert_eq!(l, 2); // Low + Info both count as low
}

// ============================================================================
// FJ-2702: evaluate_gate — exit code
// ============================================================================

fn gate() -> QualityGate {
    QualityGate::default()
}

#[test]
fn gate_exit_zero_pass() {
    assert_eq!(evaluate_gate(&gate(), 0, ""), GateResult::Pass);
}

#[test]
fn gate_exit_nonzero_fail() {
    let r = evaluate_gate(&gate(), 1, "");
    assert!(matches!(r, GateResult::Fail(GateAction::Block, _)));
}

#[test]
fn gate_custom_message() {
    let mut g = gate();
    g.message = Some("build failed".into());
    let r = evaluate_gate(&g, 1, "");
    match r {
        GateResult::Fail(_, msg) => assert_eq!(msg, "build failed"),
        _ => panic!("expected fail"),
    }
}

// ============================================================================
// FJ-2702: evaluate_gate — JSON field
// ============================================================================

#[test]
fn gate_json_threshold_pass() {
    let mut g = gate();
    g.parse = Some("json".into());
    g.field = Some("status".into());
    g.threshold = vec!["ok".into(), "pass".into()];
    let r = evaluate_gate(&g, 0, r#"{"status":"ok"}"#);
    assert_eq!(r, GateResult::Pass);
}

#[test]
fn gate_json_threshold_fail() {
    let mut g = gate();
    g.parse = Some("json".into());
    g.field = Some("status".into());
    g.threshold = vec!["ok".into()];
    let r = evaluate_gate(&g, 0, r#"{"status":"error"}"#);
    assert!(matches!(r, GateResult::Fail(_, _)));
}

#[test]
fn gate_json_field_missing() {
    let mut g = gate();
    g.parse = Some("json".into());
    g.field = Some("coverage".into());
    let r = evaluate_gate(&g, 0, r#"{"status":"ok"}"#);
    assert!(matches!(r, GateResult::Fail(_, _)));
}

#[test]
fn gate_json_invalid() {
    let mut g = gate();
    g.parse = Some("json".into());
    g.field = Some("f".into());
    let r = evaluate_gate(&g, 0, "not json");
    assert!(matches!(r, GateResult::Fail(_, _)));
}

#[test]
fn gate_json_no_field_passes() {
    let mut g = gate();
    g.parse = Some("json".into());
    // No field set — passes without checking
    let r = evaluate_gate(&g, 0, r#"{"x":1}"#);
    assert_eq!(r, GateResult::Pass);
}

#[test]
fn gate_json_min_pass() {
    let mut g = gate();
    g.parse = Some("json".into());
    g.field = Some("coverage".into());
    g.min = Some(80.0);
    let r = evaluate_gate(&g, 0, r#"{"coverage":95.0}"#);
    assert_eq!(r, GateResult::Pass);
}

#[test]
fn gate_json_min_fail() {
    let mut g = gate();
    g.parse = Some("json".into());
    g.field = Some("coverage".into());
    g.min = Some(80.0);
    let r = evaluate_gate(&g, 0, r#"{"coverage":60.0}"#);
    assert!(matches!(r, GateResult::Fail(_, _)));
}

// ============================================================================
// FJ-2702: evaluate_gate — regex
// ============================================================================

#[test]
fn gate_regex_pass() {
    let mut g = gate();
    g.regex = Some("OK|PASS".into());
    let r = evaluate_gate(&g, 0, "test result: PASS");
    assert_eq!(r, GateResult::Pass);
}

#[test]
fn gate_regex_fail() {
    let mut g = gate();
    g.regex = Some("^SUCCESS$".into());
    let r = evaluate_gate(&g, 0, "FAILURE");
    assert!(matches!(r, GateResult::Fail(_, _)));
}

#[test]
fn gate_regex_invalid() {
    let mut g = gate();
    g.regex = Some("[invalid".into());
    let r = evaluate_gate(&g, 0, "anything");
    assert!(matches!(r, GateResult::Fail(_, _)));
}

// ============================================================================
// FJ-2702: GateAction / on_fail
// ============================================================================

#[test]
fn gate_action_warn() {
    let mut g = gate();
    g.on_fail = Some("warn".into());
    let r = evaluate_gate(&g, 1, "");
    assert!(matches!(r, GateResult::Fail(GateAction::Warn, _)));
}

#[test]
fn gate_action_skip() {
    let mut g = gate();
    g.on_fail = Some("skip_dependents".into());
    let r = evaluate_gate(&g, 1, "");
    assert!(matches!(r, GateResult::Fail(GateAction::SkipDependents, _)));
}

#[test]
fn gate_action_default_block() {
    let r = evaluate_gate(&gate(), 1, "");
    assert!(matches!(r, GateResult::Fail(GateAction::Block, _)));
}

// ============================================================================
// FJ-2703: gpu_env_vars
// ============================================================================

#[test]
fn gpu_env_vars_some() {
    let vars = gpu_env_vars(Some(2));
    assert_eq!(vars.len(), 2);
    assert_eq!(vars[0], ("CUDA_VISIBLE_DEVICES".into(), "2".into()));
    assert_eq!(vars[1], ("HIP_VISIBLE_DEVICES".into(), "2".into()));
}

#[test]
fn gpu_env_vars_none() {
    assert!(gpu_env_vars(None).is_empty());
}

#[test]
fn gpu_env_vars_zero() {
    let vars = gpu_env_vars(Some(0));
    assert_eq!(vars[0].1, "0");
}
