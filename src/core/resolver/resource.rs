use super::template::resolve_template;
use crate::core::types::*;
use std::collections::HashMap;

/// Resolve a single optional string field through the template engine.
fn resolve_opt(
    field: &Option<String>,
    params: &HashMap<String, serde_yaml_ng::Value>,
    machines: &indexmap::IndexMap<String, Machine>,
) -> Result<Option<String>, String> {
    match field {
        Some(val) => Ok(Some(resolve_template(val, params, machines)?)),
        None => Ok(None),
    }
}

/// Resolve a list of strings through the template engine.
fn resolve_list(
    items: &[String],
    params: &HashMap<String, serde_yaml_ng::Value>,
    machines: &indexmap::IndexMap<String, Machine>,
) -> Result<Vec<String>, String> {
    items
        .iter()
        .map(|s| resolve_template(s, params, machines))
        .collect()
}

/// Resolve core string fields (path, content, ownership, etc.).
fn resolve_core_fields(
    r: &mut Resource,
    params: &HashMap<String, serde_yaml_ng::Value>,
    machines: &indexmap::IndexMap<String, Machine>,
) -> Result<(), String> {
    r.content = resolve_opt(&r.content, params, machines)?;
    r.source = resolve_opt(&r.source, params, machines)?;
    r.path = resolve_opt(&r.path, params, machines)?;
    r.target = resolve_opt(&r.target, params, machines)?;
    r.owner = resolve_opt(&r.owner, params, machines)?;
    r.group = resolve_opt(&r.group, params, machines)?;
    r.mode = resolve_opt(&r.mode, params, machines)?;
    r.name = resolve_opt(&r.name, params, machines)?;
    r.options = resolve_opt(&r.options, params, machines)?;
    r.command = resolve_opt(&r.command, params, machines)?;
    r.schedule = resolve_opt(&r.schedule, params, machines)?;
    r.port = resolve_opt(&r.port, params, machines)?;
    r.protocol = resolve_opt(&r.protocol, params, machines)?;
    r.action = resolve_opt(&r.action, params, machines)?;
    r.from_addr = resolve_opt(&r.from_addr, params, machines)?;
    r.image = resolve_opt(&r.image, params, machines)?;
    r.shell = resolve_opt(&r.shell, params, machines)?;
    r.home = resolve_opt(&r.home, params, machines)?;
    r.restart = resolve_opt(&r.restart, params, machines)?;
    r.version = resolve_opt(&r.version, params, machines)?;
    Ok(())
}

/// Resolve GPU, task, and extended string fields.
fn resolve_extended_fields(
    r: &mut Resource,
    params: &HashMap<String, serde_yaml_ng::Value>,
    machines: &indexmap::IndexMap<String, Machine>,
) -> Result<(), String> {
    // PMAT-039: GPU / ML model fields
    r.driver_version = resolve_opt(&r.driver_version, params, machines)?;
    r.cuda_version = resolve_opt(&r.cuda_version, params, machines)?;
    r.rocm_version = resolve_opt(&r.rocm_version, params, machines)?;
    r.gpu_backend = resolve_opt(&r.gpu_backend, params, machines)?;
    r.compute_mode = resolve_opt(&r.compute_mode, params, machines)?;

    // Task fields (ALB-027)
    r.working_dir = resolve_opt(&r.working_dir, params, machines)?;
    r.completion_check = resolve_opt(&r.completion_check, params, machines)?;
    r.pre_apply = resolve_opt(&r.pre_apply, params, machines)?;
    r.post_apply = resolve_opt(&r.post_apply, params, machines)?;
    r.script = resolve_opt(&r.script, params, machines)?;
    Ok(())
}

/// Resolve all templates in a resource's string fields.
pub fn resolve_resource_templates(
    resource: &Resource,
    params: &HashMap<String, serde_yaml_ng::Value>,
    machines: &indexmap::IndexMap<String, Machine>,
) -> Result<Resource, String> {
    let mut r = resource.clone();

    resolve_core_fields(&mut r, params, machines)?;
    resolve_extended_fields(&mut r, params, machines)?;

    r.ports = resolve_list(&r.ports, params, machines)?;
    r.environment = resolve_list(&r.environment, params, machines)?;
    r.volumes = resolve_list(&r.volumes, params, machines)?;
    r.packages = resolve_list(&r.packages, params, machines)?;
    r.output_artifacts = resolve_list(&r.output_artifacts, params, machines)?;

    Ok(r)
}
