//! FJ-2700: `forjar run` — dispatch-mode task invocation.
//!
//! Finds a task resource with `mode: dispatch`, prepares the command
//! with param overrides, and executes it.
//!
//! # Two defects this module used to have
//!
//! 1. It shelled out the RAW `command:` straight from the parsed config, so a
//!    task carrying `{{params.who}}` executed the literal seven characters
//!    `{{param` … and printed `status: pass`. `apply` resolves the same
//!    template on the same resource; `run` did not, and `--param` was parsed
//!    into a map nothing read.
//! 2. `--json` returned after serialising the task descriptor. No execution at
//!    all — and a task that fails with exit 3 in the plain form exited 0 under
//!    `--json`. `--help` calls `--json` an output format, not a mode.
//!
//! Both are the same shape: the command reported a result it never measured.
//! The tests for them assert the file the task writes, never the banner.

use super::helpers::*;
use crate::core::task::dispatch;
use crate::core::types::{self, DispatchConfig};
use std::path::Path;

/// Flatten `config.params` into the `(key, value)` pairs `prepare_dispatch`
/// substitutes for the bare `{{ key }}` dispatch shorthand.
///
/// `--param` overrides are already merged into `config.params` by
/// [`super::apply_helpers::apply_param_overrides`] before this is called, so
/// an override genuinely wins here — it is the same map `{{params.key}}`
/// resolves against, which is the point: one value, two spellings.
fn dispatch_params(
    params: &std::collections::HashMap<String, serde_yaml_ng::Value>,
) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = params
        .iter()
        .map(|(k, v)| (k.clone(), yaml_scalar_to_string(v)))
        .collect();
    // Deterministic order: `prepare_dispatch` applies substitutions in
    // sequence, and a HashMap iteration order would make the result depend on
    // the hash seed.
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    pairs
}

/// Render a YAML scalar the way the template resolver does.
fn yaml_scalar_to_string(value: &serde_yaml_ng::Value) -> String {
    match value {
        serde_yaml_ng::Value::String(s) => s.clone(),
        serde_yaml_ng::Value::Number(n) => n.to_string(),
        serde_yaml_ng::Value::Bool(b) => b.to_string(),
        other => serde_yaml_ng::to_string(other)
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

/// Build a DispatchConfig from a task resource.
fn build_dispatch_config(
    task_id: &str,
    resource: &types::Resource,
    params: &[(String, String)],
) -> DispatchConfig {
    let command = resource.command.clone().unwrap_or_default();
    DispatchConfig {
        name: task_id.to_string(),
        command,
        params: params.to_vec(),
        timeout_secs: None,
    }
}

/// Look the task up and refuse anything that is not a task.
fn select_task<'a>(
    config: &'a types::ForjarConfig,
    task_id: &str,
) -> Result<&'a types::Resource, String> {
    let resource = config
        .resources
        .get(task_id)
        .ok_or_else(|| format!("resource '{task_id}' not found in config"))?;

    if resource.resource_type != types::ResourceType::Task {
        return Err(format!(
            "resource '{task_id}' is not a task (type: {:?})",
            resource.resource_type
        ));
    }
    Ok(resource)
}

/// Resolve the task's templates exactly the way `apply` resolves them.
///
/// The bare `{{ key }}` dispatch shorthand is substituted first (it is not a
/// config-level template and the resolver would reject it), then the whole
/// resource goes through `resolve_resource_templates_with_secrets`, which is
/// the same call `apply`, `plan` and `destroy` make. An unresolved template is
/// an ERROR here, never something to hand to a shell: executing the literal
/// text of a template is the defect this module is named for.
fn resolve_task(
    config: &types::ForjarConfig,
    resource: &types::Resource,
    task_id: &str,
) -> Result<types::Resource, String> {
    let pairs = dispatch_params(&config.params);
    let shorthand = build_dispatch_config(task_id, resource, &pairs);
    let mut staged = resource.clone();
    staged.command = Some(dispatch::prepare_dispatch(&shorthand, &[]).command);

    crate::core::resolver::resolve_resource_templates_with_secrets(
        &staged,
        &config.params,
        &config.machines,
        &config.secrets,
    )
}

