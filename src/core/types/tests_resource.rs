//! Tests for Resource, ResourceType, MachineTarget (FJ-001, FJ-131, FJ-132, FJ-142).

use super::*;

#[test]
fn test_fj001_machine_target_single() {
    let t = MachineTarget::Single("lambda".to_string());
    assert_eq!(t.to_vec(), vec!["lambda"]);
}

#[test]
fn test_fj001_machine_target_multiple() {
    let yaml = r#"[intel, jetson]"#;
    let t: MachineTarget = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(t.to_vec(), vec!["intel", "jetson"]);
}

#[test]
fn test_fj001_resource_type_display() {
    assert_eq!(ResourceType::Package.to_string(), "package");
    assert_eq!(ResourceType::Service.to_string(), "service");
    assert_eq!(ResourceType::Mount.to_string(), "mount");
}

#[test]
fn test_fj001_resource_status_display() {
    assert_eq!(ResourceStatus::Converged.to_string(), "CONVERGED");
    assert_eq!(ResourceStatus::Failed.to_string(), "FAILED");
    assert_eq!(ResourceStatus::Drifted.to_string(), "DRIFTED");
    assert_eq!(ResourceStatus::Unknown.to_string(), "UNKNOWN");
}

#[test]
fn test_fj001_plan_action_display() {
    assert_eq!(PlanAction::Create.to_string(), "CREATE");
    assert_eq!(PlanAction::Update.to_string(), "UPDATE");
    assert_eq!(PlanAction::Destroy.to_string(), "DESTROY");
    assert_eq!(PlanAction::NoOp.to_string(), "NO-OP");
}

#[test]
fn test_fj001_resource_type_display_all() {
    assert_eq!(ResourceType::File.to_string(), "file");
    assert_eq!(ResourceType::User.to_string(), "user");
    assert_eq!(ResourceType::Docker.to_string(), "docker");
    assert_eq!(ResourceType::Pepita.to_string(), "pepita");
    assert_eq!(ResourceType::Network.to_string(), "network");
    assert_eq!(ResourceType::Cron.to_string(), "cron");
}

#[test]
fn test_fj131_machine_target_default() {
    let t = MachineTarget::default();
    assert_eq!(t.to_vec(), vec!["localhost"]);
}

#[test]
fn test_fj131_machine_target_multiple_empty() {
    let t = MachineTarget::Multiple(vec![]);
    assert!(t.to_vec().is_empty());
}

#[test]
fn test_fj131_resource_type_recipe_display() {
    assert_eq!(ResourceType::Recipe.to_string(), "recipe");
}

#[test]
fn test_fj131_resource_type_serde_roundtrip() {
    let types = [
        ResourceType::Package,
        ResourceType::File,
        ResourceType::Service,
        ResourceType::Mount,
        ResourceType::User,
        ResourceType::Docker,
        ResourceType::Pepita,
        ResourceType::Network,
        ResourceType::Cron,
        ResourceType::Recipe,
    ];
    for rt in &types {
        let json = serde_json::to_string(rt).unwrap();
        let back: ResourceType = serde_json::from_str(&json).unwrap();
        assert_eq!(&back, rt, "roundtrip failed for {rt:?}");
    }
}

#[test]
fn test_fj131_failure_policy_serde() {
    let yaml_stop = "\"stop_on_first\"";
    let fp: FailurePolicy = serde_yaml_ng::from_str(yaml_stop).unwrap();
    assert_eq!(fp, FailurePolicy::StopOnFirst);

    let yaml_cont = "\"continue_independent\"";
    let fp2: FailurePolicy = serde_yaml_ng::from_str(yaml_cont).unwrap();
    assert_eq!(fp2, FailurePolicy::ContinueIndependent);
}

#[test]
fn test_fj131_failure_policy_default() {
    let fp = FailurePolicy::default();
    assert_eq!(fp, FailurePolicy::StopOnFirst);
}

#[test]
fn test_fj131_resource_all_fields_roundtrip() {
    let yaml = r#"
version: "1.0"
name: all-fields
machines:
  m:
    hostname: m
    addr: 1.1.1.1
resources:
  full-file:
    type: file
    machine: m
    state: file
    path: /etc/app.conf
    content: "key=val"
    owner: www-data
    group: www-data
    mode: "0600"
    depends_on: []
    arch: [x86_64, aarch64]
    tags: [web, critical]
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let r = &config.resources["full-file"];
    assert_eq!(r.resource_type, ResourceType::File);
    assert_eq!(r.state.as_deref(), Some("file"));
    assert_eq!(r.owner.as_deref(), Some("www-data"));
    assert_eq!(r.mode.as_deref(), Some("0600"));
    assert_eq!(r.arch, vec!["x86_64", "aarch64"]);
    assert_eq!(r.tags, vec!["web", "critical"]);
}

#[test]
fn test_fj132_machine_target_to_vec_single() {
    let t = MachineTarget::Single("web".to_string());
    assert_eq!(t.to_vec(), vec!["web".to_string()]);
}

