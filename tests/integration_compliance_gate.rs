//! FJ-3203: Integration test — Compliance gate pre-apply evaluation.
//!
//! Tests the full compliance gate pipeline: pack loading → resource mapping →
//! rule evaluation → gate pass/fail → severity counting.

use forjar::core::compliance_gate::{
    check_compliance_gate, config_to_resource_map, format_gate_result, ComplianceGateResult,
};
use forjar::core::compliance_pack::{evaluate_pack, load_pack, parse_pack};
use forjar::core::types::{ForjarConfig, Resource, ResourceType};
use tempfile::TempDir;

fn make_config() -> ForjarConfig {
    let mut config = ForjarConfig::default();

    let mut nginx = Resource::default();
    nginx.resource_type = ResourceType::File;
    nginx.owner = Some("root".into());
    nginx.mode = Some("0644".into());
    nginx.tags = vec!["web".into(), "config".into()];
    config.resources.insert("nginx-conf".into(), nginx);

    let mut sshd = Resource::default();
    sshd.resource_type = ResourceType::File;
    sshd.owner = Some("root".into());
    sshd.mode = Some("0600".into());
    config.resources.insert("sshd-config".into(), sshd);

    let mut docker = Resource::default();
    docker.resource_type = ResourceType::Package;
    config.resources.insert("docker-ce".into(), docker);

    config
}

fn write_pack(dir: &std::path::Path, filename: &str, content: &str) {
    std::fs::write(dir.join(filename), content).unwrap();
}

/// Test: gate passes when all packs are satisfied.
#[test]
fn gate_passes_all_rules_satisfied() {
    let dir = TempDir::new().unwrap();
    write_pack(
        dir.path(),
        "ownership.yaml",
        r#"
name: ownership
version: "1.0"
framework: INTERNAL
rules:
  - id: OWN-001
    title: Files must have owner
    severity: error
    type: require
    resource_type: file
    field: owner
"#,
    );

    let config = make_config();
    let result = check_compliance_gate(dir.path(), &config, false).unwrap();
    assert!(result.passed());
    assert_eq!(result.error_count, 0);
    assert_eq!(result.packs_evaluated, 1);
}

/// Test: gate fails when error-severity rule is violated.
#[test]
fn gate_blocks_on_error_severity() {
    let dir = TempDir::new().unwrap();
    write_pack(
        dir.path(),
        "strict.yaml",
        r#"
name: strict-naming
version: "1.0"
framework: CUSTOM
rules:
  - id: NAME-001
    title: Packages must have name field
    severity: error
    type: require
    resource_type: package
    field: name
"#,
    );

    let config = make_config(); // docker-ce has no name field
    let result = check_compliance_gate(dir.path(), &config, false).unwrap();
    assert!(!result.passed());
    assert!(result.error_count > 0);
}

/// Test: gate passes with only warning-severity failures.
#[test]
fn gate_passes_with_warnings_only() {
    let dir = TempDir::new().unwrap();
    write_pack(
        dir.path(),
        "advisory.yaml",
        r#"
name: advisory
version: "1.0"
framework: INTERNAL
rules:
  - id: ADV-001
    title: Packages should have name
    severity: warning
    type: require
    resource_type: package
    field: name
"#,
    );

    let config = make_config();
    let result = check_compliance_gate(dir.path(), &config, false).unwrap();
    assert!(result.passed()); // warnings don't block
    assert!(result.warning_count > 0);
    assert_eq!(result.error_count, 0);
}

/// Test: multiple packs evaluated together.
#[test]
fn multiple_packs_combined_evaluation() {
    let dir = TempDir::new().unwrap();
    write_pack(
        dir.path(),
        "pack-a.yaml",
        r#"
name: pack-a
version: "1.0"
framework: CIS
rules:
  - id: A-001
    title: Files must have owner
    severity: error
    type: require
    resource_type: file
    field: owner
"#,
    );
    write_pack(
        dir.path(),
        "pack-b.yaml",
        r#"
name: pack-b
version: "1.0"
framework: SOC2
rules:
  - id: B-001
    title: Files must have mode
    severity: error
    type: require
    resource_type: file
    field: mode
"#,
    );

    let config = make_config();
    let result = check_compliance_gate(dir.path(), &config, false).unwrap();
    assert!(result.passed());
    assert_eq!(result.packs_evaluated, 2);
}

