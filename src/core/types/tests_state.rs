//! Tests for state lock, provenance, plan, and apply result types (FJ-001, FJ-131, FJ-132).

use super::*;
use indexmap::IndexMap;
use std::collections::HashMap;

#[test]
fn test_fj001_state_lock_roundtrip() {
    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "lambda".to_string(),
        hostname: "test-box".to_string(),
        generated_at: "2026-02-16T14:00:00Z".to_string(),
        generator: "forjar 0.1.0".to_string(),
        blake3_version: "1.8".to_string(),
        resources: IndexMap::from([(
            "test-pkg".to_string(),
            ResourceLock {
                resource_type: ResourceType::Package,
                status: ResourceStatus::Converged,
                applied_at: Some("2026-02-16T14:00:01Z".to_string()),
                duration_seconds: Some(1.5),
                hash: "blake3:abc123".to_string(),
                details: HashMap::new(),
            },
        )]),
    };
    let yaml = serde_yaml_ng::to_string(&lock).unwrap();
    let lock2: StateLock = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(lock2.machine, "lambda");
    assert_eq!(
        lock2.resources["test-pkg"].status,
        ResourceStatus::Converged
    );
}

#[test]
fn test_fj001_provenance_event_serde() {
    let event = ProvenanceEvent::ApplyStarted {
        machine: "lambda".to_string(),
        run_id: "r-abc".to_string(),
        forjar_version: "0.1.0".to_string(),
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("\"event\":\"apply_started\""));
    assert!(json.contains("\"run_id\":\"r-abc\""));
}

#[test]
fn test_fj001_yaml_value_to_string() {
    assert_eq!(
        yaml_value_to_string(&serde_yaml_ng::Value::String("hello".into())),
        "hello"
    );
    assert_eq!(
        yaml_value_to_string(&serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(
            42
        ))),
        "42"
    );
    assert_eq!(
        yaml_value_to_string(&serde_yaml_ng::Value::Bool(true)),
        "true"
    );
    assert_eq!(yaml_value_to_string(&serde_yaml_ng::Value::Null), "");
    // Sequence/Mapping falls through to Debug format
    let seq = serde_yaml_ng::Value::Sequence(vec![serde_yaml_ng::Value::Null]);
    assert!(!yaml_value_to_string(&seq).is_empty());
}

#[test]
fn test_fj131_global_lock_roundtrip() {
    let lock = GlobalLock {
        schema: "1.0".to_string(),
        name: "prod".to_string(),
        last_apply: "2026-02-25T12:00:00Z".to_string(),
        generator: "forjar 0.1.0".to_string(),
        machines: IndexMap::from([(
            "web".to_string(),
            MachineSummary {
                resources: 5,
                converged: 4,
                failed: 1,
                last_apply: "2026-02-25T12:00:00Z".to_string(),
            },
        )]),
    };
    let yaml = serde_yaml_ng::to_string(&lock).unwrap();
    let lock2: GlobalLock = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(lock2.name, "prod");
    assert_eq!(lock2.machines["web"].resources, 5);
    assert_eq!(lock2.machines["web"].converged, 4);
    assert_eq!(lock2.machines["web"].failed, 1);
}

#[test]
fn test_fj131_resource_lock_optional_fields() {
    let yaml = r#"
type: file
status: converged
hash: "blake3:abc"
"#;
    let rl: ResourceLock = serde_yaml_ng::from_str(yaml).unwrap();
    assert!(rl.applied_at.is_none());
    assert!(rl.duration_seconds.is_none());
    assert!(rl.details.is_empty());
}

#[test]
fn test_fj131_resource_status_serde_roundtrip() {
    let statuses = [
        ResourceStatus::Converged,
        ResourceStatus::Failed,
        ResourceStatus::Drifted,
        ResourceStatus::Unknown,
    ];
    for s in &statuses {
        let yaml = serde_yaml_ng::to_string(s).unwrap();
        let back: ResourceStatus = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(&back, s, "roundtrip failed for {:?}", s);
    }
}

