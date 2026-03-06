//! Coverage tests for cli/plan.rs — cmd_plan, cmd_plan_compact, print_plan_cost, what-if, why.

fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    std::io::Write::write_all(&mut f, yaml.as_bytes()).unwrap();
    std::io::Write::flush(&mut f).unwrap();
    f
}

const CONFIG: &str = "version: '1.0'\nname: test\nmachines:\n  web1:\n    hostname: web1\n    addr: 127.0.0.1\nresources:\n  nginx:\n    type: package\n    provider: apt\n    packages:\n      - nginx\n  my-config:\n    type: file\n    path: /etc/app.conf\n    content: hello\n    requires:\n      - nginx\n";

const CONFIG_TAGGED: &str = "version: '1.0'\nname: test\nmachines:\n  web1:\n    hostname: web1\n    addr: 127.0.0.1\nresources:\n  nginx:\n    type: package\n    provider: apt\n    packages:\n      - nginx\n    tags:\n      - web\n  redis:\n    type: package\n    provider: apt\n    packages:\n      - redis-server\n    tags:\n      - cache\n";

const CONFIG_PARAMS: &str = "version: '1.0'\nname: test\nparams:\n  env: staging\nmachines:\n  web1:\n    hostname: web1\n    addr: 127.0.0.1\nresources:\n  nginx:\n    type: package\n    provider: apt\n    packages:\n      - nginx\n";

// ── cmd_plan ──

#[test]
fn plan_basic_text() {
    let cfg = write_temp_config(CONFIG);
    let d = tempfile::tempdir().unwrap();
    let r = super::plan::cmd_plan(
        cfg.path(), d.path(), None, None, None,
        false, false, None, None, None, false, None, false, &[], None, false,
    );
    assert!(r.is_ok());
}

#[test]
fn plan_basic_json() {
    let cfg = write_temp_config(CONFIG);
    let d = tempfile::tempdir().unwrap();
    let r = super::plan::cmd_plan(
        cfg.path(), d.path(), None, None, None,
        true, false, None, None, None, false, None, false, &[], None, false,
    );
    assert!(r.is_ok());
}

#[test]
fn plan_verbose() {
    let cfg = write_temp_config(CONFIG);
    let d = tempfile::tempdir().unwrap();
    let r = super::plan::cmd_plan(
        cfg.path(), d.path(), None, None, None,
        false, true, None, None, None, false, None, false, &[], None, false,
    );
    assert!(r.is_ok());
}

#[test]
fn plan_no_diff() {
    let cfg = write_temp_config(CONFIG);
    let d = tempfile::tempdir().unwrap();
    let r = super::plan::cmd_plan(
        cfg.path(), d.path(), None, None, None,
        false, false, None, None, None, true, None, false, &[], None, false,
    );
    assert!(r.is_ok());
}

#[test]
fn plan_with_cost() {
    let cfg = write_temp_config(CONFIG);
    let d = tempfile::tempdir().unwrap();
    let r = super::plan::cmd_plan(
        cfg.path(), d.path(), None, None, None,
        false, false, None, None, None, false, None, true, &[], None, false,
    );
    assert!(r.is_ok());
}

#[test]
fn plan_with_why() {
    let cfg = write_temp_config(CONFIG);
    let d = tempfile::tempdir().unwrap();
    let r = super::plan::cmd_plan(
        cfg.path(), d.path(), None, None, None,
        false, false, None, None, None, false, None, false, &[], None, true,
    );
    assert!(r.is_ok());
}

#[test]
fn plan_tag_filter() {
    let cfg = write_temp_config(CONFIG_TAGGED);
    let d = tempfile::tempdir().unwrap();
    let r = super::plan::cmd_plan(
        cfg.path(), d.path(), None, None, Some("web"),
        false, false, None, None, None, false, None, false, &[], None, false,
    );
    assert!(r.is_ok());
}

#[test]
fn plan_target_resource() {
    let cfg = write_temp_config(CONFIG);
    let d = tempfile::tempdir().unwrap();
    let r = super::plan::cmd_plan(
        cfg.path(), d.path(), None, None, None,
        false, false, None, None, None, false, Some("nginx"), false, &[], None, false,
    );
    assert!(r.is_ok());
}

#[test]
fn plan_target_with_deps() {
    let cfg = write_temp_config(CONFIG);
    let d = tempfile::tempdir().unwrap();
    let r = super::plan::cmd_plan(
        cfg.path(), d.path(), None, None, None,
        false, false, None, None, None, false, Some("my-config"), false, &[], None, false,
    );
    assert!(r.is_ok());
}

