//! Apply dry-run variants.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::apply::*;


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
            println!("  {} (no dependencies — runs first)", name);
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

    println!("=== Canary: applying to '{}' first ===\n", canary);
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
        )?;
    }

    println!(
        "\n{} Fleet deploy complete ({} machines).",
        green("✓"),
        remaining.len() + 1
    );
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
    println!("  Create:  {}", creates);
    println!("  Update:  {}", updates);
    println!("  Destroy: {}", deletes);
    println!("  No-op:   {}", noops);
    println!("  ─────────────");
    println!("  Total changes: {}", creates + updates + deletes);
    Ok(())
}

