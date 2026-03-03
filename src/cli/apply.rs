//! Apply command.

use super::apply_helpers::*;
use super::apply_output::*;
use super::helpers::*;
use super::helpers_state::*;
use super::workspace::*;
use crate::core::{executor, parser, planner, resolver, state, types};
use std::path::Path;

#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_apply(
    file: &Path,
    state_dir: &Path,
    machine_filter: Option<&str>,
    resource_filter: Option<&str>,
    tag_filter: Option<&str>,
    group_filter: Option<&str>,
    force: bool,
    dry_run: bool,
    no_tripwire: bool,
    param_overrides: &[String],
    auto_commit: bool,
    timeout_secs: Option<u64>,
    json: bool,
    verbose: bool,
    env_file: Option<&Path>,
    workspace: Option<&str>,
    report: bool,
    force_unlock: bool,
    output_mode: Option<&str>,
    progress: bool,
    timing: bool,
    retry: u32,
    yes: bool,
    parallel: bool,
    resource_timeout: Option<u64>,
    rollback_on_failure: bool,
    max_parallel: Option<usize>,
    notify: Option<&str>,
    subset: Option<&str>,
    confirm_destructive: bool,
    exclude: Option<&str>,
    _sequential: bool,
) -> Result<(), String> {
    use std::time::Instant;
    let t_total = Instant::now();

    let events_mode = output_mode == Some("events");
    let t_parse = Instant::now();
    let mut config = parse_and_validate(file)?;
    if let Some(path) = env_file {
        load_env_params(&mut config, path)?;
    }
    inject_workspace_param(&mut config, workspace);
    resolver::resolve_data_sources(&mut config)?;
    let dur_parse = t_parse.elapsed();

    if verbose {
        eprintln!(
            "Applying {} ({} machines, {} resources)",
            config.name,
            config.machines.len(),
            config.resources.len()
        );
    }
    if no_tripwire {
        config.policy.tripwire = false;
    }
    apply_param_overrides(&mut config, param_overrides)?;

    apply_filters(&mut config, subset, exclude, verbose)?;
    apply_pre_validate(
        &config,
        state_dir,
        machine_filter,
        tag_filter,
        confirm_destructive,
        dry_run,
        force,
        yes,
        verbose,
    )?;

    let cfg = executor::ApplyConfig {
        config: &config,
        state_dir,
        force,
        dry_run,
        machine_filter,
        resource_filter,
        tag_filter,
        group_filter,
        timeout_secs,
        force_unlock,
        progress,
        retry,
        parallel: if parallel { Some(true) } else { None },
        resource_timeout,
        rollback_on_failure,
        max_parallel,
    };

    maybe_auto_snapshot(&config, state_dir, dry_run, verbose);

    // FJ-1388: Record pre-apply generation for rollback-on-failure
    let pre_apply_gen = pre_apply_generation(state_dir);

    let t_apply = Instant::now();
    let results = executor::apply(&cfg)?;
    let dur_apply = t_apply.elapsed();

    if dry_run {
        return apply_dry_run_output(&config, state_dir, machine_filter, tag_filter, json);
    }

    let (total_converged, total_unchanged, total_failed) = count_results(&results);

    for result in &results {
        if let Err(e) = state::save_apply_report(state_dir, result) {
            eprintln!("warning: cannot save apply report: {}", e);
        }
    }

    if events_mode {
        return print_events_output(&results);
    }

    print_apply_summary(
        &config,
        &results,
        total_converged,
        total_unchanged,
        total_failed,
        dur_apply,
        json,
    )?;

    if report {
        print_resource_report(&results);
    }
    if timing {
        print_timing(dur_parse, dur_apply, t_total.elapsed());
    }
    check_convergence_budget(&config, dur_apply)?;
    if total_failed > 0 {
        // FJ-1388: Generation-based rollback on failure
        maybe_rollback_generation(rollback_on_failure, state_dir, pre_apply_gen, verbose);
        return Err(format!("{} resource(s) failed", total_failed));
    }

    apply_post_actions(
        state_dir,
        &config,
        &results,
        total_converged,
        auto_commit,
        verbose,
        notify,
        &t_total,
    )?;

    Ok(())
}

/// Apply subset and exclude filters to config.
fn apply_filters(
    config: &mut types::ForjarConfig,
    subset: Option<&str>,
    exclude: Option<&str>,
    verbose: bool,
) -> Result<(), String> {
    if let Some(pattern) = subset {
        config
            .resources
            .retain(|id, _| simple_glob_match(pattern, id));
        if config.resources.is_empty() {
            return Err(format!("no resources match subset pattern '{}'", pattern));
        }
        if verbose {
            eprintln!(
                "Subset filter '{}': {} resources selected",
                pattern,
                config.resources.len()
            );
        }
    }
    if let Some(pattern) = exclude {
        let before = config.resources.len();
        config
            .resources
            .retain(|id, _| !simple_glob_match(pattern, id));
        if verbose {
            eprintln!(
                "Exclude filter '{}': removed {} resources ({} remaining)",
                pattern,
                before - config.resources.len(),
                config.resources.len()
            );
        }
    }
    Ok(())
}