/// Test: resource map captures all fields.
#[test]
fn resource_map_captures_all_fields() {
    let config = make_config();
    let map = config_to_resource_map(&config);

    let nginx = map.get("nginx-conf").unwrap();
    assert_eq!(nginx.get("type").unwrap(), "file");
    assert_eq!(nginx.get("owner").unwrap(), "root");
    assert_eq!(nginx.get("mode").unwrap(), "0644");
    assert_eq!(nginx.get("tags").unwrap(), "web,config");

    let sshd = map.get("sshd-config").unwrap();
    assert_eq!(sshd.get("mode").unwrap(), "0600");
    assert!(sshd.get("tags").is_none()); // no tags on sshd

    let docker = map.get("docker-ce").unwrap();
    assert_eq!(docker.get("type").unwrap(), "package");
    assert!(docker.get("owner").is_none()); // no owner on package
}

/// Test: empty policy directory means no gate evaluation needed.
#[test]
fn empty_policy_dir_passes() {
    let dir = TempDir::new().unwrap();
    let config = make_config();
    let result = check_compliance_gate(dir.path(), &config, false).unwrap();
    assert!(result.passed());
    assert_eq!(result.packs_evaluated, 0);
}

/// Test: malformed pack YAML is skipped (not crash).
#[test]
fn malformed_pack_skipped() {
    let dir = TempDir::new().unwrap();
    write_pack(dir.path(), "bad.yaml", "this is not valid yaml: [[[");

    let config = make_config();
    let result = check_compliance_gate(dir.path(), &config, true).unwrap();
    // Bad packs are skipped, gate still passes
    assert!(result.passed());
    assert_eq!(result.packs_evaluated, 0);
}

/// Test: format_gate_result output contains expected information.
#[test]
fn format_gate_result_content() {
    let result = ComplianceGateResult {
        packs_evaluated: 3,
        results: vec![],
        error_count: 1,
        warning_count: 2,
    };
    let text = format_gate_result(&result);
    assert!(text.contains("FAIL"));
    assert!(text.contains("3 packs"));
    assert!(text.contains("1 errors"));
    assert!(text.contains("2 warnings"));
}

/// Test: assert rule catches field value mismatch.
#[test]
fn assert_rule_field_mismatch() {
    let dir = TempDir::new().unwrap();
    write_pack(
        dir.path(),
        "assert-test.yaml",
        r#"
name: assert-test
version: "1.0"
framework: CIS
rules:
  - id: ASSERT-001
    title: File mode must be 0600
    severity: error
    type: assert
    resource_type: file
    field: mode
    expected: "0600"
"#,
    );

    let config = make_config(); // nginx has 0644, sshd has 0600
    let result = check_compliance_gate(dir.path(), &config, false).unwrap();
    // nginx-conf has mode 0644 which != 0600, so this should fail
    assert!(!result.passed());
    assert!(result.error_count > 0);
}

/// Test: deny rule catches forbidden pattern.
#[test]
fn deny_rule_catches_pattern() {
    let dir = TempDir::new().unwrap();
    write_pack(
        dir.path(),
        "deny-test.yaml",
        r#"
name: deny-test
version: "1.0"
framework: STIG
rules:
  - id: DENY-001
    title: No world-readable files
    severity: error
    type: deny
    resource_type: file
    field: mode
    pattern: "0644"
"#,
    );

    let config = make_config(); // nginx has 0644
    let result = check_compliance_gate(dir.path(), &config, false).unwrap();
    assert!(!result.passed());
}

