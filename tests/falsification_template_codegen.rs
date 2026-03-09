//! FJ-2300/005/036: Template resolution, secret redaction, codegen dispatch.
//!
//! Popperian rejection criteria for:
//! - FJ-2300: Template resolution (params, machine refs, redact_secrets)
//! - FJ-005: Codegen dispatch (check/apply/state_query scripts for all types)
//! - FJ-036: sudo wrapping and recipe rejection
//!
//! Usage: cargo test --test falsification_template_codegen

use forjar::core::codegen::{apply_script, check_script, state_query_script};
use forjar::core::resolver::{redact_secrets, resolve_template};
use forjar::core::types::*;
use std::collections::HashMap;

fn resource(rtype: ResourceType) -> Resource {
    Resource {
        resource_type: rtype,
        ..Default::default()
    }
}

fn machines() -> indexmap::IndexMap<String, Machine> {
    let mut m = indexmap::IndexMap::new();
    m.insert(
        "web".into(),
        Machine {
            hostname: "web-01".into(),
            addr: "10.0.0.1".into(),
            user: "deploy".into(),
            arch: "x86_64".into(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
            allowed_operators: vec![],
        },
    );
    m
}

// ============================================================================
// FJ-2300: resolve_template — params
// ============================================================================

#[test]
fn template_params_resolved() {
    let mut params = HashMap::new();
    params.insert("name".into(), serde_yaml_ng::Value::String("nginx".into()));
    let result =
        resolve_template("pkg: {{params.name}}", &params, &indexmap::IndexMap::new()).unwrap();
    assert_eq!(result, "pkg: nginx");
}

#[test]
fn template_multiple_params() {
    let mut params = HashMap::new();
    params.insert("pkg".into(), serde_yaml_ng::Value::String("curl".into()));
    params.insert("ver".into(), serde_yaml_ng::Value::String("7.88".into()));
    let result = resolve_template(
        "{{params.pkg}}=={{params.ver}}",
        &params,
        &indexmap::IndexMap::new(),
    )
    .unwrap();
    assert_eq!(result, "curl==7.88");
}

#[test]
fn template_no_templates_passthrough() {
    let result =
        resolve_template("plain text", &HashMap::new(), &indexmap::IndexMap::new()).unwrap();
    assert_eq!(result, "plain text");
}

#[test]
fn template_unknown_param_errors() {
    let result = resolve_template(
        "{{params.missing}}",
        &HashMap::new(),
        &indexmap::IndexMap::new(),
    );
    assert!(result.is_err());
}

#[test]
fn template_unclosed_brace_errors() {
    let result = resolve_template("{{params.x", &HashMap::new(), &indexmap::IndexMap::new());
    assert!(result.is_err());
}

// ============================================================================
// FJ-2300: resolve_template — machine refs
// ============================================================================

#[test]
fn template_machine_addr() {
    let result =
        resolve_template("host: {{machine.web.addr}}", &HashMap::new(), &machines()).unwrap();
    assert_eq!(result, "host: 10.0.0.1");
}

#[test]
fn template_machine_hostname() {
    let result =
        resolve_template("{{machine.web.hostname}}", &HashMap::new(), &machines()).unwrap();
    assert_eq!(result, "web-01");
}

#[test]
fn template_machine_user() {
    let result = resolve_template("{{machine.web.user}}", &HashMap::new(), &machines()).unwrap();
    assert_eq!(result, "deploy");
}

#[test]
fn template_machine_unknown_field_errors() {
    let result = resolve_template("{{machine.web.bogus}}", &HashMap::new(), &machines());
    assert!(result.is_err());
}

#[test]
fn template_machine_unknown_name_errors() {
    let result = resolve_template("{{machine.missing.addr}}", &HashMap::new(), &machines());
    assert!(result.is_err());
}

// ============================================================================
// FJ-2300: redact_secrets
// ============================================================================

#[test]
fn redact_replaces_values() {
    let text = "password is s3cret and key is t0ken";
    let result = redact_secrets(text, &["s3cret".into(), "t0ken".into()]);
    assert!(!result.contains("s3cret") && !result.contains("t0ken"));
    assert!(result.contains("***"));
}

#[test]
fn redact_empty_secret_skipped() {
    let text = "no change";
    let result = redact_secrets(text, &["".into()]);
    assert_eq!(result, "no change");
}

#[test]
fn redact_no_match_unchanged() {
    let text = "clean output";
    let result = redact_secrets(text, &["missing".into()]);
    assert_eq!(result, "clean output");
}

// ============================================================================
// FJ-005: codegen dispatch — check_script for all types
// ============================================================================

#[test]
fn codegen_check_all_types() {
    let types = [
        ResourceType::Package,
        ResourceType::File,
        ResourceType::Service,
        ResourceType::Mount,
        ResourceType::User,
        ResourceType::Docker,
        ResourceType::Cron,
        ResourceType::Network,
        ResourceType::Pepita,
        ResourceType::Model,
        ResourceType::Gpu,
        ResourceType::Task,
        ResourceType::WasmBundle,
        ResourceType::Image,
        ResourceType::Build,
    ];
    for rt in &types {
        let result = check_script(&resource(rt.clone()));
        assert!(
            result.is_ok(),
            "check_script failed for {rt:?}: {:?}",
            result.err()
        );
    }
}

#[test]
fn codegen_check_recipe_errors() {
    assert!(check_script(&resource(ResourceType::Recipe)).is_err());
}

// ============================================================================
// FJ-005: codegen dispatch — apply_script for all types
// ============================================================================

#[test]
fn codegen_apply_all_types() {
    let types = [
        ResourceType::Package,
        ResourceType::File,
        ResourceType::Service,
        ResourceType::Mount,
        ResourceType::User,
        ResourceType::Docker,
        ResourceType::Cron,
        ResourceType::Network,
        ResourceType::Pepita,
        ResourceType::Model,
        ResourceType::Gpu,
        ResourceType::Task,
        ResourceType::WasmBundle,
        ResourceType::Image,
        ResourceType::Build,
    ];
    for rt in &types {
        let result = apply_script(&resource(rt.clone()));
        assert!(
            result.is_ok(),
            "apply_script failed for {rt:?}: {:?}",
            result.err()
        );
    }
}

#[test]
fn codegen_apply_recipe_errors() {
    assert!(apply_script(&resource(ResourceType::Recipe)).is_err());
}

// ============================================================================
// FJ-005: codegen dispatch — state_query_script for all types
// ============================================================================

#[test]
fn codegen_state_query_all_types() {
    let types = [
        ResourceType::Package,
        ResourceType::File,
        ResourceType::Service,
        ResourceType::Mount,
        ResourceType::User,
        ResourceType::Docker,
        ResourceType::Cron,
        ResourceType::Network,
        ResourceType::Pepita,
        ResourceType::Model,
        ResourceType::Gpu,
        ResourceType::Task,
        ResourceType::WasmBundle,
        ResourceType::Image,
        ResourceType::Build,
    ];
    for rt in &types {
        let result = state_query_script(&resource(rt.clone()));
        assert!(
            result.is_ok(),
            "state_query_script failed for {rt:?}: {:?}",
            result.err()
        );
    }
}

#[test]
fn codegen_state_query_recipe_errors() {
    assert!(state_query_script(&resource(ResourceType::Recipe)).is_err());
}

// ============================================================================
// FJ-036: apply_script with sudo wrapping
// ============================================================================

#[test]
fn codegen_apply_sudo_wraps() {
    let mut r = resource(ResourceType::File);
    r.sudo = true;
    let script = apply_script(&r).unwrap();
    assert!(script.contains("sudo bash") || script.contains("id -u"));
}

#[test]
fn codegen_apply_no_sudo_plain() {
    let r = resource(ResourceType::File);
    let script = apply_script(&r).unwrap();
    assert!(!script.contains("FORJAR_SUDO"));
}

// ============================================================================
// FJ-005: check_script content correctness for specific types
// ============================================================================

#[test]
fn codegen_check_file_content() {
    let mut r = resource(ResourceType::File);
    r.path = Some("/etc/test.conf".into());
    let script = check_script(&r).unwrap();
    assert!(script.contains("/etc/test.conf"));
}

#[test]
fn codegen_check_package_content() {
    let mut r = resource(ResourceType::Package);
    r.packages = vec!["nginx".into()];
    let script = check_script(&r).unwrap();
    assert!(script.contains("nginx"));
}

#[test]
fn codegen_check_service_content() {
    let mut r = resource(ResourceType::Service);
    r.name = Some("nginx".into());
    let script = check_script(&r).unwrap();
    assert!(script.contains("nginx"));
}
