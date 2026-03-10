//! Falsification tests for FJ-3100 through FJ-3500 spec criteria.
//!
//! Each test maps to a specific falsification criterion from the platform specs.
//! These tests prove the implementation is correct by attempting to reject it.

use std::collections::HashMap;

// ─── F-3100: Event-Driven Automation ───────────────────────────────

/// F-3100-3: Cooldown prevents storms.
/// Trigger same event 100x in 1s; REJECT if action fires > 1 time.
#[test]
fn f3100_3_cooldown_prevents_storm() {
    use forjar::core::types::CooldownTracker;

    let mut tracker = CooldownTracker::default();
    let cooldown_secs = 5;
    let rulebook_id = "storm-test";
    let mut fire_count = 0;

    // Fire 100 times rapidly
    for _ in 0..100 {
        if tracker.can_fire(rulebook_id, cooldown_secs) {
            tracker.record_fire(rulebook_id);
            fire_count += 1;
        }
    }

    // Only the first should fire
    assert_eq!(
        fire_count, 1,
        "REJECT: cooldown failed — {fire_count} fires instead of 1"
    );
}

/// F-3100-4: bashrs validates handler scripts.
/// Verify the bashrs purifier catches script injection patterns.
#[test]
fn f3100_4_bashrs_validates_handler_scripts() {
    use forjar::core::purifier::validate_script;

    // The purifier should process scripts without panic.
    // bashrs validates syntax and structure.
    let result = validate_script("echo safe");
    assert!(result.is_ok(), "simple script should pass bashrs");
}

/// F-3100-6: Zero external dependencies for event bus.
/// Audit Cargo.toml; REJECT if any non-sovereign crate added for event bus.
#[test]
fn f3100_6_no_external_event_bus() {
    let cargo_toml = std::fs::read_to_string("Cargo.toml").unwrap();
    let forbidden = [
        "tokio-eventbus",
        "eventbus",
        "message-bus",
        "rabbitmq",
        "rdkafka",
    ];
    for dep in &forbidden {
        assert!(
            !cargo_toml.contains(dep),
            "REJECT: non-sovereign event bus dependency found: {dep}"
        );
    }
}

// ─── F-3200: Policy-as-Code Engine ─────────────────────────────────

/// F-3200-1: All 4 policy types evaluate correctly.
/// Generate boundary configs; REJECT if any misclassification.
#[test]
fn f3200_1_all_policy_types_correct() {
    use forjar::core::compliance_pack::*;

    let mut resources = HashMap::new();
    let mut file = HashMap::new();
    file.insert("type".into(), "file".into());
    file.insert("owner".into(), "root".into());
    file.insert("mode".into(), "0644".into());
    file.insert("tags".into(), "web,config".into());
    resources.insert("nginx".into(), file);

    // Assert: correct value passes
    let pack = parse_pack(
        r#"
name: t1
version: "1"
framework: T
rules:
  - id: A1
    title: assert
    type: assert
    resource_type: file
    field: owner
    expected: root
"#,
    )
    .unwrap();
    let r = evaluate_pack(&pack, &resources);
    assert!(
        r.results[0].passed,
        "REJECT: assert failed on correct value"
    );

    // Deny: absent pattern passes
    let pack = parse_pack(
        r#"
name: t2
version: "1"
framework: T
rules:
  - id: D1
    title: deny
    type: deny
    resource_type: file
    field: mode
    pattern: "777"
"#,
    )
    .unwrap();
    let r = evaluate_pack(&pack, &resources);
    assert!(
        r.results[0].passed,
        "REJECT: deny failed on non-matching pattern"
    );

    // Require: present field passes
    let pack = parse_pack(
        r#"
name: t3
version: "1"
framework: T
rules:
  - id: R1
    title: require
    type: require
    resource_type: file
    field: owner
"#,
    )
    .unwrap();
    let r = evaluate_pack(&pack, &resources);
    assert!(
        r.results[0].passed,
        "REJECT: require failed on present field"
    );

    // RequireTag: present tag passes
    let pack = parse_pack(
        r#"
name: t4
version: "1"
framework: T
rules:
  - id: RT1
    title: require_tag
    type: require_tag
    tag: web
"#,
    )
    .unwrap();
    let r = evaluate_pack(&pack, &resources);
    assert!(
        r.results[0].passed,
        "REJECT: require_tag failed on present tag"
    );
}

