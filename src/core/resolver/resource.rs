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
        Some(val) => Ok(Some(resolve_template_with_secrets(
            val, params, machines, secrets,
        )?)),
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

    // github_release fields
    r.install_dir = resolve_opt(&r.install_dir, params, machines, secrets)?;
    r.repo = resolve_opt(&r.repo, params, machines, secrets)?;
    r.tag = resolve_opt(&r.tag, params, machines, secrets)?;
    r.asset_pattern = resolve_opt(&r.asset_pattern, params, machines, secrets)?;
    r.binary = resolve_opt(&r.binary, params, machines, secrets)?;
    r.build_machine = resolve_opt(&r.build_machine, params, machines, secrets)?;
    Ok(())
}

/// Resolve the build, isolation and overlay fields.
///
/// FJ-2721 (PMAT-199): these were all silently unresolved — a hand-maintained
/// assignment list has no way to notice a field it omits. The worst was
/// `task_inputs`, the field v1.11's whole incremental-build release is about:
/// its sibling `output_artifacts` was resolved, it was not, so a config that
/// templated its inputs got `Apply complete: 0 converged, 1 unchanged` over a
/// stale artifact. `resolver::tests_completeness` now discovers the field set
/// by reflection so the next omission fails a test instead of shipping.
fn resolve_build_and_overlay_fields(
    r: &mut Resource,
    params: &HashMap<String, serde_yaml_ng::Value>,
    machines: &indexmap::IndexMap<String, Machine>,
    secrets: &SecretsConfig,
) -> Result<(), String> {
    // Build I/O — consumed by the staleness probe and the executor.
    r.task_inputs = resolve_list(&r.task_inputs, params, machines, secrets)?;
    r.scatter = resolve_list(&r.scatter, params, machines, secrets)?;
    r.gather = resolve_list(&r.gather, params, machines, secrets)?;
    r.cache_dir = resolve_opt(&r.cache_dir, params, machines, secrets)?;
    r.when = resolve_opt(&r.when, params, machines, secrets)?;

    // Dispatch/selection strings consumed by the generators.
    r.state = resolve_opt(&r.state, params, machines, secrets)?;
    r.provider = resolve_opt(&r.provider, params, machines, secrets)?;
    r.fs_type = resolve_opt(&r.fs_type, params, machines, secrets)?;
    r.groups = resolve_list(&r.groups, params, machines, secrets)?;
    r.ssh_authorized_keys = resolve_list(&r.ssh_authorized_keys, params, machines, secrets)?;
    r.arch = resolve_list(&r.arch, params, machines, secrets)?;

    // Model fields.
    r.checksum = resolve_opt(&r.checksum, params, machines, secrets)?;
    r.format = resolve_opt(&r.format, params, machines, secrets)?;
    r.quantization = resolve_opt(&r.quantization, params, machines, secrets)?;

    // Isolation (pepita).
    r.chroot_dir = resolve_opt(&r.chroot_dir, params, machines, secrets)?;
    r.cpuset = resolve_opt(&r.cpuset, params, machines, secrets)?;

    // Fleet overlay — a templated overlay IP would otherwise be configured
    // literally on the interface.
    r.overlay_ip = resolve_opt(&r.overlay_ip, params, machines, secrets)?;
    r.overlay_iface = resolve_opt(&r.overlay_iface, params, machines, secrets)?;
    r.overlay_lower = resolve_opt(&r.overlay_lower, params, machines, secrets)?;
    r.overlay_upper = resolve_opt(&r.overlay_upper, params, machines, secrets)?;
    r.overlay_work = resolve_opt(&r.overlay_work, params, machines, secrets)?;
    r.overlay_merged = resolve_opt(&r.overlay_merged, params, machines, secrets)?;

    // Disk budget — the cadence, and every reclaim root. Roots are the paths
    // the reaper DELETES under, so an unexpanded `{{params.home}}` there is not
    // a cosmetic bug: the literal would simply never match and the rule would
    // silently reclaim nothing, which is the exact silent-inertness this
    // resource exists to prevent.
    r.budget_schedule = resolve_opt(&r.budget_schedule, params, machines, secrets)?;
    for rule in &mut r.budget_reclaim {
        rule.roots = resolve_list(&rule.roots, params, machines, secrets)?;
    }

    // Backup sync. `backup_token` is the important one: it arrives as
    // `{{secrets.NAME}}` and an unresolved value would be written into
    // rclone.conf as the literal credential. The sources matter for the same
    // reason budget roots do — an unexpanded path matches nothing, and a backup
    // that silently protects nothing is exactly what this resource replaced.
    r.backup.remote = resolve_opt(&r.backup.remote, params, machines, secrets)?;
    r.backup.remote_type = resolve_opt(&r.backup.remote_type, params, machines, secrets)?;
    r.backup.schedule = resolve_opt(&r.backup.schedule, params, machines, secrets)?;
    r.backup.bandwidth_limit = resolve_opt(&r.backup.bandwidth_limit, params, machines, secrets)?;
    r.backup.token = resolve_opt(&r.backup.token, params, machines, secrets)?;
    r.backup.source = resolve_list(&r.backup.source, params, machines, secrets)?;
    for v in r.backup.remote_config.values_mut() {
        *v = resolve_template_with_secrets(v, params, machines, secrets)?;
    }

    Ok(())
}

