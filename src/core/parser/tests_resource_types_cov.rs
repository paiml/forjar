//! Coverage tests for resource_types.rs — kernel headers, cron schedule,
//! pepita cpuset, build fields, task pipeline, model/gpu validators.

use super::resource_types::validate_resource_type;
use crate::core::types::*;

fn make_resource(rtype: ResourceType) -> Resource {
    Resource {
        resource_type: rtype,
        machine: MachineTarget::Single("local".to_string()),
        ..Resource::default()
    }
}

// ── check_kernel_headers ────────────────────────────────────────

#[test]
fn kernel_image_without_headers_warns() {
    let mut r = make_resource(ResourceType::Package);
    r.packages = vec!["linux-image-6.1.0-generic".to_string()];
    r.provider = Some("apt".to_string());
    let mut errors = Vec::new();
    validate_resource_type("pkg", &r, &mut errors);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("linux-headers-6.1.0-generic")));
}

#[test]
fn kernel_image_with_headers_no_warning() {
    let mut r = make_resource(ResourceType::Package);
    r.packages = vec![
        "linux-image-6.1.0-generic".to_string(),
        "linux-headers-6.1.0-generic".to_string(),
    ];
    r.provider = Some("apt".to_string());
    let mut errors = Vec::new();
    validate_resource_type("pkg", &r, &mut errors);
    assert!(
        !errors.iter().any(|e| e.message.contains("linux-headers")),
        "no headers warning expected: {errors:?}"
    );
}

#[test]
fn non_kernel_packages_no_headers_check() {
    let mut r = make_resource(ResourceType::Package);
    r.packages = vec!["nginx".to_string(), "curl".to_string()];
    r.provider = Some("apt".to_string());
    let mut errors = Vec::new();
    validate_resource_type("pkg", &r, &mut errors);
    assert!(!errors.iter().any(|e| e.message.contains("linux-headers")));
}

// ── validate_cron: schedule ─────────────────────────────────────

#[test]
fn cron_valid_5_field_schedule() {
    let mut r = make_resource(ResourceType::Cron);
    r.name = Some("backup".to_string());
    r.schedule = Some("0 2 * * *".to_string());
    r.command = Some("/usr/bin/backup".to_string());
    let mut errors = Vec::new();
    validate_resource_type("cron", &r, &mut errors);
    assert!(
        !errors.iter().any(|e| e.message.contains("5 fields")),
        "valid schedule should not error: {errors:?}"
    );
}

#[test]
fn cron_invalid_field_count() {
    let mut r = make_resource(ResourceType::Cron);
    r.name = Some("bad".to_string());
    r.schedule = Some("0 2 *".to_string()); // Only 3 fields
    r.command = Some("/usr/bin/bad".to_string());
    let mut errors = Vec::new();
    validate_resource_type("cron", &r, &mut errors);
    assert!(errors.iter().any(|e| e.message.contains("5 fields")));
}

#[test]
fn cron_keyword_schedule_skips_validation() {
    let mut r = make_resource(ResourceType::Cron);
    r.name = Some("daily".to_string());
    r.schedule = Some("@daily".to_string());
    r.command = Some("/usr/bin/daily".to_string());
    let mut errors = Vec::new();
    validate_resource_type("cron", &r, &mut errors);
    assert!(!errors.iter().any(|e| e.message.contains("5 fields")));
}

#[test]
fn cron_template_schedule_skips_validation() {
    let mut r = make_resource(ResourceType::Cron);
    r.name = Some("tmpl".to_string());
    r.schedule = Some("{{inputs.schedule}}".to_string());
    r.command = Some("/usr/bin/tmpl".to_string());
    let mut errors = Vec::new();
    validate_resource_type("cron", &r, &mut errors);
    assert!(!errors.iter().any(|e| e.message.contains("5 fields")));
}

#[test]
fn cron_no_schedule_absent_ok() {
    let mut r = make_resource(ResourceType::Cron);
    r.name = Some("absent-cron".to_string());
    r.state = Some("absent".to_string());
    // No schedule, no command — absent state OK
    let mut errors = Vec::new();
    validate_resource_type("cron", &r, &mut errors);
    assert!(!errors.iter().any(|e| e.message.contains("no schedule")));
    assert!(!errors.iter().any(|e| e.message.contains("no command")));
}

#[test]
fn cron_invalid_state() {
    let mut r = make_resource(ResourceType::Cron);
    r.name = Some("bad-state".to_string());
    r.schedule = Some("0 * * * *".to_string());
    r.command = Some("/bin/true".to_string());
    r.state = Some("running".to_string());
    let mut errors = Vec::new();
    validate_resource_type("cron", &r, &mut errors);
    assert!(errors.iter().any(|e| e.message.contains("invalid state")));
}

#[test]
fn cron_no_name() {
    let mut r = make_resource(ResourceType::Cron);
    r.schedule = Some("0 * * * *".to_string());
    r.command = Some("/bin/true".to_string());
    let mut errors = Vec::new();
    validate_resource_type("cron", &r, &mut errors);
    assert!(errors.iter().any(|e| e.message.contains("no name")));
}

