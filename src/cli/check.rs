//! Pre-condition checks.

pub(crate) use super::check_test::cmd_test;

use super::helpers::*;
use crate::core::{codegen, resolver, types};
use crate::transport;
use std::path::Path;

/// Single check result for accumulation.
pub(super) struct CheckResult {
    pub(super) resource_id: String,
    pub(super) machine: String,
    pub(super) status: String,
    pub(super) exit_code: Option<i32>,
    pub(super) detail: String,
}

/// Whether a resource matches the name and tag filters.
/// Returns (skip, count_as_skip) — skip=true means skip this resource,
/// count_as_skip=true means increment the skip counter (tag mismatch counts, name mismatch doesn't).
pub(super) fn check_resource_filters(
    resource_id: &str,
    resource: &types::Resource,
    resource_filter: Option<&str>,
    tag_filter: Option<&str>,
) -> (bool, bool) {
    if let Some(filter) = resource_filter {
        if resource_id != filter {
            return (true, false);
        }
    }
    if let Some(tag) = tag_filter {
        if !resource.tags.iter().any(|t| t == tag) {
            return (true, true);
        }
    }
    (false, false)
}

/// Whether to skip a machine based on filters.
pub(super) fn skip_machine(
    machine_name: &str,
    machine_filter: Option<&str>,
    resource: &types::Resource,
    machine: &types::Machine,
) -> bool {
    if let Some(filter) = machine_filter {
        if machine_name != filter {
            return true;
        }
    }
    if !resource.arch.is_empty() && !resource.arch.contains(&machine.arch) {
        return true;
    }
    false
}

/// Default localhost machine for resources without explicit machines.
pub(super) fn localhost_machine() -> types::Machine {
    types::Machine {
        hostname: "localhost".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    }
}

/// Ensure container is running if needed.
fn ensure_container_if_needed(machine: &types::Machine) -> Result<(), String> {
    if machine.is_container_transport() {
        transport::container::ensure_container(machine)?;
    }
    Ok(())
}

/// Build a CheckResult from status/exit_code/detail.
pub(super) fn make_check_result(
    resource_id: &str,
    machine_name: &str,
    status: &str,
    exit_code: Option<i32>,
    detail: String,
) -> CheckResult {
    CheckResult {
        resource_id: resource_id.to_string(),
        machine: machine_name.to_string(),
        status: status.to_string(),
        exit_code,
        detail,
    }
}

/// Print check failure in text mode.
fn print_check_failure(resource_id: &str, machine_name: &str, detail: &str) {
    println!("  FAIL {resource_id} ({machine_name}) — {detail}");
}

/// Run a check script on one machine and record the result.
fn run_single_check(
    machine: &types::Machine,
    check_script: &str,
    resource_id: &str,
    machine_name: &str,
    json: bool,
) -> (CheckResult, bool) {
    if let Err(e) = ensure_container_if_needed(machine) {
        if !json {
            print_check_failure(resource_id, machine_name, &e);
        }
        return (
            make_check_result(resource_id, machine_name, "error", None, e),
            false,
        );
    }

    let output = transport::exec_script(machine, check_script);
    let (status, exit_code, detail, passed) = match output {
        Ok(out) if out.success() => ("pass", Some(0), String::new(), true),
        Ok(out) => (
            "fail",
            Some(out.exit_code),
            out.stderr.trim().to_string(),
            false,
        ),
        Err(e) => ("error", None, e, false),
    };

    if !json {
        if passed {
            println!("  ok {resource_id} ({machine_name})");
        } else {
            let msg = if let Some(code) = exit_code {
                format!("exit {code}")
            } else {
                detail.clone()
            };
            print_check_failure(resource_id, machine_name, &msg);
            if exit_code.is_some() {
                for line in detail.lines().filter(|l| !l.is_empty()) {
                    println!("       {line}");
                }
            }
        }
    }

    (
        make_check_result(resource_id, machine_name, status, exit_code, detail),
        passed,
    )
}

/// Format check results as JSON.
pub(super) fn format_check_json(
    config_name: &str,
    results: &[CheckResult],
    total_pass: usize,
    total_fail: usize,
    total_skip: usize,
) -> Result<(), String> {
    let json_results: Vec<_> = results
        .iter()
        .map(|r| {
            let mut obj = serde_json::json!({
                "resource": r.resource_id,
                "machine": r.machine,
                "status": r.status,
            });
            if let Some(code) = r.exit_code {
                obj["exit_code"] = serde_json::json!(code);
            }
            if !r.detail.is_empty() {
                let key = if r.status == "error" {
                    "error"
                } else {
                    "stderr"
                };
                obj[key] = serde_json::json!(r.detail);
            }
            obj
        })
        .collect();
    let report = serde_json::json!({
        "name": config_name,
        "all_passed": total_fail == 0,
        "total": total_pass + total_fail + total_skip,
        "pass": total_pass,
        "fail": total_fail,
        "skip": total_skip,
        "results": json_results,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {e}"))?
    );
    Ok(())
}

pub(crate) fn cmd_check(
    file: &Path,
    machine_filter: Option<&str>,
    resource_filter: Option<&str>,
    tag_filter: Option<&str>,
    json: bool,
    verbose: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    if verbose {
        eprintln!(
            "Checking {} ({} machines, {} resources)",
            config.name,
            config.machines.len(),
            config.resources.len()
        );
    }

    let execution_order = resolver::build_execution_order(&config)?;
    let localhost = localhost_machine();

    let mut total_pass = 0usize;
    let mut total_fail = 0usize;
    let mut total_skip = 0usize;
    let mut check_results = Vec::new();

    for resource_id in &execution_order {
        let resource = match config.resources.get(resource_id) {
            Some(r) => r,
            None => continue,
        };

        let (skip, count) =
            check_resource_filters(resource_id, resource, resource_filter, tag_filter);
        if skip {
            if count {
                total_skip += 1;
            }
            continue;
        }

        let resolved =
            resolver::resolve_resource_templates(resource, &config.params, &config.machines)?;

        let check_script = match codegen::check_script(&resolved) {
            Ok(s) => s,
            Err(_) => {
                total_skip += 1;
                if !json {
                    println!("  ? {resource_id} (no check script)");
                }
                continue;
            }
        };

        for machine_name in resource.machine.iter() {
            let machine = config.machines.get(machine_name).unwrap_or(&localhost);
            if skip_machine(machine_name, machine_filter, resource, machine) {
                total_skip += 1;
                continue;
            }

            let (result, passed) =
                run_single_check(machine, &check_script, resource_id, machine_name, json);
            if passed {
                total_pass += 1;
            } else {
                total_fail += 1;
            }
            check_results.push(result);
        }
    }

    if json {
        format_check_json(
            &config.name,
            &check_results,
            total_pass,
            total_fail,
            total_skip,
        )?;
    } else {
        println!("\nCheck: {total_pass} pass, {total_fail} fail, {total_skip} skip");
    }

    if total_fail > 0 {
        Err(format!("{total_fail} check(s) failed"))
    } else {
        Ok(())
    }
}
