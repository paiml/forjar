//! Edge-case tests: record_success and record_failure variants.

use super::test_fixtures::*;
use super::*;

// ── Edge-case tests ────────────────────────────────────────────────

#[test]
fn test_fj012_record_success_without_tripwire() {
    let dir = tempfile::tempdir().unwrap();
    let managed_file = dir.path().join("no-trip.txt");
    std::fs::write(&managed_file, "content").unwrap();
    let mut lock = state::new_lock("test", "test-box");
    let resource = Resource {
        resource_type: ResourceType::File,
        machine: MachineTarget::Single("test".to_string()),
        path: Some(managed_file.to_str().unwrap().to_string()),
        content: Some("content".to_string()),
        ..Default::default()
    };
    let mut ctx = RecordCtx {
        lock: &mut lock,
        state_dir: dir.path(),
        machine_name: "test",
        tripwire: false,
        failure_policy: &FailurePolicy::StopOnFirst,
        timeout_secs: None,
    };

    record_success(&mut ctx, "f", &resource, &resource, &local_machine(), 0.5);

    assert_eq!(ctx.lock.resources["f"].status, ResourceStatus::Converged);
    // No event log should be written when tripwire is off
    let events_path = dir.path().join("test").join("events.jsonl");
    assert!(!events_path.exists(), "no event log without tripwire");
}

#[test]
fn test_fj012_record_success_service_resource() {
    let dir = tempfile::tempdir().unwrap();
    let mut lock = state::new_lock("test", "test-box");
    let resource = Resource {
        resource_type: ResourceType::Service,
        machine: MachineTarget::Single("test".to_string()),
        name: Some("nginx".to_string()),
        state: Some("running".to_string()),
        enabled: Some(true),
        ..Default::default()
    };
    let mut ctx = RecordCtx {
        lock: &mut lock,
        state_dir: dir.path(),
        machine_name: "test",
        tripwire: false,
        failure_policy: &FailurePolicy::StopOnFirst,
        timeout_secs: None,
    };

    record_success(&mut ctx, "svc", &resource, &resource, &local_machine(), 1.5);

    let rl = &ctx.lock.resources["svc"];
    assert_eq!(rl.status, ResourceStatus::Converged);
    assert!(rl.details.contains_key("service_name"));
    assert_eq!(
        rl.details["service_name"],
        serde_yaml_ng::Value::String("nginx".to_string())
    );
}

#[test]
fn test_fj012_record_failure_tripwire_off() {
    let dir = tempfile::tempdir().unwrap();
    let mut lock = state::new_lock("test", "test-box");
    let mut ctx = RecordCtx {
        lock: &mut lock,
        state_dir: dir.path(),
        machine_name: "test",
        tripwire: false,
        failure_policy: &FailurePolicy::ContinueIndependent,
        timeout_secs: None,
    };

    record_failure(&mut ctx, "f", &ResourceType::File, 0.1, "broke");

    assert_eq!(ctx.lock.resources["f"].status, ResourceStatus::Failed);
    let events_path = dir.path().join("test").join("events.jsonl");
    assert!(!events_path.exists(), "no event log without tripwire");
}

#[test]
fn test_fj012_build_details_file_no_content() {
    // File with path but no content → no content_hash
    let resource = Resource {
        resource_type: ResourceType::File,
        machine: MachineTarget::Single("test".to_string()),
        path: Some("/etc/some.conf".to_string()),
        owner: Some("root".to_string()),
        mode: Some("0644".to_string()),
        ..Default::default()
    };
    let details = build_resource_details(&resource, &local_machine());
    assert!(details.contains_key("path"));
    assert!(details.contains_key("owner"));
    assert!(details.contains_key("mode"));
    assert!(
        !details.contains_key("content_hash"),
        "no content → no hash"
    );
}

#[test]
fn test_fj012_build_details_file_with_content_and_real_file() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("real.txt");
    std::fs::write(&file_path, "real content").unwrap();

    let resource = Resource {
        resource_type: ResourceType::File,
        machine: MachineTarget::Single("test".to_string()),
        path: Some(file_path.to_str().unwrap().to_string()),
        content: Some("real content".to_string()),
        ..Default::default()
    };
    let details = build_resource_details(&resource, &local_machine());
    assert!(
        details.contains_key("content_hash"),
        "real file should have content_hash"
    );
    let hash = details["content_hash"].as_str().unwrap();
    assert!(
        hash.starts_with("blake3:"),
        "hash should be blake3-prefixed"
    );
}