/// Test: require_tag rule passes when tags present.
#[test]
fn require_tag_passes() {
    let dir = TempDir::new().unwrap();
    write_pack(
        dir.path(),
        "tag-test.yaml",
        r#"
name: tag-test
version: "1.0"
framework: INTERNAL
rules:
  - id: TAG-001
    title: All resources must have config tag
    severity: error
    type: require_tag
    tag: config
"#,
    );

    // Only nginx has "config" tag, sshd and docker don't
    let config = make_config();
    let result = check_compliance_gate(dir.path(), &config, false).unwrap();
    // Should fail because sshd-config and docker-ce don't have "config" tag
    assert!(!result.passed());
}

/// Test: pack with script rule (passing script).
#[test]
fn script_rule_passing() {
    let pack = parse_pack(
        r#"
name: script-test
version: "1.0"
framework: TEST
rules:
  - id: SCRIPT-001
    title: True always passes
    severity: error
    type: script
    script: "true"
"#,
    )
    .unwrap();

    let resources = std::collections::HashMap::new();
    let result = evaluate_pack(&pack, &resources);
    assert_eq!(result.passed_count(), 1);
}

/// Test: pack with script rule (failing script).
#[test]
fn script_rule_failing() {
    let pack = parse_pack(
        r#"
name: script-fail
version: "1.0"
framework: TEST
rules:
  - id: SCRIPT-002
    title: False always fails
    severity: error
    type: script
    script: "false"
"#,
    )
    .unwrap();

    let resources = std::collections::HashMap::new();
    let result = evaluate_pack(&pack, &resources);
    assert_eq!(result.failed_count(), 1);
}

/// Test: CIS pack loads and evaluates against well-configured resources.
#[test]
fn cis_pack_integration() {
    use forjar::core::cis_ubuntu_pack::cis_ubuntu_2204_pack;

    let pack = cis_ubuntu_2204_pack();
    assert!(pack.rules.len() >= 20);

    // Well-configured resource set
    let mut resources = std::collections::HashMap::new();
    let mut file = std::collections::HashMap::new();
    file.insert("type".into(), "file".into());
    file.insert("owner".into(), "root".into());
    file.insert("mode".into(), "0644".into());
    file.insert("content".into(), "configured".into());
    resources.insert("etc-passwd".into(), file);

    let mut svc = std::collections::HashMap::new();
    svc.insert("type".into(), "service".into());
    svc.insert("enabled".into(), "false".into());
    resources.insert("telnetd".into(), svc);

    let result = evaluate_pack(&pack, &resources);
    // Should have some passes and some fails depending on rules
    assert!(result.results.len() >= 20);
    let rate = result.pass_rate();
    assert!(rate > 0.0, "should have at least some passes");
}

/// Test: load pack from file and evaluate.
#[test]
fn load_and_evaluate_roundtrip() {
    let dir = TempDir::new().unwrap();
    let pack_path = dir.path().join("roundtrip.yaml");
    std::fs::write(
        &pack_path,
        r#"
name: roundtrip
version: "2.0"
framework: SOC2
rules:
  - id: RT-001
    title: Files have owner
    severity: error
    type: require
    resource_type: file
    field: owner
  - id: RT-002
    title: Files have mode
    severity: warning
    type: require
    resource_type: file
    field: mode
"#,
    )
    .unwrap();

    let pack = load_pack(&pack_path).unwrap();
    assert_eq!(pack.name, "roundtrip");
    assert_eq!(pack.rules.len(), 2);

    let config = make_config();
    let resources = config_to_resource_map(&config);
    let result = evaluate_pack(&pack, &resources);
    // Both files have owner and mode
    assert_eq!(result.passed_count(), 2);
}

/// Test: verbose mode prints pack evaluation details.
#[test]
fn verbose_mode_outputs_details() {
    let dir = TempDir::new().unwrap();
    write_pack(
        dir.path(),
        "verbose-test.yaml",
        r#"
name: verbose-test
version: "1.0"
framework: INTERNAL
rules:
  - id: V-001
    title: Files have owner
    severity: error
    type: require
    resource_type: file
    field: owner
"#,
    );

    let config = make_config();
    // verbose=true exercises the stderr printing path
    let result = check_compliance_gate(dir.path(), &config, true).unwrap();
    assert!(result.passed());
}