/// FJ-1270: Check state file integrity via BLAKE3 sidecars.
fn check_state_integrity(state_dir: &Path, verbose: bool, yes: bool) -> Result<(), String> {
    if !state_dir.exists() {
        return Ok(());
    }
    let issues = state::integrity::verify_state_integrity(state_dir);
    state::integrity::print_issues(&issues, verbose);
    if state::integrity::has_errors(&issues) && !yes {
        return Err(
            "state integrity check failed — use --yes to override or fix corrupted files"
                .to_string(),
        );
    }
    Ok(())
}

/// FJ-1381: Auto-snapshot before apply if snapshot_generations is set.
fn maybe_auto_snapshot(
    config: &types::ForjarConfig,
    state_dir: &Path,
    dry_run: bool,
    verbose: bool,
) {
    let Some(gens) = config.policy.snapshot_generations else {
        return;
    };
    if gens == 0 || dry_run || !state_dir.exists() {
        return;
    }
    let snap_name = format!("pre-apply-{}", crate::tripwire::eventlog::now_iso8601());
    if let Err(e) = super::snapshot::cmd_snapshot_save(&snap_name, state_dir) {
        eprintln!("warning: pre-apply snapshot failed: {e}");
    } else if verbose {
        eprintln!("snapshot: saved {snap_name}");
    }
    gc_old_snapshots(state_dir, gens, verbose);

    // FJ-1386: Also create a numbered generation for instant rollback
    match super::generation::create_generation(state_dir) {
        Ok(gen) => {
            if verbose {
                eprintln!("generation: created gen {gen}");
            }
            super::generation::gc_generations(state_dir, gens, verbose);
        }
        Err(e) => eprintln!("warning: generation creation failed: {e}"),
    }
}

/// FJ-1381: Garbage-collect old snapshots, keeping only the newest `keep` snapshots.
fn gc_old_snapshots(state_dir: &Path, keep: u32, verbose: bool) {
    let snap_dir = super::snapshot::snapshots_dir(state_dir);
    if !snap_dir.exists() {
        return;
    }
    let mut entries: Vec<_> = match std::fs::read_dir(&snap_dir) {
        Ok(e) => e.flatten().filter(|e| e.path().is_dir()).collect(),
        Err(_) => return,
    };
    if entries.len() <= keep as usize {
        return;
    }
    entries.sort_by_key(|e| e.file_name());
    let to_remove = entries.len() - keep as usize;
    for entry in entries.iter().take(to_remove) {
        if verbose {
            eprintln!(
                "snapshot gc: removing {}",
                entry.file_name().to_string_lossy()
            );
        }
        let _ = std::fs::remove_dir_all(entry.path());
    }
}

/// FJ-1388: Get the current generation number before apply starts.
fn pre_apply_generation(state_dir: &Path) -> Option<u32> {
    let gen_dir = state_dir.join("generations");
    super::generation::current_generation(&gen_dir)
}

/// FJ-1388: Rollback to pre-apply generation on failure.
fn maybe_rollback_generation(
    rollback_on_failure: bool,
    state_dir: &Path,
    pre_apply_gen: Option<u32>,
    verbose: bool,
) {
    if !rollback_on_failure {
        return;
    }
    let Some(gen) = pre_apply_gen else { return };
    eprintln!("rollback: restoring state to generation {gen}");
    if let Err(e) = super::generation::rollback_to_generation(state_dir, gen, true) {
        eprintln!("warning: generation rollback failed: {e}");
    } else if verbose {
        eprintln!("rollback: restored to generation {gen}");
    }
}

/// FJ-1380: Check convergence budget — warn/fail if apply exceeded time budget.
fn check_convergence_budget(
    config: &types::ForjarConfig,
    dur_apply: std::time::Duration,
) -> Result<(), String> {
    if let Some(budget_secs) = config.policy.convergence_budget {
        let elapsed = dur_apply.as_secs();
        if elapsed > budget_secs {
            eprintln!(
                "ERROR: convergence budget exceeded — budget {}s, actual {}s",
                budget_secs, elapsed
            );
            return Err(format!(
                "convergence budget exceeded: {}s > {}s",
                elapsed, budget_secs
            ));
        }
    }
    Ok(())
}

/// FJ-1378: Pre-apply drift gate — block apply if live state has drifted.
/// Uses local file hashing only (no SSH). Skip with --force or --no-tripwire.
fn check_pre_apply_drift(
    config: &types::ForjarConfig,
    state_dir: &Path,
    machine_filter: Option<&str>,
    force: bool,
    verbose: bool,
) -> Result<(), String> {
    if !config.policy.tripwire || force {
        return Ok(());
    }
    let locks = load_machine_locks(config, state_dir, machine_filter)?;
    let mut total_drift = 0usize;
    for (machine_name, lock) in &locks {
        let findings = crate::tripwire::drift::detect_drift(lock);
        if !findings.is_empty() {
            total_drift += findings.len();
            for f in &findings {
                eprintln!(
                    "  drift: [{}] {} — {}",
                    machine_name, f.resource_id, f.detail
                );
            }
        }
    }
    if total_drift > 0 {
        if verbose {
            eprintln!(
                "{} resource(s) drifted — run 'forjar drift' for details",
                total_drift
            );
        }
        return Err(format!(
            "{} drift finding(s) block apply — use --force to override",
            total_drift
        ));
    }
    Ok(())
}

