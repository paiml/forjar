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
        None,  // telemetry_endpoint
        false, // refresh
        None,  // force_tag
        &[],
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
            None,  // telemetry_endpoint
            false, // refresh
            None,  // force_tag
            &[],
        )?;
    }

    println!(
        "\n{} Fleet deploy complete ({} machines).",
        green("✓"),
        remaining.len() + 1
    );
    Ok(())
}

/// Re-query ONE converged resource's live state and hash it the same way the
/// executor did when it recorded the stored `live_hash`.
///
/// FJ-154 / #22: resolve with the SAME SecretsConfig the executor used to
/// produce the stored live_hash (record_success →
/// resolve_resource_templates_with_secrets(.., &cfg.config.secrets)), so the
/// refresh-query script matches and we don't report spurious drift / rewrite
/// state on every refresh.
///
/// `None` means "no answer" — the resource has no refresh-query script, or the
/// query did not succeed — and the caller then leaves the stored hash untouched.
fn refreshed_live_hash(
    machine: &types::Machine,
    resource: &types::Resource,
    config: &types::ForjarConfig,
    timeout: Option<u64>,
) -> Option<String> {
    let resolved = resolver::resolve_resource_templates_with_secrets(
        resource,
        &config.params,
        &config.machines,
        &config.secrets,
    )
    .unwrap_or_else(|_| resource.clone());

    // STRONG contract: refresh-query stdout may legitimately be empty when
    // state is absent — use the sentinel wrapper to uphold `!input.is_empty()`.
    let query = codegen::state_query_script(&resolved).ok()?;
    match transport::exec_script_timeout(machine, &query, timeout) {
        Ok(out) if out.success() => Some(hasher::hash_string_or_sentinel(&out.stdout)),
        _ => None,
    }
}

/// Live hash for a lock entry that is still eligible for refresh: it must be
/// recorded as converged and still be present in the config. `None` means the
/// entry is skipped — either it is not converged, the resource was removed from
/// the config, or the live query did not answer.
fn refreshable_live_hash(
    config: &types::ForjarConfig,
    machine: &types::Machine,
    id: &str,
    rl: &types::ResourceLock,
    timeout: Option<u64>,
) -> Option<String> {
    if rl.status != types::ResourceStatus::Converged {
        return None;
    }
    let resource = config.resources.get(id)?;
    refreshed_live_hash(machine, resource, config, timeout)
}

/// True when a freshly queried hash differs from the OBSERVED state already
/// recorded on the lock entry (an absent recording counts as a difference
/// unless the new hash is empty, matching the pre-refactor comparison).
///
/// Reads through `observed_state()` rather than `details["live_hash"]`: #338
/// split SPEC from STATUS and the accessor prefers the typed `observed` field,
/// so a raw `details` read here would disagree with the drift path.
fn observed_state_drifted(rl: &types::ResourceLock, hash: &str) -> bool {
    // Compare against the OBSERVED state through the accessor, so this path and
    // the drift path agree on where that value lives.
    let old_hash = rl.observed_state().unwrap_or("");
    hash != old_hash
}

/// Re-queries every refreshable resource of one machine, returning the lock with
/// updated observed state plus (queried, drifted) counts.
fn refresh_machine_lock(
    config: &types::ForjarConfig,
    machine: &types::Machine,
    machine_name: &str,
    lock: &types::StateLock,
    timeout: Option<u64>,
    verbose: bool,
) -> (types::StateLock, usize, usize) {
    let mut updated_lock = lock.clone();
    let mut refreshed = 0usize;
    let mut drift_count = 0usize;

    for (id, rl) in &lock.resources {
        let Some(hash) = refreshable_live_hash(config, machine, id, rl, timeout) else {
            continue;
        };
        if observed_state_drifted(rl, &hash) {
            drift_count += 1;
            if verbose {
                eprintln!("  drift: {id} on {machine_name} (hash changed)");
            }
        }
        if let Some(entry) = updated_lock.resources.get_mut(id) {
            // MUST go through the setter. Writing only `details` here would
            // leave the typed `observed` field holding the PREVIOUS digest, and
            // `observed_state()` prefers the typed field — so `--refresh` would
            // update one of two copies and every later reader would see the
            // stale one. That is forjar#305's exact shape (two stores, readers
            // split between them), which this refactor exists to remove.
            entry.set_observed_state(hash);
        }
        refreshed += 1;
    }

    (updated_lock, refreshed, drift_count)
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
        let Some(machine) = config.machines.get(machine_name) else {
            continue;
        };
        let (updated_lock, machine_refreshed, machine_drift) =
            refresh_machine_lock(&config, machine, machine_name, lock, timeout, verbose);
        refreshed += machine_refreshed;
        drift_count += machine_drift;
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
///
/// # forjar#370: this is an apply, so it is authorized like one
///
/// `dispatch_apply_b::apply_mode_exits` returns here for `--plan-file` BEFORE
/// `apply_execute`, whose first line is `check_operator_auth`. So the gate was
/// reachable only on the ordinary path, and the whole of it was skippable by
/// routing through a plan file. Measured on 1.21.0 with
/// `allowed_operators: [alice]`, as a non-alice operator:
///
/// ```text
///   forjar apply --yes                              -> not authorized   EXIT=1
///   forjar plan --out p.json                        -> EXIT=0
///   forjar apply --plan-file p.json --yes           -> 2 converged      EXIT=0
///   forjar apply --plan-file p2.json --operator mallory --yes -> applied EXIT=0
/// ```
///
/// A plan file is unauthenticated — any user can write one in a text editor —
/// so the bypass needed no privilege at all. The check lives at the TOP of this
/// function, not at the call site, so it holds for every caller of the
/// plan-file executor rather than for the one dispatcher that remembered.
///
/// `forjar plan --out` is deliberately NOT gated; see `cli::plan::cmd_plan`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_apply_from_plan(
    file: &Path,
    state_dir: &Path,
    plan_path: &Path,
    verbose: bool,
    env_file: Option<&Path>,
    workspace: Option<&str>,
    operator: Option<&str>,
) -> Result<(), String> {
    use super::plan_file::load_plan_file;

    // forjar#370: FIRST, and before the plan file is even read — the same
    // position and the same function `apply_execute` uses.
    super::dispatch_apply::check_operator_auth(file, operator)?;

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
        refresh: false,
        force_tag: None,
    };

    let results = executor::apply(&cfg)?;
    let (converged, unchanged, failed) = super::apply_output::count_results(&results);

    println!("Plan applied: {converged} converged, {unchanged} unchanged, {failed} failed");

    if failed > 0 {
        return Err(format!("{failed} resource(s) failed"));
    }
    Ok(())
}
