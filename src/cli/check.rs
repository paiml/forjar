//! Pre-condition checks.

pub(crate) use super::check_test::cmd_test;

use super::helpers::*;
use crate::core::{codegen, resolver, state, types};
use crate::transport;
use std::collections::HashMap;
use std::path::Path;

/// Single check result for accumulation.
pub(super) struct CheckResult {
    pub(super) resource_id: String,
    pub(super) machine: String,
    pub(super) status: String,
    pub(super) exit_code: Option<i32>,
    pub(super) detail: String,
    /// FJ-178: whether the resource is recorded in the state dir for its
    /// machine. `None` when there is no recorded state for the machine
    /// (e.g. the default `state` dir is absent — same as historical behavior).
    pub(super) in_state: Option<bool>,
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
        in_state: None,
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
        // FJ-2720: exit 2 = NOT APPLICABLE on this host (no systemd, no docker).
        // Neither a pass nor a failure — forjar cannot observe the state, and
        // claiming either would be a guess.
        Ok(out) if out.exit_code == 2 => ("skip", Some(2), out.stdout.trim().to_string(), true),
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
            if let Some(in_state) = r.in_state {
                obj["in_state"] = serde_json::json!(in_state);
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

/// FJ-178: Per-machine state locks plus whether the state dir exists at all.
pub(super) struct CheckState {
    /// Whether `state_dir` exists on disk. When absent (e.g. the default
    /// `state` dir was never created), checks stay state-agnostic so behavior
    /// is identical to the historical, state-unaware `check`.
    pub(super) dir_exists: bool,
    /// Machine name -> recorded state lock (only for machines with a lock).
    pub(super) locks: HashMap<String, types::StateLock>,
}

/// FJ-178: Load the per-machine state locks from `state_dir` for the machines
/// referenced by the config.
pub(super) fn load_check_locks(state_dir: &Path, config: &types::ForjarConfig) -> CheckState {
    let mut locks = HashMap::new();
    for machine_name in config.machines.keys() {
        if let Ok(Some(lock)) = state::load_lock(state_dir, machine_name) {
            locks.insert(machine_name.clone(), lock);
        }
    }
    CheckState {
        dir_exists: state_dir.exists(),
        locks,
    }
}

/// FJ-178: Whether `resource_id` is recorded for `machine_name` in the state.
/// `None` when the state dir does not exist (state-agnostic, default behavior);
/// `Some(true)` when recorded; `Some(false)` when the dir exists but the
/// resource is not recorded for the machine (config/state drift).
pub(super) fn resource_in_state(
    state: &CheckState,
    machine_name: &str,
    resource_id: &str,
) -> Option<bool> {
    if !state.dir_exists {
        return None;
    }
    let recorded = state
        .locks
        .get(machine_name)
        .is_some_and(|lock| lock.resources.contains_key(resource_id));
    Some(recorded)
}

/// FJ-178: In text mode, warn when a checked resource is absent from the
/// recorded state for its machine (drift between config and state dir).
fn report_unrecorded(in_state: Option<bool>, resource_id: &str, machine_name: &str, json: bool) {
    if json {
        return;
    }
    if in_state == Some(false) {
        println!("  ! {resource_id} ({machine_name}) — not recorded in state");
    }
}

/// The running totals and per-check results accumulated by `cmd_check`.
#[derive(Default)]
struct CheckTally {
    pass: usize,
    fail: usize,
    skip: usize,
    results: Vec<CheckResult>,
}

/// The parts of `cmd_check`'s environment that do not vary per resource.
struct CheckCtx<'a> {
    machines: &'a indexmap::IndexMap<String, types::Machine>,
    localhost: types::Machine,
    machine_filter: Option<&'a str>,
    state: CheckState,
    json: bool,
}

/// The check script for `resource`, or `None` when the resource has nothing to
/// observe. Both no-script cases print their reason in text mode; the caller
/// counts the skip.
fn check_script_or_skip(
    resource_id: &str,
    resource: &types::Resource,
    resolved: &types::Resource,
    json: bool,
) -> Option<String> {
    // FJ-2725: a phony resource names an action with no artifact. Since
    // FJ-2720 made "no evidence" a failure, checking one would report a
    // permanent FAIL for something that has nothing to observe.
    if resource.phony {
        if !json {
            println!("  ? {resource_id} (phony — nothing to observe)");
        }
        return None;
    }
    match codegen::check_script(resolved) {
        Ok(s) => Some(s),
        Err(_) => {
            if !json {
                println!("  ? {resource_id} (no check script)");
            }
            None
        }
    }
}

/// Run `check_script` on every machine the resource targets, folding each
/// outcome into `tally`.
fn check_resource_machines(
    ctx: &CheckCtx<'_>,
    resource: &types::Resource,
    resource_id: &str,
    check_script: &str,
    tally: &mut CheckTally,
) {
    for machine_name in resource.machine.iter() {
        let machine = ctx.machines.get(machine_name).unwrap_or(&ctx.localhost);
        if skip_machine(machine_name, ctx.machine_filter, resource, machine) {
            tally.skip += 1;
            continue;
        }

        let (mut result, passed) =
            run_single_check(machine, check_script, resource_id, machine_name, ctx.json);
        result.in_state = resource_in_state(&ctx.state, machine_name, resource_id);
        report_unrecorded(result.in_state, resource_id, machine_name, ctx.json);
        if passed {
            tally.pass += 1;
        } else {
            tally.fail += 1;
        }
        tally.results.push(result);
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_check(
    file: &Path,
    machine_filter: Option<&str>,
    resource_filter: Option<&str>,
    tag_filter: Option<&str>,
    state_dir: &Path,
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
    let ctx = CheckCtx {
        machines: &config.machines,
        localhost: localhost_machine(),
        machine_filter,
        state: load_check_locks(state_dir, &config),
        json,
    };

    let mut tally = CheckTally::default();

    for resource_id in &execution_order {
        let resource = match config.resources.get(resource_id) {
            Some(r) => r,
            None => continue,
        };

        let (skip, count) =
            check_resource_filters(resource_id, resource, resource_filter, tag_filter);
        if skip {
            if count {
                tally.skip += 1;
            }
            continue;
        }

        let resolved =
            resolver::resolve_resource_templates(resource, &config.params, &config.machines)?;

        let Some(check_script) = check_script_or_skip(resource_id, resource, &resolved, json)
        else {
            tally.skip += 1;
            continue;
        };

        check_resource_machines(&ctx, resource, resource_id, &check_script, &mut tally);
    }

    let CheckTally {
        pass: total_pass,
        fail: total_fail,
        skip: total_skip,
        results: check_results,
    } = tally;

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
