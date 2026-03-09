//! FJ-2700: `forjar run` — dispatch-mode task invocation.
//!
//! Finds a task resource with `mode: dispatch`, prepares the command
//! with param overrides, and executes it.

use super::helpers::*;
use crate::core::task::dispatch;
use crate::core::types::{self, DispatchConfig};
use std::path::Path;

/// Parse `key=value` param strings into tuples.
fn parse_params(params: &[String]) -> Result<Vec<(String, String)>, String> {
    params
        .iter()
        .map(|p| {
            let (k, v) = p
                .split_once('=')
                .ok_or_else(|| format!("invalid param '{p}': expected KEY=VALUE"))?;
            Ok((k.to_string(), v.to_string()))
        })
        .collect()
}

/// Build a DispatchConfig from a task resource.
fn build_dispatch_config(task_id: &str, resource: &types::Resource) -> DispatchConfig {
    let command = resource.command.clone().unwrap_or_default();
    DispatchConfig {
        name: task_id.to_string(),
        command,
        params: Vec::new(),
        timeout_secs: None,
    }
}

/// FJ-2700: Run a dispatch-mode task.
pub(crate) fn cmd_run(
    file: &Path,
    task_id: &str,
    param_strings: &[String],
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;

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

    let overrides = parse_params(param_strings)?;
    let dispatch_config = build_dispatch_config(task_id, resource);
    dispatch::validate_dispatch(&dispatch_config)?;
    let prepared = dispatch::prepare_dispatch(&dispatch_config, &overrides);
    let script = dispatch::dispatch_script(&prepared);

    if json {
        println!(
            "{}",
            serde_json::json!({
                "task": task_id,
                "command": prepared.command,
                "script": script,
                "timeout_secs": prepared.timeout_secs,
            })
        );
        return Ok(());
    }

    println!("Running task: {task_id}");
    println!("  command: {}", prepared.command);
    if let Some(timeout) = prepared.timeout_secs {
        println!("  timeout: {timeout}s");
    }

    // Execute via transport
    let machine_name = resource
        .machine
        .iter()
        .next()
        .map(|s| s.to_owned())
        .unwrap_or_default();
    let machine = config
        .machines
        .get(&machine_name)
        .cloned()
        .unwrap_or_else(super::check::localhost_machine);

    let output = crate::transport::exec_script(&machine, &script);
    match output {
        Ok(out) if out.success() => {
            println!("  status: {}", green("pass"));
            if !out.stdout.is_empty() {
                println!("{}", out.stdout);
            }
            Ok(())
        }
        Ok(out) => {
            println!("  status: {}", red("FAIL"));
            if !out.stderr.is_empty() {
                eprintln!("{}", out.stderr);
            }
            Err(format!(
                "task '{task_id}' failed with exit {}",
                out.exit_code
            ))
        }
        Err(e) => Err(format!("task '{task_id}' execution error: {e}")),
    }
}