#[test]
fn test_fj131_provenance_event_all_variants_serde() {
    let events = vec![
        ProvenanceEvent::ApplyStarted {
            machine: "m".to_string(),
            run_id: "r".to_string(),
            forjar_version: "0.1".to_string(),
        },
        ProvenanceEvent::ResourceStarted {
            machine: "m".to_string(),
            resource: "pkg".to_string(),
            action: "create".to_string(),
        },
        ProvenanceEvent::ResourceConverged {
            machine: "m".to_string(),
            resource: "pkg".to_string(),
            duration_seconds: 1.5,
            hash: "blake3:h".to_string(),
        },
        ProvenanceEvent::ResourceFailed {
            machine: "m".to_string(),
            resource: "pkg".to_string(),
            error: "oops".to_string(),
        },
        ProvenanceEvent::ApplyCompleted {
            machine: "m".to_string(),
            run_id: "r".to_string(),
            resources_converged: 3,
            resources_unchanged: 1,
            resources_failed: 0,
            total_seconds: 5.0,
        },
        ProvenanceEvent::DriftDetected {
            machine: "m".to_string(),
            resource: "cfg".to_string(),
            expected_hash: "a".to_string(),
            actual_hash: "b".to_string(),
        },
        ProvenanceEvent::SecretAccessed {
            resource: "db-config".to_string(),
            marker_count: 2,
            identity_recipient: "age1test".to_string(),
        },
        ProvenanceEvent::SecretRotated {
            file: "forjar.yaml".to_string(),
            marker_count: 3,
            new_recipients: vec!["age1a".to_string(), "age1b".to_string()],
        },
    ];
    for event in &events {
        let json = serde_json::to_string(event).unwrap();
        let back: ProvenanceEvent = serde_json::from_str(&json).unwrap();
        // Verify roundtrip doesn't panic and produces valid JSON
        let json2 = serde_json::to_string(&back).unwrap();
        assert_eq!(json, json2);
    }
}

#[test]
fn test_fj131_timestamped_event_flatten() {
    let te = TimestampedEvent {
        ts: "2026-02-25T12:00:00Z".to_string(),
        event: ProvenanceEvent::DriftDetected {
            machine: "web".to_string(),
            resource: "cfg".to_string(),
            expected_hash: "aaa".to_string(),
            actual_hash: "bbb".to_string(),
        },
    };
    let json = serde_json::to_string(&te).unwrap();
    // Flattened: ts appears at top level alongside event fields
    assert!(json.contains("\"ts\":\"2026-02-25T12:00:00Z\""));
    assert!(json.contains("\"event\":\"drift_detected\""));
    assert!(json.contains("\"expected_hash\":\"aaa\""));
    // Verify roundtrip
    let back: TimestampedEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(back.ts, "2026-02-25T12:00:00Z");
}

#[test]
fn test_fj131_planned_change_serialize() {
    let pc = PlannedChange {
        resource_id: "web-config".to_string(),
        machine: "web".to_string(),
        resource_type: ResourceType::File,
        action: PlanAction::Create,
        description: "Create file /etc/app.conf".to_string(),
    };
    let json = serde_json::to_string(&pc).unwrap();
    assert!(json.contains("\"resource_id\":\"web-config\""));
    assert!(json.contains("\"action\":\"create\""));
}

#[test]
fn test_fj131_execution_plan_serialize() {
    let ep = ExecutionPlan {
        name: "prod".to_string(),
        changes: vec![],
        execution_order: vec!["a".to_string(), "b".to_string()],
        to_create: 1,
        to_update: 2,
        to_destroy: 0,
        unchanged: 3,
    };
    let json = serde_json::to_string(&ep).unwrap();
    assert!(json.contains("\"to_create\":1"));
    assert!(json.contains("\"unchanged\":3"));
}

#[test]
fn test_fj131_yaml_value_to_string_mapping() {
    let mut map = serde_yaml_ng::Mapping::new();
    map.insert(
        serde_yaml_ng::Value::String("key".into()),
        serde_yaml_ng::Value::String("val".into()),
    );
    let val = serde_yaml_ng::Value::Mapping(map);
    let s = yaml_value_to_string(&val);
    assert!(
        !s.is_empty(),
        "Mapping should produce non-empty debug string"
    );
}

#[test]
fn test_fj131_yaml_value_to_string_float() {
    let n = serde_yaml_ng::Number::from(9.81_f64);
    let val = serde_yaml_ng::Value::Number(n);
    assert_eq!(yaml_value_to_string(&val), "9.81");
}

#[test]
fn test_fj131_apply_result_debug() {
    let ar = ApplyResult {
        machine: "web".to_string(),
        resources_converged: 5,
        resources_unchanged: 2,
        resources_failed: 0,
        total_duration: std::time::Duration::from_secs(3),
        resource_reports: Vec::new(),
    };
    let debug = format!("{:?}", ar);
    assert!(debug.contains("web"));
    assert!(debug.contains("5"));
}

#[test]
fn test_fj132_yaml_value_to_string_null() {
    let val = serde_yaml_ng::Value::Null;
    assert_eq!(yaml_value_to_string(&val), "");
}

#[test]
fn test_fj132_yaml_value_to_string_bool() {
    let val = serde_yaml_ng::Value::Bool(true);
    assert_eq!(yaml_value_to_string(&val), "true");
    let val = serde_yaml_ng::Value::Bool(false);
    assert_eq!(yaml_value_to_string(&val), "false");
}
