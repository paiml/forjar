//! FJ-1387/3205/3208: Compliance benchmarks, compliance packs, and policy
//! coverage falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-1387: Compliance benchmarks (CIS, NIST, SOC2, HIPAA)
//! - FJ-3205: Compliance packs (parse, evaluate, filesystem)
//! - FJ-3208: Policy coverage (compute, format, JSON)
//!
//! Usage: cargo test --test falsification_compliance_policy

use forjar::core::compliance::{count_by_severity, evaluate_benchmark, supported_benchmarks};
use forjar::core::compliance_pack::{evaluate_pack, list_packs, load_pack, parse_pack};
use forjar::core::policy_coverage::{compute_coverage, coverage_to_json, format_coverage};
use forjar::core::types::*;
use indexmap::IndexMap;
use std::collections::HashMap;

fn cfg1(id: &str, r: Resource) -> ForjarConfig {
    let mut resources = IndexMap::new();
    resources.insert(id.into(), r);
    ForjarConfig {
        name: "test".into(),
        resources,
        ..Default::default()
    }
}

// ============================================================================
// FJ-1387: supported_benchmarks + unknown benchmark
// ============================================================================

#[test]
fn compliance_supported_benchmarks_and_unknown() {
    let b = supported_benchmarks();
    assert_eq!(b.len(), 4);
    assert!(b.contains(&"cis") && b.contains(&"nist-800-53"));
    assert!(b.contains(&"soc2") && b.contains(&"hipaa"));
    // Unknown benchmark returns info finding
    let findings = evaluate_benchmark("bogus", &ForjarConfig::default());
    assert_eq!(findings.len(), 1);
    assert!(findings[0].message.contains("unknown"));
}

// ============================================================================
// FJ-1387: CIS rules (consolidated)
// ============================================================================

#[test]
fn cis_rules_all() {
    // World-writable mode triggers CIS-6.1.1
    let f1 = evaluate_benchmark(
        "cis",
        &cfg1(
            "f",
            Resource {
                resource_type: ResourceType::File,
                mode: Some("0777".into()),
                ..Default::default()
            },
        ),
    );
    assert!(f1.iter().any(|f| f.rule_id == "CIS-6.1.1"));
    // Restrictive mode is clean
    let f2 = evaluate_benchmark(
        "cis",
        &cfg1(
            "f",
            Resource {
                resource_type: ResourceType::File,
                mode: Some("0600".into()),
                ..Default::default()
            },
        ),
    );
    assert!(!f2.iter().any(|f| f.rule_id == "CIS-6.1.1"));
    // Root-owned /tmp file
    let f3 = evaluate_benchmark(
        "cis",
        &cfg1(
            "f",
            Resource {
                resource_type: ResourceType::File,
                path: Some("/tmp/lock".into()),
                owner: Some("root".into()),
                ..Default::default()
            },
        ),
    );
    assert!(f3.iter().any(|f| f.rule_id == "CIS-1.1.5"));
    // Service without restart policy
    let f4 = evaluate_benchmark(
        "cis",
        &cfg1(
            "s",
            Resource {
                resource_type: ResourceType::Service,
                ..Default::default()
            },
        ),
    );
    assert!(f4.iter().any(|f| f.rule_id == "CIS-5.2.1"));
    // Package without version pin
    let f5 = evaluate_benchmark(
        "cis",
        &cfg1(
            "p",
            Resource {
                resource_type: ResourceType::Package,
                ..Default::default()
            },
        ),
    );
    assert!(f5.iter().any(|f| f.rule_id == "CIS-6.2.1"));
}

// ============================================================================
// FJ-1387: NIST 800-53 rules (consolidated)
// ============================================================================

#[test]
fn nist_rules_all() {
    // AC-3: missing owner + mode on file
    let f1 = evaluate_benchmark(
        "nist-800-53",
        &cfg1(
            "f",
            Resource {
                resource_type: ResourceType::File,
                ..Default::default()
            },
        ),
    );
    assert!(f1.iter().any(|f| f.rule_id == "NIST-AC-3.1"));
    assert!(f1.iter().any(|f| f.rule_id == "NIST-AC-3.2"));
    // AC-6: root service
    let f2 = evaluate_benchmark(
        "nist",
        &cfg1(
            "s",
            Resource {
                resource_type: ResourceType::Service,
                owner: Some("root".into()),
                ..Default::default()
            },
        ),
    );
    assert!(f2.iter().any(|f| f.rule_id == "NIST-AC-6"));
    // SC-28: sensitive path without mode
    let f3 = evaluate_benchmark(
        "nist",
        &cfg1(
            "ssh",
            Resource {
                resource_type: ResourceType::File,
                path: Some("/etc/ssh/sshd_config".into()),
                ..Default::default()
            },
        ),
    );
    assert!(f3.iter().any(|f| f.rule_id == "NIST-SC-28"));
    // SI-7: external source no check
    let f4 = evaluate_benchmark(
        "nist",
        &cfg1(
            "dl",
            Resource {
                resource_type: ResourceType::File,
                source: Some("https://example.com/bin".into()),
                ..Default::default()
            },
        ),
    );
    assert!(f4.iter().any(|f| f.rule_id == "NIST-SI-7"));
}