/// Resolve pipeline stages.
///
/// `stages` is a `Vec<PipelineStage>`, so a per-field assignment list never
/// reaches it, yet `pipeline_script` splices `command` straight into executed
/// shell and hashes `inputs`/`outputs` for stage caching.
fn resolve_stages(
    r: &mut Resource,
    params: &HashMap<String, serde_yaml_ng::Value>,
    machines: &indexmap::IndexMap<String, Machine>,
    secrets: &SecretsConfig,
) -> Result<(), String> {
    for stage in &mut r.stages {
        stage.command = resolve_opt(&stage.command, params, machines, secrets)?;
        stage.inputs = resolve_list(&stage.inputs, params, machines, secrets)?;
        stage.outputs = resolve_list(&stage.outputs, params, machines, secrets)?;
    }
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
    resolve_build_and_overlay_fields(&mut r, params, machines, secrets)?;
    resolve_stages(&mut r, params, machines, secrets)?;

    r.ports = resolve_list(&r.ports, params, machines, secrets)?;
    r.environment = resolve_list(&r.environment, params, machines, secrets)?;
    r.volumes = resolve_list(&r.volumes, params, machines, secrets)?;
    r.packages = resolve_list(&r.packages, params, machines, secrets)?;
    r.output_artifacts = resolve_list(&r.output_artifacts, params, machines, secrets)?;

    Ok(r)
}

/// FJ-1500 (PMAT-197): Resolve a resource's templates, falling back to the
/// unresolved resource on error.
///
/// This is the ONE place that decision is made. It previously lived privately
/// in the planner, and the same bug was then reintroduced twice by callers who
/// passed raw `config.resources` straight through:
///
/// * the planner itself (FJ-154 / #19) — secret-bearing resources replanned as
///   a spurious `Update` forever, violating `f(f(x)) = f(x)`;
/// * `forjar drift` — ANY templated resource reported permanent false drift,
///   and because the apply-time drift gate is global, that blocked every
///   targeted apply fleet-wide (paiml/infra PMAT-196 had to work around it
///   with `--no-tripwire`);
/// * `forjar destroy` — generated destroy scripts containing literal
///   `{{params.*}}` paths.
///
/// Resolving with the SAME `SecretsConfig` the executor uses is load-bearing:
/// resolving with the default (env) provider makes the planner-computed
/// desired hash disagree with the executor-stored hash.
pub fn resolve_or_fallback(
    resource_id: &str,
    resource: &Resource,
    params: &HashMap<String, serde_yaml_ng::Value>,
    machines: &indexmap::IndexMap<String, Machine>,
    secrets: &SecretsConfig,
) -> Resource {
    resolve_resource_templates_with_secrets(resource, params, machines, secrets).unwrap_or_else(
        |e| {
            eprintln!("warning: template resolution failed for {resource_id}: {e}");
            resource.clone()
        },
    )
}

/// Resolve every resource in a map. Use this anywhere a whole config is handed
/// to a consumer that compares against live machine state.
pub fn resolve_all(
    resources: &indexmap::IndexMap<String, Resource>,
    params: &HashMap<String, serde_yaml_ng::Value>,
    machines: &indexmap::IndexMap<String, Machine>,
    secrets: &SecretsConfig,
) -> indexmap::IndexMap<String, Resource> {
    resources
        .iter()
        .map(|(id, r)| {
            (
                id.clone(),
                resolve_or_fallback(id, r, params, machines, secrets),
            )
        })
        .collect()
}

/// Resource IDs whose RESOLVED form still carries a `{{secrets.*}}` placeholder.
///
/// `resolve_or_fallback` above deliberately returns the UNRESOLVED resource when
/// template resolution fails, so plan/drift/destroy all make the same decision.
/// The cost is that a secret which cannot be resolved survives as the literal
/// string `{{secrets.name}}` — and a credential-shaped placeholder shipped to a
/// machine is not a credential. `forjar apply` must refuse rather than write it.
///
/// Serialising the whole resource, rather than checking a hand-written list of
/// secret-bearing fields, means a newly added field is covered the day it lands.
pub fn unresolved_secret_resources(
    resources: &indexmap::IndexMap<String, Resource>,
) -> Vec<String> {
    resources
        .iter()
        .filter(|(_, r)| has_unresolved_secret(r))
        .map(|(id, _)| id.clone())
        .collect()
}

/// True when this single resource still carries a `{{secrets.*}}` placeholder.
///
/// Checked at the codegen chokepoint so ONE unresolvable secret fails ONE
/// resource, the way `backup_sync` has always behaved — rather than aborting an
/// otherwise-fine apply of the whole machine.
pub fn has_unresolved_secret(resource: &Resource) -> bool {
    serde_yaml_ng::to_string(resource).is_ok_and(|s| s.contains("{{secrets."))
}
