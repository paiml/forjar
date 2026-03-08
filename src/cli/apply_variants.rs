//! Apply dry-run variants.

use super::apply::*;
use super::apply_helpers::*;
use super::helpers::*;
use super::helpers_state::*;
use super::workspace::*;
use crate::core::{codegen, executor, planner, resolver, state, types};
use crate::transport;
use crate::tripwire::hasher;
use std::path::Path;

/// FJ-583: Show execution graph without applying.
pub(crate) fn cmd_apply_dry_run_graph(file: &Path) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    // Build and display the execution DAG
    let mut graph: Vec<(String, Vec<String>)> = Vec::new();
    for (name, res) in &config.resources {
        graph.push((name.clone(), res.depends_on.clone()));
    }
    graph.sort_by(|a, b| a.0.cmp(&b.0));

    println!("Execution graph (dry run):");
    println!("  {} resources", graph.len());
    println!();
    for (name, deps) in &graph {
        if deps.is_empty() {
            println!("  {name} (no dependencies — runs first)");
        } else {
            println!("  {} → depends on: {}", name, deps.join(", "));
        }
    }
    Ok(())
}

/// FJ-510: Canary machine — apply to single machine first, then remaining.
pub(crate) fn cmd_apply_canary_machine(
    file: &Path,
    state_dir: &Path,
    canary: &str,
    params: &[String],
    timeout: Option<u64>,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    if !config.machines.contains_key(canary) {
        return Err(format!(
            "canary machine '{}' not found (available: {})",
            canary,
            config
                .machines
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    println!("=== Canary: applying to '{canary}' first ===\n");
    cmd_apply(
        file,
        state_dir,
        Some(canary),
        None,
        None,
        None,
        false,
        false,
        false,
        params,
        false,
        timeout,
        false,
        false,
        None,
        None,
        false,
        false,
        None,
        false,
        false,
        0,
        true,
        false,
        None,
        false,
        None,
        None,
        None,
        false,
        None,
        false,
        None, // telemetry_endpoint
    )?;

    println!("\n{} Canary '{}' succeeded.", green("✓"), canary);

    let remaining: Vec<String> = config
        .machines
        .keys()
        .filter(|k| *k != canary)
        .cloned()
        .collect();

    if remaining.is_empty() {
        println!("No remaining machines. Canary deploy complete.");
        return Ok(());
    }

    println!(
        "\n=== Fleet: applying to {} remaining machines ===\n",
        remaining.len()
    );
    for machine_name in &remaining {
        cmd_apply(
            file,
            state_dir,
            Some(machine_name),
            None,
            None,
            None,
            false,
            false,
            false,
            params,
            false,
            timeout,
            false,
            false,
            None,
            None,
            false,
            false,
            None,
            false,
            false,
            0,
            true,
            false,
            None,
            false,
            None,
            None,
            None,
            false,
            None,
            false,
            None, // telemetry_endpoint
        )?;
    }

    println!(
        "\n{} Fleet deploy complete ({} machines).",
        green("✓"),
        remaining.len() + 1
    );
    Ok(())
}

/// FJ-1230: Refresh state only — re-query live state for all converged resources
/// and update lock hashes without applying any changes.
#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_refresh_only(
    file: &Path,
    state_dir: &Path,
    machine_filter: Option<&str>,
    verbose: bool,
    timeout: Option<u64>,
    env_file: Option<&Path>,
    workspace: Option<&str>,
) -> Result<(), String> {
    let mut config = parse_and_validate(file)?;
    if let Some(path) = env_file {
        load_env_params(&mut config, path)?;
    }
    inject_workspace_param(&mut config, workspace);
    resolver::resolve_data_sources(&mut config)?;

    let locks = load_machine_locks(&config, state_dir, machine_filter)?;
    let mut refreshed = 0usize;
    let mut drift_count = 0usize;

    for (machine_name, lock) in &locks {
        let machine = match config.machines.get(machine_name) {
            Some(m) => m,
            None => continue,
        };

        let mut updated_lock = lock.clone();
        for (id, rl) in &lock.resources {
            if rl.status != types::ResourceStatus::Converged {
                continue;
            }
            let resource = match config.resources.get(id) {
                Some(r) => r,
                None => continue,
            };
            let resolved =
                resolver::resolve_resource_templates(resource, &config.params, &config.machines)
                    .unwrap_or_else(|_| resource.clone());

            let new_hash = match codegen::state_query_script(&resolved) {
                Ok(query) => match transport::exec_script_timeout(machine, &query, timeout) {
                    Ok(out) if out.success() => Some(hasher::hash_string(&out.stdout)),
                    _ => None,
                },
                Err(_) => None,
            };

            if let Some(ref hash) = new_hash {
                let old_hash = rl
                    .details
                    .get("live_hash")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if hash != old_hash {
                    drift_count += 1;
                    if verbose {
                        eprintln!("  drift: {id} on {machine_name} (hash changed)");
                    }
                }
                if let Some(entry) = updated_lock.resources.get_mut(id) {
                    entry.details.insert(
                        "live_hash".to_string(),
                        serde_yaml_ng::Value::String(hash.clone()),
                    );
                }
                refreshed += 1;
            }
        }

        state::save_lock(state_dir, &updated_lock)?;
    }

    println!("Refresh complete: {refreshed} resources queried, {drift_count} drifted");
    Ok(())
}

/// FJ-536: Dry run cost — show estimated change count without applying.
pub(crate) fn cmd_apply_dry_run_cost(
    file: &Path,
    state_dir: &Path,
    machine: Option<&str>,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let order = resolver::build_execution_order(&config)?;
    let locks = load_machine_locks(&config, state_dir, machine)?;
    let plan = planner::plan(&config, &order, &locks, None);

    let creates = plan
        .changes
        .iter()
        .filter(|c| c.action == types::PlanAction::Create)
        .count();
    let updates = plan
        .changes
        .iter()
        .filter(|c| c.action == types::PlanAction::Update)
        .count();
    let deletes = plan
        .changes
        .iter()
        .filter(|c| c.action == types::PlanAction::Destroy)
        .count();
    let noops = plan
        .changes
        .iter()
        .filter(|c| c.action == types::PlanAction::NoOp)
        .count();

    println!("Dry run cost estimate:\n");
    println!("  Create:  {creates}");
    println!("  Update:  {updates}");
    println!("  Destroy: {deletes}");
    println!("  No-op:   {noops}");
    println!("  ─────────────");
    println!("  Total changes: {}", creates + updates + deletes);
    Ok(())
}

/// FJ-1250: Execute a previously saved plan file.
/// Validates config hash matches, then runs the planned changes.
#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_apply_from_plan(
    file: &Path,
    state_dir: &Path,
    plan_path: &Path,
    verbose: bool,
    env_file: Option<&Path>,
    workspace: Option<&str>,
) -> Result<(), String> {
    use super::plan::load_plan_file;

    let mut config = parse_and_validate(file)?;
    if let Some(path) = env_file {
        load_env_params(&mut config, path)?;
    }
    inject_workspace_param(&mut config, workspace);
    resolver::resolve_data_sources(&mut config)?;

    let plan = load_plan_file(plan_path, &config)?;
    let n_changes = plan.to_create + plan.to_update + plan.to_destroy;

    if verbose {
        eprintln!(
            "Executing saved plan: {} changes ({} create, {} update, {} destroy)",
            n_changes, plan.to_create, plan.to_update, plan.to_destroy
        );
    }

    if n_changes == 0 {
        println!("Plan has no changes to apply.");
        return Ok(());
    }

    // Execute as a normal apply using the plan's resource list
    let cfg = executor::ApplyConfig {
        config: &config,
        state_dir,
        force: false,
        dry_run: false,
        machine_filter: None,
        resource_filter: None,
        tag_filter: None,
        group_filter: None,
        timeout_secs: None,
        force_unlock: false,
        progress: false,
        retry: 0,
        parallel: None,
        resource_timeout: None,
        rollback_on_failure: false,
        max_parallel: None,
        trace: false,
        run_id: Some(crate::core::types::generate_run_id()),
    };

    let results = executor::apply(&cfg)?;
    let (converged, unchanged, failed) = super::apply_output::count_results(&results);

    println!("Plan applied: {converged} converged, {unchanged} unchanged, {failed} failed");

    if failed > 0 {
        return Err(format!("{failed} resource(s) failed"));
    }
    Ok(())
}