/// Pick the machine the task runs on, defaulting to localhost.
fn task_machine(config: &types::ForjarConfig, resource: &types::Resource) -> types::Machine {
    let machine_name = resource
        .machine
        .iter()
        .next()
        .map(|s| s.to_owned())
        .unwrap_or_default();
    config
        .machines
        .get(&machine_name)
        .cloned()
        .unwrap_or_else(super::check::localhost_machine)
}

/// Serialise the finished run. Every field is a MEASUREMENT — exit code and
/// streams come from the process that actually ran.
fn print_json_outcome(
    task_id: &str,
    prepared: &dispatch::PreparedDispatch,
    script: &str,
    out: &crate::transport::ExecOutput,
    duration_ms: u128,
) {
    println!(
        "{}",
        serde_json::json!({
            "task": task_id,
            "command": prepared.command,
            "script": script,
            "timeout_secs": prepared.timeout_secs,
            "exit_code": out.exit_code,
            "success": out.success(),
            "status": if out.success() { "pass" } else { "FAIL" },
            "stdout": out.stdout,
            "stderr": out.stderr,
            "duration_ms": duration_ms,
        })
    );
}

/// Human-readable banner, printed BEFORE execution so a hanging task is
/// attributable.
fn print_text_header(task_id: &str, prepared: &dispatch::PreparedDispatch) {
    println!("Running task: {task_id}");
    println!("  command: {}", prepared.command);
    if let Some(timeout) = prepared.timeout_secs {
        println!("  timeout: {timeout}s");
    }
}

fn print_text_outcome(out: &crate::transport::ExecOutput) {
    if out.success() {
        println!("  status: {}", green("pass"));
        if !out.stdout.is_empty() {
            println!("{}", out.stdout);
        }
    } else {
        println!("  status: {}", red("FAIL"));
        if !out.stderr.is_empty() {
            eprintln!("{}", out.stderr);
        }
    }
}

/// FJ-2700: Run a dispatch-mode task.
pub(crate) fn cmd_run(
    file: &Path,
    task_id: &str,
    param_strings: &[String],
    json: bool,
) -> Result<(), String> {
    let mut config = parse_and_validate(file)?;

    // `--param` is merged into `config.params`, which is what makes it visible
    // to BOTH spellings — `{{params.key}}` (resolver) and the bare `{{ key }}`
    // dispatch shorthand. It used to be parsed into a map nothing read.
    // The KEY=VALUE shape is validated here, with the same wording as before.
    super::apply_helpers::apply_param_overrides(&mut config, param_strings)?;

    let resource = select_task(&config, task_id)?;
    let resolved = resolve_task(&config, resource, task_id)?;

    let dispatch_config = build_dispatch_config(task_id, &resolved, &[]);
    dispatch::validate_dispatch(&dispatch_config)?;
    let prepared = dispatch::prepare_dispatch(&dispatch_config, &[]);
    let script = dispatch::dispatch_script(&prepared);

    if !json {
        print_text_header(task_id, &prepared);
    }

    let machine = task_machine(&config, &resolved);
    let started = std::time::Instant::now();
    let output = crate::transport::exec_script(&machine, &script);
    let duration_ms = started.elapsed().as_millis();

    let out = output.map_err(|e| format!("task '{task_id}' execution error: {e}"))?;

    if json {
        print_json_outcome(task_id, &prepared, &script, &out, duration_ms);
    } else {
        print_text_outcome(&out);
    }

    if out.success() {
        Ok(())
    } else {
        Err(format!(
            "task '{task_id}' failed with exit {}",
            out.exit_code
        ))
    }
}
