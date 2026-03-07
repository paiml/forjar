use super::template::resolve_template_with_secrets;
use crate::core::types::*;
use std::collections::HashMap;

/// Resolve a single optional string field through the template engine.
fn resolve_opt(
    field: &Option<String>,
    params: &HashMap<String, serde_yaml_ng::Value>,
    machines: &indexmap::IndexMap<String, Machine>,
    secrets: &SecretsConfig,
) -> Result<Option<String>, String> {
    match field {
        Some(val) => Ok(Some(resolve_template_with_secrets(val, params, machines, secrets)?)),
        None => Ok(None),
    }
}

/// Resolve a list of strings through the template engine.
fn resolve_list(
    items: &[String],
    params: &HashMap<String, serde_yaml_ng::Value>,
    machines: &indexmap::IndexMap<String, Machine>,
    secrets: &SecretsConfig,
) -> Result<Vec<String>, String> {
    items
        .iter()
        .map(|s| resolve_template_with_secrets(s, params, machines, secrets))
        .collect()
}

/// Resolve core string fields (path, content, ownership, etc.).
fn resolve_core_fields(
    r: &mut Resource,
    params: &HashMap<String, serde_yaml_ng::Value>,
    machines: &indexmap::IndexMap<String, Machine>,
    secrets: &SecretsConfig,
) -> Result<(), String> {
    r.content = resolve_opt(&r.content, params, machines, secrets)?;
    r.source = resolve_opt(&r.source, params, machines, secrets)?;
    r.path = resolve_opt(&r.path, params, machines, secrets)?;
    r.target = resolve_opt(&r.target, params, machines, secrets)?;
    r.owner = resolve_opt(&r.owner, params, machines, secrets)?;
    r.group = resolve_opt(&r.group, params, machines, secrets)?;
    r.mode = resolve_opt(&r.mode, params, machines, secrets)?;
    r.name = resolve_opt(&r.name, params, machines, secrets)?;
    r.options = resolve_opt(&r.options, params, machines, secrets)?;
    r.command = resolve_opt(&r.command, params, machines, secrets)?;
    r.schedule = resolve_opt(&r.schedule, params, machines, secrets)?;
    r.port = resolve_opt(&r.port, params, machines, secrets)?;
    r.protocol = resolve_opt(&r.protocol, params, machines, secrets)?;
    r.action = resolve_opt(&r.action, params, machines, secrets)?;
    r.from_addr = resolve_opt(&r.from_addr, params, machines, secrets)?;
    r.image = resolve_opt(&r.image, params, machines, secrets)?;
    r.shell = resolve_opt(&r.shell, params, machines, secrets)?;
    r.home = resolve_opt(&r.home, params, machines, secrets)?;
    r.restart = resolve_opt(&r.restart, params, machines, secrets)?;
    r.version = resolve_opt(&r.version, params, machines, secrets)?;
    Ok(())
}

/// Resolve GPU, task, and extended string fields.
fn resolve_extended_fields(
    r: &mut Resource,
    params: &HashMap<String, serde_yaml_ng::Value>,
    machines: &indexmap::IndexMap<String, Machine>,
    secrets: &SecretsConfig,
) -> Result<(), String> {
    // PMAT-039: GPU / ML model fields
    r.driver_version = resolve_opt(&r.driver_version, params, machines, secrets)?;
    r.cuda_version = resolve_opt(&r.cuda_version, params, machines, secrets)?;
    r.rocm_version = resolve_opt(&r.rocm_version, params, machines, secrets)?;
    r.gpu_backend = resolve_opt(&r.gpu_backend, params, machines, secrets)?;
    r.compute_mode = resolve_opt(&r.compute_mode, params, machines, secrets)?;

    // Task fields (ALB-027)
    r.working_dir = resolve_opt(&r.working_dir, params, machines, secrets)?;
    r.completion_check = resolve_opt(&r.completion_check, params, machines, secrets)?;
    r.pre_apply = resolve_opt(&r.pre_apply, params, machines, secrets)?;
    r.post_apply = resolve_opt(&r.post_apply, params, machines, secrets)?;
    r.script = resolve_opt(&r.script, params, machines, secrets)?;
    Ok(())
}

/// Resolve all templates in a resource's string fields.
pub fn resolve_resource_templates(
    resource: &Resource,
    params: &HashMap<String, serde_yaml_ng::Value>,
    machines: &indexmap::IndexMap<String, Machine>,
) -> Result<Resource, String> {
    resolve_resource_templates_with_secrets(resource, params, machines, &SecretsConfig::default())
}

/// Resolve all templates with explicit secrets configuration.
pub fn resolve_resource_templates_with_secrets(
    resource: &Resource,
    params: &HashMap<String, serde_yaml_ng::Value>,
    machines: &indexmap::IndexMap<String, Machine>,
    secrets: &SecretsConfig,
) -> Result<Resource, String> {
    let mut r = resource.clone();

    resolve_core_fields(&mut r, params, machines, secrets)?;
    resolve_extended_fields(&mut r, params, machines, secrets)?;

    r.ports = resolve_list(&r.ports, params, machines, secrets)?;
    r.environment = resolve_list(&r.environment, params, machines, secrets)?;
    r.volumes = resolve_list(&r.volumes, params, machines, secrets)?;
    r.packages = resolve_list(&r.packages, params, machines, secrets)?;
    r.output_artifacts = resolve_list(&r.output_artifacts, params, machines, secrets)?;

    Ok(r)
}