#[test]
fn cron_no_command() {
    let mut r = make_resource(ResourceType::Cron);
    r.name = Some("nocmd".to_string());
    r.schedule = Some("0 * * * *".to_string());
    let mut errors = Vec::new();
    validate_resource_type("cron", &r, &mut errors);
    assert!(errors.iter().any(|e| e.message.contains("no command")));
}

// ── validate_pepita ─────────────────────────────────────────────

#[test]
fn pepita_empty_cpuset_error() {
    let mut r = make_resource(ResourceType::Pepita);
    r.name = Some("p".to_string());
    r.cpuset = Some(String::new());
    let mut errors = Vec::new();
    validate_resource_type("p", &r, &mut errors);
    assert!(errors.iter().any(|e| e.message.contains("empty cpuset")));
}

#[test]
fn pepita_invalid_state() {
    let mut r = make_resource(ResourceType::Pepita);
    r.name = Some("p".to_string());
    r.state = Some("running".to_string());
    let mut errors = Vec::new();
    validate_resource_type("p", &r, &mut errors);
    assert!(errors.iter().any(|e| e.message.contains("invalid state")));
}

#[test]
fn pepita_valid_state_present() {
    let mut r = make_resource(ResourceType::Pepita);
    r.name = Some("p".to_string());
    r.state = Some("present".to_string());
    let mut errors = Vec::new();
    validate_resource_type("p", &r, &mut errors);
    assert!(!errors.iter().any(|e| e.message.contains("invalid state")));
}

// ── validate_model ──────────────────────────────────────────────

#[test]
fn model_no_name_error() {
    let r = make_resource(ResourceType::Model);
    let mut errors = Vec::new();
    validate_resource_type("mdl", &r, &mut errors);
    assert!(errors.iter().any(|e| e.message.contains("no name")));
}

#[test]
fn model_invalid_state() {
    let mut r = make_resource(ResourceType::Model);
    r.name = Some("llama".to_string());
    r.state = Some("running".to_string());
    let mut errors = Vec::new();
    validate_resource_type("mdl", &r, &mut errors);
    assert!(errors.iter().any(|e| e.message.contains("invalid state")));
}

// ── validate_gpu ────────────────────────────────────────────────

#[test]
fn gpu_no_driver_version() {
    let r = make_resource(ResourceType::Gpu);
    let mut errors = Vec::new();
    validate_resource_type("gpu", &r, &mut errors);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("no driver_version")));
}

#[test]
fn gpu_invalid_state() {
    let mut r = make_resource(ResourceType::Gpu);
    r.driver_version = Some("535".to_string());
    r.state = Some("running".to_string());
    let mut errors = Vec::new();
    validate_resource_type("gpu", &r, &mut errors);
    assert!(errors.iter().any(|e| e.message.contains("invalid state")));
}

// ── validate_recipe ─────────────────────────────────────────────

#[test]
fn recipe_no_recipe_name() {
    let r = make_resource(ResourceType::Recipe);
    let mut errors = Vec::new();
    validate_resource_type("rcp", &r, &mut errors);
    assert!(errors.iter().any(|e| e.message.contains("no recipe name")));
}

// ── validate_task: pipeline mode ────────────────────────────────

#[test]
fn task_pipeline_no_stages() {
    let mut r = make_resource(ResourceType::Task);
    r.task_mode = Some(TaskMode::Pipeline);
    let mut errors = Vec::new();
    validate_resource_type("t", &r, &mut errors);
    assert!(errors.iter().any(|e| e.message.contains("no stages")));
    // Pipeline mode should NOT require command
    assert!(!errors.iter().any(|e| e.message.contains("no command")));
}

#[test]
fn task_zero_timeout() {
    let mut r = make_resource(ResourceType::Task);
    r.command = Some("/bin/true".to_string());
    r.timeout = Some(0);
    let mut errors = Vec::new();
    validate_resource_type("t", &r, &mut errors);
    assert!(errors.iter().any(|e| e.message.contains("timeout of 0")));
}

#[test]
fn task_no_command_no_pipeline() {
    let r = make_resource(ResourceType::Task);
    let mut errors = Vec::new();
    validate_resource_type("t", &r, &mut errors);
    assert!(errors.iter().any(|e| e.message.contains("no command")));
}

// ── validate_build ──────────────────────────────────────────────

#[test]
fn build_no_build_machine() {
    let r = make_resource(ResourceType::Build);
    let mut errors = Vec::new();
    validate_resource_type("b", &r, &mut errors);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("no build_machine")));
}

#[test]
fn build_no_command() {
    let mut r = make_resource(ResourceType::Build);
    r.build_machine = Some("ci".to_string());
    let mut errors = Vec::new();
    validate_resource_type("b", &r, &mut errors);
    assert!(errors.iter().any(|e| e.message.contains("no command")));
}

