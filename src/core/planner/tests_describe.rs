use super::*;
use super::tests_helpers::make_base_resource;

#[test]
fn test_fj004_describe_action() {
    let mut r = make_base_resource(ResourceType::Package);
    r.provider = Some("apt".to_string());
    r.packages = vec!["curl".to_string(), "wget".to_string()];
    let desc = describe_action("test-pkg", &r, &PlanAction::Create);
    assert!(desc.contains("curl, wget"));
}

#[test]
fn test_fj004_describe_action_file() {
    let mut r = make_base_resource(ResourceType::File);
    r.path = Some("/etc/conf".to_string());
    assert!(describe_action("f", &r, &PlanAction::Create).contains("/etc/conf"));
    assert!(describe_action("f", &r, &PlanAction::Update).contains("update"));
    assert!(describe_action("f", &r, &PlanAction::Destroy).contains("destroy"));
    assert!(describe_action("f", &r, &PlanAction::NoOp).contains("no changes"));
}

#[test]
fn test_fj004_describe_action_service() {
    let mut r = make_base_resource(ResourceType::Service);
    r.name = Some("nginx".to_string());
    assert!(describe_action("svc", &r, &PlanAction::Create).contains("nginx"));
}

#[test]
fn test_fj004_describe_action_mount() {
    let mut r = make_base_resource(ResourceType::Mount);
    r.path = Some("/mnt/data".to_string());
    assert!(describe_action("mnt", &r, &PlanAction::Create).contains("/mnt/data"));
}

#[test]
fn test_fj004_describe_action_file_no_path() {
    let r = make_base_resource(ResourceType::File);
    let desc = describe_action("f", &r, &PlanAction::Create);
    assert!(desc.contains("?"), "missing path should show ?");
}

#[test]
fn test_fj004_describe_action_service_no_name() {
    let r = make_base_resource(ResourceType::Service);
    let desc = describe_action("svc", &r, &PlanAction::Create);
    assert!(desc.contains("?"), "missing name should show ?");
}

#[test]
fn test_fj004_describe_action_docker_type() {
    let r = make_base_resource(ResourceType::Docker);
    let desc = describe_action("dock", &r, &PlanAction::Create);
    assert!(desc.contains("create"), "Docker create should say create");
}

#[test]
fn test_fj132_describe_action_user_type() {
    let r = make_base_resource(ResourceType::User);
    let desc = describe_action("myuser", &r, &PlanAction::Create);
    assert_eq!(desc, "myuser: create");
}

#[test]
fn test_fj132_describe_action_cron_type() {
    let r = make_base_resource(ResourceType::Cron);
    let desc = describe_action("backup-job", &r, &PlanAction::Create);
    assert_eq!(desc, "backup-job: create");
}

#[test]
fn test_fj132_describe_action_network_type() {
    let r = make_base_resource(ResourceType::Network);
    let desc = describe_action("fw-rule", &r, &PlanAction::Create);
    assert_eq!(desc, "fw-rule: create");
}
