//! FJ-3010: Selective force locks tests.

#![allow(unused_imports)]
use super::test_fixtures::*;
use super::*;

fn make_test_lock(machine: &str, resource_ids: &[&str]) -> StateLock {
    let mut resources = indexmap::IndexMap::new();
    for rid in resource_ids {
        resources.insert(
            rid.to_string(),
            ResourceLock {
                resource_type: ResourceType::File,
                hash: format!("hash-{rid}"),
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                details: std::collections::HashMap::new(),
            },
        );
    }
    StateLock {
        schema: "1".to_string(),
        machine: machine.to_string(),
        hostname: machine.to_string(),
        generated_at: String::new(),
        generator: "test".to_string(),
        blake3_version: "1".to_string(),
        resources,
    }
}

#[test]
fn test_fj3010_selective_force_locks_strips_tagged() {
    let yaml = r#"
version: "1.0"
name: force-tag
machines:
  m1:
    hostname: m
    addr: 127.0.0.1
resources:
  build-app:
    type: task
    machine: m1
    command: "make build"
    tags: [build]
  serve-app:
    type: task
    machine: m1
    command: "systemctl start app"
    tags: [service]
  config-file:
    type: file
    machine: m1
    path: /etc/app.conf
    content: "key=val"
    tags: [config]
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let mut locks = HashMap::new();
    locks.insert(
        "m1".to_string(),
        make_test_lock("m1", &["build-app", "serve-app", "config-file"]),
    );

    // Force only "service" tagged resources
    let result = selective_force_locks(&locks, &config, "service");
    let m1_lock = &result["m1"];
    // serve-app should be stripped (forced)
    assert!(
        !m1_lock.resources.contains_key("serve-app"),
        "serve-app should be stripped by --force-tag service"
    );
    // build-app and config-file should remain
    assert!(m1_lock.resources.contains_key("build-app"));
    assert!(m1_lock.resources.contains_key("config-file"));
}

#[test]
fn test_fj3010_selective_force_no_match_keeps_all() {
    let yaml = r#"
version: "1.0"
name: force-tag-none
machines:
  m1:
    hostname: m
    addr: 127.0.0.1
resources:
  app:
    type: file
    machine: m1
    path: /tmp/test
    content: "x"
    tags: [web]
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let mut locks = HashMap::new();
    locks.insert("m1".to_string(), make_test_lock("m1", &["app"]));
    // Force "nonexistent" tag — should keep all locks
    let result = selective_force_locks(&locks, &config, "nonexistent");
    assert!(result["m1"].resources.contains_key("app"));
}
