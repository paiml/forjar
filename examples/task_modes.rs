//! FJ-2700/E21: Task mode script differentiation.
//!
//! Demonstrates how `task_mode` (batch, pipeline, service, dispatch) produces
//! different shell scripts for each mode.

use forjar::core::types::{
    HealthCheck, MachineTarget, PipelineStage, QualityGate, Resource, ResourceType, TaskMode,
};
use forjar::resources::task::{apply_script, check_script};

fn make_resource(cmd: &str) -> Resource {
    Resource {
        resource_type: ResourceType::Task,
        machine: MachineTarget::Single("worker".into()),
        command: Some(cmd.into()),
        ..Default::default()
    }
}

fn main() {
    println!("=== Batch Mode (default) ===");
    let batch = make_resource("cargo build --release");
    println!("{}", apply_script(&batch));

    println!("=== Pipeline Mode ===");
    let mut pipeline = make_resource("ignored");
    pipeline.task_mode = Some(TaskMode::Pipeline);
    pipeline.stages = vec![
        PipelineStage {
            name: "lint".into(),
            command: Some("cargo clippy -- -D warnings".into()),
            gate: true,
            ..Default::default()
        },
        PipelineStage {
            name: "test".into(),
            command: Some("cargo test".into()),
            gate: true,
            ..Default::default()
        },
        PipelineStage {
            name: "bench".into(),
            command: Some("cargo bench".into()),
            gate: false,
            ..Default::default()
        },
    ];
    println!("{}", apply_script(&pipeline));

    println!("=== Service Mode ===");
    let mut service = make_resource("python serve.py --port 8080");
    service.task_mode = Some(TaskMode::Service);
    service.name = Some("api-server".into());
    service.health_check = Some(HealthCheck {
        command: "curl -sf http://localhost:8080/health".into(),
        timeout: Some("5s".into()),
        retries: Some(3),
        ..Default::default()
    });
    println!("{}", apply_script(&service));
    println!("── check_script (service) ──");
    println!("{}", check_script(&service));

    println!("=== Dispatch Mode ===");
    let mut dispatch = make_resource("deploy.sh {{dispatch.version}}");
    dispatch.task_mode = Some(TaskMode::Dispatch);
    dispatch.quality_gate = Some(QualityGate {
        command: Some("test -f /opt/app/deploy-ready.flag".into()),
        message: Some("Deployment not ready — run pre-flight first".into()),
        ..Default::default()
    });
    println!("{}", apply_script(&dispatch));

    println!("All 4 task modes demonstrated.");
}