#[test]
fn build_no_source() {
    let mut r = make_resource(ResourceType::Build);
    r.build_machine = Some("ci".to_string());
    r.command = Some("make".to_string());
    let mut errors = Vec::new();
    validate_resource_type("b", &r, &mut errors);
    assert!(errors.iter().any(|e| e.message.contains("no source")));
}

#[test]
fn build_no_target() {
    let mut r = make_resource(ResourceType::Build);
    r.build_machine = Some("ci".to_string());
    r.command = Some("make".to_string());
    r.source = Some("/opt/build/out".to_string());
    let mut errors = Vec::new();
    validate_resource_type("b", &r, &mut errors);
    assert!(errors.iter().any(|e| e.message.contains("no target")));
}

#[test]
fn build_all_fields_ok() {
    let mut r = make_resource(ResourceType::Build);
    r.build_machine = Some("ci".to_string());
    r.command = Some("make".to_string());
    r.source = Some("/opt/build/out".to_string());
    r.target = Some("/srv/app/bin".to_string());
    let mut errors = Vec::new();
    validate_resource_type("b", &r, &mut errors);
    // File validator fires (WasmBundle/Image fall through to validate_file)
    // but Build has its own validator — no build-specific errors
    assert!(!errors.iter().any(|e| e.message.contains("build")));
}

// ── validate_file: state variants ───────────────────────────────

#[test]
fn file_invalid_state() {
    let mut r = make_resource(ResourceType::File);
    r.path = Some("/tmp/x".to_string());
    r.state = Some("running".to_string());
    let mut errors = Vec::new();
    validate_resource_type("f", &r, &mut errors);
    assert!(errors.iter().any(|e| e.message.contains("invalid state")));
}

#[test]
fn file_symlink_without_target() {
    let mut r = make_resource(ResourceType::File);
    r.path = Some("/tmp/link".to_string());
    r.state = Some("symlink".to_string());
    let mut errors = Vec::new();
    validate_resource_type("f", &r, &mut errors);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("requires a target")));
}

#[test]
fn file_content_and_source_conflict() {
    let mut r = make_resource(ResourceType::File);
    r.path = Some("/tmp/x".to_string());
    r.content = Some("hello".to_string());
    r.source = Some("/src/x".to_string());
    let mut errors = Vec::new();
    validate_resource_type("f", &r, &mut errors);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("both content and source")));
}

// ── validate_docker: state variants ─────────────────────────────

#[test]
fn docker_no_image_absent_ok() {
    let mut r = make_resource(ResourceType::Docker);
    r.name = Some("d".to_string());
    r.state = Some("absent".to_string());
    let mut errors = Vec::new();
    validate_resource_type("d", &r, &mut errors);
    assert!(!errors.iter().any(|e| e.message.contains("no image")));
}

#[test]
fn docker_invalid_state() {
    let mut r = make_resource(ResourceType::Docker);
    r.name = Some("d".to_string());
    r.image = Some("nginx:latest".to_string());
    r.state = Some("enabled".to_string());
    let mut errors = Vec::new();
    validate_resource_type("d", &r, &mut errors);
    assert!(errors.iter().any(|e| e.message.contains("invalid state")));
}

// ── validate_network: action variants ───────────────────────────

#[test]
fn network_invalid_action() {
    let mut r = make_resource(ResourceType::Network);
    r.port = Some("80".to_string());
    r.action = Some("block".to_string());
    let mut errors = Vec::new();
    validate_resource_type("net", &r, &mut errors);
    assert!(errors.iter().any(|e| e.message.contains("invalid action")));
}

#[test]
fn network_invalid_protocol() {
    let mut r = make_resource(ResourceType::Network);
    r.port = Some("443".to_string());
    r.protocol = Some("icmp".to_string());
    let mut errors = Vec::new();
    validate_resource_type("net", &r, &mut errors);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("invalid protocol")));
}

// ── validate_mount: state variants ──────────────────────────────

#[test]
fn mount_invalid_state() {
    let mut r = make_resource(ResourceType::Mount);
    r.source = Some("/dev/sda1".to_string());
    r.path = Some("/mnt/data".to_string());
    r.state = Some("enabled".to_string());
    let mut errors = Vec::new();
    validate_resource_type("mnt", &r, &mut errors);
    assert!(errors.iter().any(|e| e.message.contains("invalid state")));
}

// ── validate_service: state variants ────────────────────────────

#[test]
fn service_invalid_state() {
    let mut r = make_resource(ResourceType::Service);
    r.name = Some("nginx".to_string());
    r.state = Some("absent".to_string());
    let mut errors = Vec::new();
    validate_resource_type("svc", &r, &mut errors);
    assert!(errors.iter().any(|e| e.message.contains("invalid state")));
}

// ── validate_user ───────────────────────────────────────────────

#[test]
fn user_no_name_error() {
    let r = make_resource(ResourceType::User);
    let mut errors = Vec::new();
    validate_resource_type("u", &r, &mut errors);
    assert!(errors.iter().any(|e| e.message.contains("no name")));
}