#[test]
fn plan_what_if() {
    let cfg = write_temp_config(CONFIG_PARAMS);
    let d = tempfile::tempdir().unwrap();
    let what_if = vec!["env=production".to_string()];
    let r = super::plan::cmd_plan(
        cfg.path(), d.path(), None, None, None,
        false, false, None, None, None, false, None, false, &what_if, None, false,
    );
    assert!(r.is_ok());
}

#[test]
fn plan_what_if_invalid() {
    let cfg = write_temp_config(CONFIG);
    let d = tempfile::tempdir().unwrap();
    let what_if = vec!["bad-format-no-equals".to_string()];
    let r = super::plan::cmd_plan(
        cfg.path(), d.path(), None, None, None,
        false, false, None, None, None, false, None, false, &what_if, None, false,
    );
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("--what-if"));
}

#[test]
fn plan_save_to_file() {
    let cfg = write_temp_config(CONFIG);
    let d = tempfile::tempdir().unwrap();
    let plan_out = d.path().join("plan.json");
    let r = super::plan::cmd_plan(
        cfg.path(), d.path(), None, None, None,
        false, false, None, None, None, false, None, false, &[], Some(&plan_out), false,
    );
    assert!(r.is_ok());
    assert!(plan_out.exists());
}

#[test]
fn plan_missing_config() {
    let d = tempfile::tempdir().unwrap();
    let r = super::plan::cmd_plan(
        std::path::Path::new("/nonexistent/f.yaml"), d.path(), None, None, None,
        false, false, None, None, None, false, None, false, &[], None, false,
    );
    assert!(r.is_err());
}

// ── cmd_plan_compact ──

#[test]
fn plan_compact_text() {
    let cfg = write_temp_config(CONFIG);
    let d = tempfile::tempdir().unwrap();
    let r = super::plan::cmd_plan_compact(cfg.path(), d.path(), None, false);
    assert!(r.is_ok());
}

#[test]
fn plan_compact_json() {
    let cfg = write_temp_config(CONFIG);
    let d = tempfile::tempdir().unwrap();
    let r = super::plan::cmd_plan_compact(cfg.path(), d.path(), None, true);
    assert!(r.is_ok());
}

#[test]
fn plan_compact_machine_filter() {
    let cfg = write_temp_config(CONFIG);
    let d = tempfile::tempdir().unwrap();
    let r = super::plan::cmd_plan_compact(cfg.path(), d.path(), Some("web1"), false);
    assert!(r.is_ok());
}

// ── print_plan_cost ──

#[test]
fn plan_cost_empty() {
    let plan = crate::core::types::ExecutionPlan {
        name: "test".to_string(),
        changes: vec![],
        execution_order: vec![],
        to_create: 0, to_update: 0, to_destroy: 0, unchanged: 0,
    };
    super::plan::print_plan_cost(&plan);
}

#[test]
fn plan_cost_with_changes() {
    use crate::core::types::*;
    let plan = ExecutionPlan {
        name: "test".to_string(),
        changes: vec![
            PlannedChange {
                resource_id: "pkg".to_string(),
                machine: "m1".to_string(),
                resource_type: ResourceType::Package,
                action: PlanAction::Create,
                description: "install".to_string(),
            },
            PlannedChange {
                resource_id: "svc".to_string(),
                machine: "m1".to_string(),
                resource_type: ResourceType::Service,
                action: PlanAction::Destroy,
                description: "remove".to_string(),
            },
        ],
        execution_order: vec!["pkg".to_string(), "svc".to_string()],
        to_create: 1, to_update: 0, to_destroy: 1, unchanged: 0,
    };
    super::plan::print_plan_cost(&plan);
}

#[test]
fn plan_cost_high_destroy() {
    use crate::core::types::*;
    let mut changes = Vec::new();
    for i in 0..5 {
        changes.push(PlannedChange {
            resource_id: format!("docker-{i}"),
            machine: "m1".to_string(),
            resource_type: ResourceType::Docker,
            action: PlanAction::Destroy,
            description: "destroy".to_string(),
        });
    }
    let plan = ExecutionPlan {
        name: "test".to_string(),
        changes,
        execution_order: vec![],
        to_create: 0, to_update: 0, to_destroy: 5, unchanged: 0,
    };
    // Should print high destructive cost warning
    super::plan::print_plan_cost(&plan);
}