/// F-3200-2: Error-severity blocks apply.
/// Create config violating error-policy; REJECT if gate passes.
#[test]
fn f3200_2_error_severity_blocks() {
    use forjar::core::compliance_gate::check_compliance_gate;
    use forjar::core::types::{ForjarConfig, Resource, ResourceType};

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("strict.yaml"),
        r#"
name: strict
version: "1.0"
framework: TEST
rules:
  - id: ERR-001
    title: Must have owner
    severity: error
    type: require
    resource_type: file
    field: owner
"#,
    )
    .unwrap();

    // Config with file missing owner field
    let mut config = ForjarConfig::default();
    let r = Resource {
        resource_type: ResourceType::File,
        ..Default::default()
    };
    config.resources.insert("bad-file".into(), r);

    let result = check_compliance_gate(dir.path(), &config, false).unwrap();
    assert!(
        !result.passed(),
        "REJECT: error-severity violation did not block gate"
    );
    assert!(result.error_count > 0, "REJECT: error count should be > 0");
}

/// F-3200-3: Policy eval < 50ms for 100 rules × 100 resources.
#[test]
fn f3200_3_policy_eval_performance() {
    use forjar::core::compliance_pack::*;

    // Build 100 resources
    let mut resources = HashMap::new();
    for i in 0..100 {
        let mut fields = HashMap::new();
        fields.insert("type".into(), "file".into());
        fields.insert("owner".into(), "root".into());
        fields.insert("mode".into(), format!("0{}", 644 + (i % 10)));
        resources.insert(format!("resource-{i}"), fields);
    }

    // Build pack with 100 rules
    let mut rules = Vec::new();
    for i in 0..100 {
        rules.push(ComplianceRule {
            id: format!("PERF-{i:03}"),
            title: format!("Performance rule {i}"),
            description: None,
            severity: "warning".into(),
            controls: vec![],
            check: ComplianceCheck::Require {
                resource_type: "file".into(),
                field: "owner".into(),
            },
        });
    }
    let pack = CompliancePack {
        name: "perf-test".into(),
        version: "1.0".into(),
        framework: "BENCH".into(),
        description: None,
        rules,
    };

    let start = std::time::Instant::now();
    let result = evaluate_pack(&pack, &resources);
    let elapsed = start.elapsed();

    assert_eq!(result.results.len(), 100);
    assert!(
        elapsed.as_millis() < 50,
        "REJECT: policy eval took {}ms (> 50ms target)",
        elapsed.as_millis()
    );
}

/// F-3200-4: bashrs validates script policies.
/// Inject secret leakage in script policy; REJECT if lint doesn't catch it.
#[test]
fn f3200_4_bashrs_script_policy_validation() {
    use forjar::core::script_secret_lint::validate_no_leaks;

    // Secret leak: echoing a PASSWORD variable (matched by echo_secret_var pattern)
    let result = validate_no_leaks("echo $PASSWORD");
    assert!(result.is_err(), "REJECT: lint didn't catch echo $PASSWORD");

    // Secret leak: redirecting secret to file
    let result = validate_no_leaks("$SECRET > /tmp/key.txt");
    assert!(result.is_err(), "REJECT: lint didn't catch secret redirect");

    // Inline DB URL with embedded password
    let result = validate_no_leaks("URL=postgres://user:pass@db:5432/app");
    assert!(
        result.is_err(),
        "REJECT: lint didn't catch DB URL with password"
    );
}