#[test]
fn test_fj132_machine_target_to_vec_multiple() {
    let t = MachineTarget::Multiple(vec!["web".into(), "db".into(), "cache".into()]);
    assert_eq!(t.to_vec(), vec!["web", "db", "cache"]);
}

#[test]
fn test_fj132_resource_type_clone() {
    let rt = ResourceType::Docker;
    let cloned = rt.clone();
    assert_eq!(format!("{rt}"), format!("{}", cloned));
}

#[test]
fn test_fj132_resource_status_all_variants_display() {
    // Verify all four variants have non-empty Display output
    for status in &[
        ResourceStatus::Converged,
        ResourceStatus::Failed,
        ResourceStatus::Drifted,
        ResourceStatus::Unknown,
    ] {
        let s = format!("{status}");
        assert!(!s.is_empty(), "ResourceStatus display should not be empty");
    }
}

#[test]
fn test_fj132_plan_action_all_variants() {
    // Verify all four variants have non-empty Display output
    for action in &[
        PlanAction::Create,
        PlanAction::Update,
        PlanAction::Destroy,
        PlanAction::NoOp,
    ] {
        let s = format!("{action}");
        assert!(!s.is_empty(), "PlanAction display should not be empty");
    }
}

#[test]
fn test_fj132_resource_defaults() {
    // A resource with minimal fields should have sensible defaults
    let yaml = r#"
type: file
machine: m1
path: /tmp/test
"#;
    let r: Resource = serde_yaml_ng::from_str(yaml).unwrap();
    assert!(r.packages.is_empty());
    assert!(r.depends_on.is_empty());
    assert!(r.restart_on.is_empty());
    assert!(r.tags.is_empty());
    assert!(r.arch.is_empty());
    assert!(r.ports.is_empty());
    assert!(r.volumes.is_empty());
    assert!(r.environment.is_empty());
    assert!(r.ssh_authorized_keys.is_empty());
    assert!(r.groups.is_empty());
}

#[test]
fn test_fj132_resource_type_display_all() {
    let types = [
        (ResourceType::Package, "package"),
        (ResourceType::File, "file"),
        (ResourceType::Service, "service"),
        (ResourceType::Mount, "mount"),
        (ResourceType::User, "user"),
        (ResourceType::Docker, "docker"),
        (ResourceType::Cron, "cron"),
        (ResourceType::Network, "network"),
    ];
    for (rt, expected) in &types {
        assert_eq!(format!("{rt}"), *expected);
    }
}

#[test]
fn test_fj132_machine_target_single_deserialization() {
    let yaml = "machine: web";
    let r: Resource =
        serde_yaml_ng::from_str(&format!("type: file\n{yaml}\npath: /tmp/x")).unwrap();
    match &r.machine {
        MachineTarget::Single(name) => assert_eq!(name, "web"),
        MachineTarget::Multiple(_) => panic!("expected Single"),
    }
}

#[test]
fn test_fj132_machine_target_multiple_deserialization() {
    let yaml = "type: file\nmachine: [web, db]\npath: /tmp/x";
    let r: Resource = serde_yaml_ng::from_str(yaml).unwrap();
    match &r.machine {
        MachineTarget::Multiple(names) => {
            assert_eq!(names.len(), 2);
            assert_eq!(names[0], "web");
            assert_eq!(names[1], "db");
        }
        MachineTarget::Single(_) => panic!("expected Multiple"),
    }
}

// ── FJ-142: Display + PartialEq for MachineTarget/FailurePolicy ──

#[test]
fn test_fj142_machine_target_display_single() {
    let t = MachineTarget::Single("web1".to_string());
    assert_eq!(format!("{t}"), "web1");
}

#[test]
fn test_fj142_machine_target_display_multiple() {
    let t = MachineTarget::Multiple(vec!["web1".to_string(), "web2".to_string()]);
    assert_eq!(format!("{t}"), "[web1, web2]");
}

#[test]
fn test_fj142_machine_target_display_empty_multiple() {
    let t = MachineTarget::Multiple(vec![]);
    assert_eq!(format!("{t}"), "[]");
}

#[test]
fn test_fj142_machine_target_partial_eq() {
    let a = MachineTarget::Single("web".to_string());
    let b = MachineTarget::Single("web".to_string());
    let c = MachineTarget::Single("db".to_string());
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn test_fj142_machine_target_eq_multiple() {
    let a = MachineTarget::Multiple(vec!["a".to_string(), "b".to_string()]);
    let b = MachineTarget::Multiple(vec!["a".to_string(), "b".to_string()]);
    let c = MachineTarget::Multiple(vec!["b".to_string(), "a".to_string()]);
    assert_eq!(a, b);
    assert_ne!(a, c); // order matters
}

#[test]
fn test_fj142_failure_policy_display_stop() {
    assert_eq!(format!("{}", FailurePolicy::StopOnFirst), "stop_on_first");
}

#[test]
fn test_fj142_failure_policy_display_continue() {
    assert_eq!(
        format!("{}", FailurePolicy::ContinueIndependent),
        "continue_independent"
    );
}
