//! Tests for task resource handler — batch, pipeline, service, dispatch modes.

use super::task::*;
use crate::core::types::{MachineTarget, Resource, ResourceType, TaskMode};

fn make_task_resource(cmd: &str) -> Resource {
    Resource {
        resource_type: ResourceType::Task,
        machine: MachineTarget::Single("worker".to_string()),
        command: Some(cmd.to_string()),
        ..Default::default()
    }
}

#[test]
fn test_check_no_completion_check_no_artifacts() {
    let r = make_task_resource("echo hello");
    let script = check_script(&r);
    assert_eq!(script, "echo 'task=pending'");
}

#[test]
fn test_check_with_completion_check() {
    let mut r = make_task_resource("train model");
    r.completion_check = Some("test -f model.bin".to_string());
    let script = check_script(&r);
    assert!(script.contains("test -f model.bin"));
    assert!(script.contains("task=completed"));
    assert!(script.contains("task=pending"));
}

#[test]
fn test_check_with_output_artifacts() {
    let mut r = make_task_resource("build");
    r.output_artifacts = vec!["out/model.bin".to_string(), "out/vocab.json".to_string()];
    let script = check_script(&r);
    assert!(script.contains("[ -e 'out/model.bin' ]"));
    assert!(script.contains("[ -e 'out/vocab.json' ]"));
    assert!(script.contains("task=completed"));
}

#[test]
fn test_apply_basic() {
    let r = make_task_resource("apr train apply config.yaml");
    let script = apply_script(&r);
    assert!(script.contains("set -euo pipefail"));
    assert!(script.contains("apr train apply config.yaml"));
}

#[test]
fn test_apply_with_working_dir() {
    let mut r = make_task_resource("make build");
    r.working_dir = Some("/opt/project".to_string());
    let script = apply_script(&r);
    assert!(script.contains("cd '/opt/project'"));
    assert!(script.contains("make build"));
}

#[test]
fn test_apply_with_timeout() {
    let mut r = make_task_resource("long-running-train");
    r.timeout = Some(3600);
    let script = apply_script(&r);
    assert!(script.contains("timeout 3600 bash <<'FORJAR_TIMEOUT'"));
    assert!(script.contains("long-running-train"));
    assert!(script.contains("FORJAR_TIMEOUT"));
}

#[test]
fn test_apply_with_timeout_multiline() {
    let mut r = make_task_resource("git pull\ncargo build");
    r.timeout = Some(300);
    r.working_dir = Some("/opt/project".to_string());
    let script = apply_script(&r);
    assert!(script.contains("timeout 300 bash <<'FORJAR_TIMEOUT'"));
    assert!(script.contains("git pull\ncargo build"));
    assert!(script.contains("cd '/opt/project'"));
}

#[test]
fn test_apply_with_timeout_quoting() {
    let mut r = make_task_resource("echo 'hello world'");
    r.timeout = Some(60);
    let script = apply_script(&r);
    assert!(script.contains("echo 'hello world'"));
    assert!(script.contains("timeout 60 bash <<'FORJAR_TIMEOUT'"));
}

#[test]
fn test_state_query_with_artifacts() {
    let mut r = make_task_resource("train");
    r.output_artifacts = vec!["model.bin".to_string()];
    let script = state_query_script(&r);
    assert!(script.contains("b3sum 'model.bin'"));
    assert!(script.contains("missing:model.bin"));
}

#[test]
fn test_state_query_no_artifacts() {
    let r = make_task_resource("echo hello");
    let script = state_query_script(&r);
    assert!(script.contains("command=echo hello"));
}

#[test]
fn test_apply_no_command_defaults_to_true() {
    let mut r = make_task_resource("placeholder");
    r.command = None;
    let script = apply_script(&r);
    assert!(script.contains("true"));
}

#[test]
fn test_scatter_empty() {
    let r = make_task_resource("train");
    assert!(scatter_script(&r).is_none());
}

#[test]
fn test_scatter_with_mappings() {
    let mut r = make_task_resource("train");
    r.scatter = vec![
        "/local/data.csv:/remote/data.csv".to_string(),
        "/local/config.yaml:/remote/config.yaml".to_string(),
    ];
    let script = scatter_script(&r).unwrap();
    assert!(script.contains("cp -r '/local/data.csv' '/remote/data.csv'"));
    assert!(script.contains("cp -r '/local/config.yaml' '/remote/config.yaml'"));
    assert!(script.contains("mkdir -p"));
}

#[test]
fn test_gather_empty() {
    let r = make_task_resource("train");
    assert!(gather_script(&r).is_none());
}

#[test]
fn test_gather_with_mappings() {
    let mut r = make_task_resource("train");
    r.gather = vec!["/remote/model.bin:/local/model.bin".to_string()];
    let script = gather_script(&r).unwrap();
    assert!(script.contains("cp -r '/remote/model.bin' '/local/model.bin'"));
    assert!(script.contains("# FJ-2704: gather artifacts"));
}