// ============================================================================
// FJ-1387: SOC2 + HIPAA rules (consolidated)
// ============================================================================

#[test]
fn soc2_and_hipaa_rules() {
    // SOC2: file missing owner
    let f1 = evaluate_benchmark(
        "soc2",
        &cfg1(
            "f",
            Resource {
                resource_type: ResourceType::File,
                ..Default::default()
            },
        ),
    );
    assert!(f1.iter().any(|f| f.rule_id == "SOC2-CC6.1"));
    // SOC2: service no restart_on
    let f2 = evaluate_benchmark(
        "soc2",
        &cfg1(
            "s",
            Resource {
                resource_type: ResourceType::Service,
                ..Default::default()
            },
        ),
    );
    assert!(f2.iter().any(|f| f.rule_id == "SOC2-CC7.2"));
    // HIPAA: file with other-access mode
    let f3 = evaluate_benchmark(
        "hipaa",
        &cfg1(
            "f",
            Resource {
                resource_type: ResourceType::File,
                mode: Some("0644".into()),
                ..Default::default()
            },
        ),
    );
    assert!(f3.iter().any(|f| f.rule_id == "HIPAA-164.312a"));
    // HIPAA: unencrypted port
    let f4 = evaluate_benchmark(
        "hipaa",
        &cfg1(
            "net",
            Resource {
                resource_type: ResourceType::Network,
                port: Some("80".into()),
                ..Default::default()
            },
        ),
    );
    assert!(f4.iter().any(|f| f.rule_id == "HIPAA-164.312e"));
}

// ============================================================================
// FJ-1387: count_by_severity
// ============================================================================

#[test]
fn count_severity_sums_to_total() {
    let mut cfg = ForjarConfig::default();
    cfg.resources.insert(
        "f1".into(),
        Resource {
            resource_type: ResourceType::File,
            mode: Some("0777".into()),
            ..Default::default()
        },
    );
    cfg.resources.insert(
        "net".into(),
        Resource {
            resource_type: ResourceType::Network,
            port: Some("8080".into()),
            ..Default::default()
        },
    );
    let findings = evaluate_benchmark("hipaa", &cfg);
    let (c, h, m, l) = count_by_severity(&findings);
    assert!(c + h + m + l == findings.len());
}

#[test]
fn pack_parse_rules_and_errors() {
    // Assert rule: default severity = "warning"
    let p1 = parse_pack("name: t\nversion: '1'\nframework: CIS\nrules:\n  - id: R1\n    title: t\n    type: assert\n    resource_type: file\n    field: owner\n    expected: root").unwrap();
    assert_eq!(p1.rules[0].severity, "warning");
    // Deny rule: explicit severity = "error"
    let p2 = parse_pack("name: t\nversion: '1'\nframework: STIG\nrules:\n  - id: D1\n    title: t\n    severity: error\n    type: deny\n    resource_type: file\n    field: mode\n    pattern: '777'").unwrap();
    assert_eq!(p2.rules[0].severity, "error");
    // Require rule: framework check
    let p3 = parse_pack("name: t\nversion: '1'\nframework: SOC2\nrules:\n  - id: Q1\n    title: t\n    type: require\n    resource_type: file\n    field: owner").unwrap();
    assert_eq!(p3.framework, "SOC2");
    // Invalid YAML errors
    assert!(parse_pack("not: valid: [yaml").is_err());
}

fn res(entries: &[(&str, &[(&str, &str)])]) -> HashMap<String, HashMap<String, String>> {
    let mut resources = HashMap::new();
    for (name, fields) in entries {
        let mut map = HashMap::new();
        for (k, v) in *fields {
            map.insert(k.to_string(), v.to_string());
        }
        resources.insert(name.to_string(), map);
    }
    resources
}

fn pack_yaml(rule_type: &str, extra: &str) -> String {
    format!(
        r#"
name: t
version: "1"
framework: CIS
rules:
  - id: R1
    title: test
    type: {rule_type}
    {extra}
"#
    )
}