/// Pre-apply validation: policies, confirmation, hooks.
#[allow(clippy::too_many_arguments)]
fn apply_pre_validate(
    config: &types::ForjarConfig,
    state_dir: &Path,
    machine_filter: Option<&str>,
    tag_filter: Option<&str>,
    confirm_destructive: bool,
    dry_run: bool,
    force: bool,
    yes: bool,
    verbose: bool,
) -> Result<(), String> {
    check_state_integrity(state_dir, verbose, yes)?;
    check_pre_apply_drift(config, state_dir, machine_filter, force, verbose)?;

    // FJ-335: Confirm destructive actions
    if confirm_destructive && !dry_run && !yes {
        let order = resolver::build_execution_order(config)?;
        let cd_locks = load_machine_locks(config, state_dir, machine_filter)?;
        let plan = planner::plan(config, &order, &cd_locks, tag_filter);
        let destroy_count = plan
            .changes
            .iter()
            .filter(|p| p.action == types::PlanAction::Destroy)
            .count();
        if destroy_count > 0 {
            eprintln!(
                "WARNING: {} resource(s) will be DESTROYED. Use --yes to confirm.",
                destroy_count
            );
            return Err(format!(
                "{} destructive action(s) blocked by --confirm-destructive",
                destroy_count
            ));
        }
    }

    check_policy_violations(config)?;
    check_security_gate(config)?;

    // Run pre_apply hook
    if let Some(ref hook) = config.policy.pre_apply {
        if !dry_run {
            run_hook("pre_apply", hook, verbose)?;
        }
    }

    // FJ-286: Confirmation prompt
    if !yes && !dry_run {
        let execution_order = resolver::build_execution_order(config)?;
        let preview_locks = load_machine_locks(config, state_dir, machine_filter)?;
        let preview_plan = planner::plan(config, &execution_order, &preview_locks, tag_filter);
        let n_changes = preview_plan.to_create + preview_plan.to_update + preview_plan.to_destroy;
        if n_changes > 0 {
            eprint!(
                "Apply {} change(s) ({} create, {} update, {} destroy)? [y/N] ",
                n_changes, preview_plan.to_create, preview_plan.to_update, preview_plan.to_destroy
            );
            let mut answer = String::new();
            std::io::stdin()
                .read_line(&mut answer)
                .map_err(|e| format!("stdin error: {}", e))?;
            if !answer.trim().eq_ignore_ascii_case("y") {
                return Err("aborted by user".to_string());
            }
        }
    }

    Ok(())
}

/// FJ-220: Check policy rules and block apply if any deny/require violations exist.
fn check_policy_violations(config: &types::ForjarConfig) -> Result<(), String> {
    if config.policies.is_empty() {
        return Ok(());
    }
    let violations = parser::evaluate_policies(config);
    let is_deny = |v: &types::PolicyViolation| {
        matches!(
            v.severity,
            types::PolicyRuleType::Deny | types::PolicyRuleType::Require
        )
    };
    let deny_count = violations.iter().filter(|v| is_deny(v)).count();
    if deny_count == 0 {
        return Ok(());
    }
    for v in &violations {
        let sev = if is_deny(v) { "DENY" } else { "WARN" };
        eprintln!("  [{sev}] {}: {}", v.resource_id, v.rule_message);
    }
    Err(format!(
        "policy violations block apply ({deny_count} denied)"
    ))
}

/// FJ-1390: Run security scanner as pre-apply gate if policy.security_gate is set.
fn check_security_gate(config: &types::ForjarConfig) -> Result<(), String> {
    let threshold = match &config.policy.security_gate {
        Some(t) => t.clone(),
        None => return Ok(()),
    };
    let findings = crate::core::security_scanner::scan(config);
    if findings.is_empty() {
        return Ok(());
    }
    let (crit, high, med, _low) = crate::core::security_scanner::severity_counts(&findings);
    let should_fail = match threshold.to_lowercase().as_str() {
        "critical" => crit > 0,
        "high" => crit + high > 0,
        "medium" => crit + high + med > 0,
        "low" => !findings.is_empty(),
        _ => return Err(format!("unknown security_gate severity: {threshold}")),
    };
    if !should_fail {
        return Ok(());
    }
    for f in &findings {
        eprintln!(
            "  [{:?}] {} ({}): {}",
            f.severity, f.rule_id, f.resource_id, f.message
        );
    }
    Err(format!(
        "security gate blocks apply: {} findings at or above '{threshold}'",
        findings.len()
    ))
}