#[test]
fn test_apply_with_scatter_and_gather() {
    let mut r = make_task_resource("python train.py");
    r.scatter = vec!["/data/input.csv:/remote/input.csv".to_string()];
    r.gather = vec!["/remote/model.bin:/local/model.bin".to_string()];
    let script = apply_script(&r);
    let scatter_pos = script.find("scatter artifacts").unwrap();
    let cmd_pos = script.find("python train.py").unwrap();
    let gather_pos = script.find("gather artifacts").unwrap();
    assert!(scatter_pos < cmd_pos, "scatter must run before command");
    assert!(cmd_pos < gather_pos, "gather must run after command");
}

#[test]
fn test_apply_scatter_only() {
    let mut r = make_task_resource("train");
    r.scatter = vec!["/a:/b".to_string()];
    let script = apply_script(&r);
    assert!(script.contains("scatter artifacts"));
    assert!(!script.contains("gather artifacts"));
}

#[test]
fn test_apply_gather_only() {
    let mut r = make_task_resource("train");
    r.gather = vec!["/a:/b".to_string()];
    let script = apply_script(&r);
    assert!(!script.contains("scatter artifacts"));
    assert!(script.contains("gather artifacts"));
}

#[test]
fn test_scatter_invalid_mapping_skipped() {
    let mut r = make_task_resource("train");
    r.scatter = vec!["no-colon-here".to_string()];
    let script = scatter_script(&r).unwrap();
    assert!(!script.contains("cp"));
}

#[test]
fn test_pipeline_stages_basic() {
    use crate::core::types::PipelineStage;
    let mut r = make_task_resource("ignored");
    r.stages = vec![
        PipelineStage {
            name: "lint".into(),
            command: Some("cargo clippy".into()),
            gate: false,
            ..Default::default()
        },
        PipelineStage {
            name: "test".into(),
            command: Some("cargo test".into()),
            gate: true,
            ..Default::default()
        },
    ];
    let script = apply_script(&r);
    assert!(script.contains("=== Stage: lint ==="));
    assert!(script.contains("=== Stage: test ==="));
    assert!(script.contains("cargo clippy"));
    assert!(script.contains("cargo test"));
}

#[test]
fn test_pipeline_gate_enforcement() {
    use crate::core::types::PipelineStage;
    let mut r = make_task_resource("ignored");
    r.stages = vec![PipelineStage {
        name: "qa-gate".into(),
        command: Some("check_quality".into()),
        gate: true,
        ..Default::default()
    }];
    let script = apply_script(&r);
    assert!(script.contains("GATE FAILED: qa-gate"));
    assert!(script.contains("exit 1"));
}

#[test]
fn test_pipeline_non_gate_no_abort() {
    use crate::core::types::PipelineStage;
    let mut r = make_task_resource("ignored");
    r.stages = vec![PipelineStage {
        name: "optional".into(),
        command: Some("echo optional".into()),
        gate: false,
        ..Default::default()
    }];
    let script = apply_script(&r);
    assert!(script.contains("echo optional"));
    assert!(!script.contains("GATE FAILED"));
}

#[test]
fn test_pipeline_with_working_dir() {
    use crate::core::types::PipelineStage;
    let mut r = make_task_resource("ignored");
    r.working_dir = Some("/opt/ml".into());
    r.stages = vec![PipelineStage {
        name: "build".into(),
        command: Some("make".into()),
        gate: false,
        ..Default::default()
    }];
    let script = apply_script(&r);
    assert!(script.contains("cd '/opt/ml'"));
}

#[test]
fn test_pipeline_stages_override_command() {
    use crate::core::types::PipelineStage;
    let mut r = make_task_resource("this-is-ignored");
    r.stages = vec![PipelineStage {
        name: "s1".into(),
        command: Some("echo stage1".into()),
        ..Default::default()
    }];
    let script = apply_script(&r);
    assert!(!script.contains("this-is-ignored"));
    assert!(script.contains("echo stage1"));
}

// ── FJ-2700/E21: Mode-aware script tests ──

#[test]
fn test_service_mode_apply_script() {
    let mut r = make_task_resource("apr serve --port 8080");
    r.task_mode = Some(TaskMode::Service);
    r.name = Some("inference".into());
    let script = apply_script(&r);
    assert!(script.contains("nohup"), "service must background");
    assert!(script.contains("forjar-svc-inference.pid"), "PID file");
    assert!(script.contains("already running"), "idempotent check");
    assert!(script.contains("FORJAR_SVC_PID"));
}

#[test]
fn test_service_mode_with_health_check() {
    let mut r = make_task_resource("python server.py");
    r.task_mode = Some(TaskMode::Service);
    r.name = Some("api".into());
    r.health_check = Some(crate::core::types::HealthCheck {
        command: "curl -sf http://localhost:8080/health".into(),
        timeout: Some("5s".into()),
        retries: Some(3),
        ..Default::default()
    });
    let script = apply_script(&r);
    assert!(script.contains("curl -sf"), "health check command");
    assert!(script.contains("seq 1 3"), "retry loop");
    assert!(script.contains("healthy"), "health status output");
}