#[test]
fn pack_eval_all_check_types() {
    // Assert pass + fail
    let assert_yaml = pack_yaml(
        "assert",
        "resource_type: file\n    field: owner\n    expected: root",
    );
    let assert_pack = parse_pack(&assert_yaml).unwrap();
    let pass = evaluate_pack(
        &assert_pack,
        &res(&[("f1", &[("type", "file"), ("owner", "root")])]),
    );
    assert_eq!(pass.passed_count(), 1);
    assert!((pass.pass_rate() - 100.0).abs() < f64::EPSILON);
    let fail = evaluate_pack(
        &assert_pack,
        &res(&[("f1", &[("type", "file"), ("owner", "nobody")])]),
    );
    assert_eq!(fail.failed_count(), 1);
    // Deny catches pattern
    let deny_pack = parse_pack(&pack_yaml(
        "deny",
        "resource_type: file\n    field: mode\n    pattern: \"777\"",
    ))
    .unwrap();
    assert_eq!(
        evaluate_pack(
            &deny_pack,
            &res(&[("f1", &[("type", "file"), ("mode", "0777")])])
        )
        .failed_count(),
        1
    );
    // Require missing field
    let req_pack = parse_pack(&pack_yaml(
        "require",
        "resource_type: file\n    field: owner",
    ))
    .unwrap();
    assert_eq!(
        evaluate_pack(&req_pack, &res(&[("f1", &[("type", "file")])])).failed_count(),
        1
    );
    // Require tag missing
    let tag_pack = parse_pack(&pack_yaml("require_tag", "tag: production")).unwrap();
    assert_eq!(
        evaluate_pack(&tag_pack, &res(&[("r1", &[("tags", "dev,staging")])])).failed_count(),
        1
    );
    // Empty rules → 100% pass rate
    let empty = parse_pack("name: e\nversion: '1'\nframework: CIS\nrules: []").unwrap();
    assert!((evaluate_pack(&empty, &HashMap::new()).pass_rate() - 100.0).abs() < f64::EPSILON);
}

// ============================================================================
// FJ-3205: load_pack / list_packs
// ============================================================================

#[test]
fn pack_filesystem_ops() {
    let dir = tempfile::tempdir().unwrap();
    // load_pack
    let path = dir.path().join("test.yaml");
    std::fs::write(
        &path,
        "name: loaded\nversion: '1'\nframework: CIS\nrules: []",
    )
    .unwrap();
    assert_eq!(load_pack(&path).unwrap().name, "loaded");
    // list_packs filters by extension
    std::fs::write(dir.path().join("cis.yaml"), "").unwrap();
    std::fs::write(dir.path().join("stig.yml"), "").unwrap();
    std::fs::write(dir.path().join("readme.txt"), "").unwrap();
    let packs = list_packs(dir.path());
    assert!(packs.contains(&"cis".to_string()));
    assert!(packs.contains(&"stig".to_string()));
    assert!(!packs.contains(&"readme".to_string()));
    // Empty dir
    let empty = tempfile::tempdir().unwrap();
    assert!(list_packs(empty.path()).is_empty());
}

// ============================================================================
// FJ-3208: compute_coverage
// ============================================================================

fn cov_cfg(resources: &[(&str, ResourceType)], policies: Vec<PolicyRule>) -> ForjarConfig {
    let mut cfg = ForjarConfig::default();
    for (id, rtype) in resources {
        cfg.resources.insert(
            id.to_string(),
            Resource {
                resource_type: rtype.clone(),
                ..Default::default()
            },
        );
    }
    cfg.policies = policies;
    cfg
}

fn req_pol(rtype: &str) -> PolicyRule {
    PolicyRule {
        id: Some(format!("P-{rtype}")),
        rule_type: PolicyRuleType::Require,
        message: "test".into(),
        resource_type: Some(rtype.into()),
        tag: None,
        field: Some("owner".into()),
        condition_field: None,
        condition_value: None,
        max_count: None,
        min_count: None,
        severity: None,
        remediation: None,
        compliance: vec![],
    }
}

#[test]
fn coverage_full_and_partial() {
    let full = compute_coverage(&cov_cfg(
        &[("f1", ResourceType::File), ("f2", ResourceType::File)],
        vec![req_pol("file")],
    ));
    assert_eq!(full.total_resources, 2);
    assert!(full.fully_covered());
    assert!((full.coverage_percent() - 100.0).abs() < f64::EPSILON);
    let partial = compute_coverage(&cov_cfg(
        &[("f1", ResourceType::File), ("p1", ResourceType::Package)],
        vec![req_pol("file")],
    ));
    assert_eq!(partial.uncovered, vec!["p1"]);
    assert!(!partial.fully_covered());
}

#[test]
fn coverage_none_and_empty() {
    let none = compute_coverage(&cov_cfg(
        &[("f1", ResourceType::File), ("p1", ResourceType::Package)],
        vec![],
    ));
    assert_eq!(none.covered_resources, 0);
    let empty = compute_coverage(&cov_cfg(&[], vec![req_pol("file")]));
    assert!(empty.fully_covered());
}

#[test]
fn coverage_format_and_json() {
    let cfg = cov_cfg(
        &[("f1", ResourceType::File), ("p1", ResourceType::Package)],
        vec![req_pol("file")],
    );
    let cov = compute_coverage(&cfg);
    assert!(format_coverage(&cov).contains("50.0%"));
    let json = coverage_to_json(&cov);
    assert_eq!(json["total_resources"], 2);
}

#[test]
fn coverage_framework_and_type_tracking() {
    let mut pol = req_pol("file");
    pol.compliance = vec![ComplianceMapping {
        framework: "CIS".into(),
        control: "1.1".into(),
    }];
    let cfg = cov_cfg(&[("f1", ResourceType::File)], vec![pol, req_pol("package")]);
    let cov = compute_coverage(&cfg);
    assert!(cov.frameworks.contains("CIS"));
    assert_eq!(cov.by_type.get("require"), Some(&2));
}