#[test]
fn test_service_mode_check_script() {
    let mut r = make_task_resource("server");
    r.task_mode = Some(TaskMode::Service);
    r.name = Some("web".into());
    let script = check_script(&r);
    assert!(script.contains("forjar-svc-web.pid"), "PID file check");
    assert!(script.contains("kill -0"), "process liveness check");
}

#[test]
fn test_dispatch_mode_apply_script() {
    let mut r = make_task_resource("deploy.sh v1.0");
    r.task_mode = Some(TaskMode::Dispatch);
    let script = apply_script(&r);
    assert!(script.contains("deploy.sh v1.0"));
    assert!(!script.contains("DISPATCH BLOCKED"));
}

#[test]
fn test_dispatch_mode_with_gate() {
    use crate::core::types::QualityGate;
    let mut r = make_task_resource("deploy.sh v1.0");
    r.task_mode = Some(TaskMode::Dispatch);
    r.quality_gate = Some(QualityGate {
        command: Some("test -f /ready.flag".into()),
        message: Some("Not ready for deploy".into()),
        ..Default::default()
    });
    let script = apply_script(&r);
    assert!(script.contains("test -f /ready.flag"), "gate command");
    assert!(
        script.contains("DISPATCH BLOCKED: Not ready for deploy"),
        "gate failure message"
    );
    assert!(script.contains("deploy.sh v1.0"), "dispatch command");
    let gate_pos = script.find("DISPATCH BLOCKED").unwrap();
    let cmd_pos = script.find("deploy.sh v1.0").unwrap();
    assert!(gate_pos < cmd_pos, "gate must run before dispatch");
}

#[test]
fn test_batch_mode_explicit() {
    let mut r = make_task_resource("cargo build");
    r.task_mode = Some(TaskMode::Batch);
    let script = apply_script(&r);
    assert!(script.contains("set -euo pipefail"));
    assert!(script.contains("cargo build"));
    assert!(!script.contains("nohup"));
    assert!(!script.contains("DISPATCH BLOCKED"));
}

#[test]
fn test_service_mode_with_working_dir() {
    let mut r = make_task_resource("./start.sh");
    r.task_mode = Some(TaskMode::Service);
    r.name = Some("app".into());
    r.working_dir = Some("/opt/app".into());
    let script = apply_script(&r);
    assert!(script.contains("cd '/opt/app'"));
    assert!(script.contains("nohup"));
}

// ── FJ-3000: PID-aware health check ──

#[test]
fn test_fj3000_service_health_check_pid_liveness() {
    let mut r = make_task_resource("python server.py");
    r.task_mode = Some(TaskMode::Service);
    r.name = Some("api".into());
    r.health_check = Some(crate::core::types::HealthCheck {
        command: "curl -sf http://localhost:8080/health".into(),
        timeout: Some("10s".into()),
        retries: Some(5),
        ..Default::default()
    });
    let script = apply_script(&r);
    // Must check PID liveness before each health probe
    assert!(
        script.contains("kill -0 \"$FORJAR_SVC_PID\""),
        "health loop must check PID liveness"
    );
    // Must tail log on PID death for debugging
    assert!(script.contains("tail -20"), "must tail log when PID dies");
    // Must clean up PID file on death
    assert!(script.contains("rm -f"), "must clean up PID file on death");
    // Must exit 1 on PID death
    assert!(
        script.contains("DIED during startup"),
        "must report PID death"
    );
}

#[test]
fn test_fj3000_service_no_health_check_no_pid_loop() {
    let mut r = make_task_resource("./daemon");
    r.task_mode = Some(TaskMode::Service);
    r.name = Some("daemon".into());
    // No health_check configured
    let script = apply_script(&r);
    // Should NOT have a health check loop at all
    assert!(
        !script.contains("seq 1"),
        "no retry loop without health check"
    );
    // But must still have PID capture
    assert!(script.contains("FORJAR_SVC_PID=$!"));
}

#[test]
fn test_fj3000_pid_check_before_health_probe() {
    let mut r = make_task_resource("./start");
    r.task_mode = Some(TaskMode::Service);
    r.name = Some("svc".into());
    r.health_check = Some(crate::core::types::HealthCheck {
        command: "test -f /ready".into(),
        timeout: Some("3s".into()),
        retries: Some(2),
        ..Default::default()
    });
    let script = apply_script(&r);
    // PID liveness check must come BEFORE health probe in the loop
    let pid_check_pos = script.find("kill -0 \"$FORJAR_SVC_PID\"").unwrap();
    let health_probe_pos = script.find("test -f /ready").unwrap();
    assert!(
        pid_check_pos < health_probe_pos,
        "PID check must precede health probe"
    );
}
