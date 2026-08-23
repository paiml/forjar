//! Apply command.

use super::apply_helpers::*;
use super::apply_output::*;
use super::apply_scope::{apply_scope, ApplyScope};
use super::apply_selection::*;
use super::helpers::*;
use super::helpers_state::*;
use super::workspace::*;
use crate::core::{executor, parser, planner, resolver, state, types};
use std::path::Path;

pub(crate) use super::apply_scope::cmd_apply;

#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_apply_scoped(
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
    sequential: bool,
    telemetry_endpoint: Option<&str>,
    refresh: bool,
    force_tag: Option<&str>,
    // FJ-2724: `make`-style goals. Empty means "the whole config".
    goals: &[String],
    // GH-211: --skip / --only-machine / --exclude-machine / --resource-filter.
    scope: &ApplyScope,
) -> Result<(), String> {
    warn_sequential_ignored(sequential);

    use std::time::Instant;
    let t_total = Instant::now();

    let events_mode = output_mode == Some("events");
    let t_parse = Instant::now();
    let mut config = load_apply_config(file, env_file, workspace)?;
    let dur_parse = t_parse.elapsed();

    print_apply_banner(&config, verbose);
    if no_tripwire {
        config.policy.tripwire = false;
    }
    apply_param_overrides(&mut config, param_overrides)?;

    reject_empty_selection(&config, resource_filter, tag_filter, group_filter)?;
    apply_scope(&mut config, scope, verbose)?;
    apply_goal_closure(&mut config, goals, verbose)?;
    strip_unrequested_phony(&mut config, goals);
    apply_filters(&mut config, subset, exclude, verbose)?;
    apply_pre_validate(
        &config,
        state_dir,
        machine_filter,
        tag_filter,
        resource_filter,
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
        parallel: super::apply_gates::parallel_flag(parallel),
        resource_timeout,
        rollback_on_failure,
        max_parallel,
        trace: verbose,
        run_id: apply_run_id(dry_run),
        refresh,
        force_tag,
    };

    maybe_auto_snapshot(&config, state_dir, Some(file), dry_run, verbose);

    // FJ-1388: Record pre-apply generation for rollback-on-failure
    let pre_apply_gen = pre_apply_generation(state_dir);

    // GH-210 (FJ-129): measured BEFORE the apply. Measuring afterwards read a
    // lock the apply had just rewritten, so every resource looked unchanged.
    let forced_noop_count = measure_forced_noops(&cfg, dry_run);

    let t_apply = Instant::now();
    let results = executor::apply(&cfg)?;
    let dur_apply = t_apply.elapsed();

    if dry_run {
        return apply_dry_run_output(&config, state_dir, machine_filter, tag_filter, json);
    }

    let (total_converged, total_unchanged, total_failed) = count_results(&results);

    save_apply_reports(state_dir, &results);

    if events_mode {
        return print_events_output(&results);
    }

    print_apply_summary(
        &config,
        &results,
        total_converged,
        total_unchanged,
        total_failed,
        forced_noop_count,
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
        return Err(apply_failure_path(
            &config,
            &results,
            state_dir,
            pre_apply_gen,
            rollback_on_failure,
            notify,
            (total_converged, total_failed, total_unchanged),
            &t_total,
            verbose,
        ));
    }

    // FJ-563: OTLP trace export (post-apply, non-blocking)
    export_otlp_traces(state_dir, telemetry_endpoint, &config.name, verbose);

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

/// GH-91: Warn that --sequential is not yet implemented.
fn warn_sequential_ignored(sequential: bool) {
    if sequential {
        eprintln!("Warning: --sequential is not yet implemented for apply. Flag ignored.");
    }
}

/// A run id is minted for every real apply and withheld for a dry run, which
/// writes no run directory.
fn apply_run_id(dry_run: bool) -> Option<String> {
    if dry_run {
        None
    } else {
        Some(crate::core::types::generate_run_id())
    }
}

/// The one-line "Applying <name> (N machines, M resources)" banner `--verbose`
/// prints before any work starts.
fn print_apply_banner(config: &types::ForjarConfig, verbose: bool) {
    if verbose {
        eprintln!(
            "Applying {} ({} machines, {} resources)",
            config.name,
            config.machines.len(),
            config.resources.len()
        );
    }
}

/// Everything `apply` does once it knows resources failed: roll the generation
/// back if asked, fire `--notify`, and return the error the caller propagates.
///
/// GH-210: `--notify` must fire on the failure path too. See
/// `apply_output::notify_on_failure`.
#[allow(clippy::too_many_arguments)]
fn apply_failure_path(
    config: &types::ForjarConfig,
    results: &[types::ApplyResult],
    state_dir: &Path,
    pre_apply_gen: Option<u32>,
    rollback_on_failure: bool,
    notify: Option<&str>,
    counts: (u32, u32, u32),
    t_total: &std::time::Instant,
    verbose: bool,
) -> String {
    let (total_converged, total_failed, total_unchanged) = counts;
    // FJ-1388: Generation-based rollback on failure
    maybe_rollback_generation(rollback_on_failure, state_dir, pre_apply_gen, verbose);
    super::apply_output::notify_on_failure(
        notify,
        config,
        results,
        (total_converged, total_failed, total_unchanged),
        t_total,
        verbose,
    );
    format!("{total_failed} resource(s) failed")
}

/// Parse the config and fold in every *input* that can still change it before
/// planning starts: the `--env-file` params, the injected workspace param and
/// the resolved data sources.
///
/// Exists so `cmd_apply_scoped` names one "read the desired state" step instead
/// of open-coding four, and so the optional `--env-file` branch is decided here
/// rather than in the command body. Order is load-bearing: env params land
/// before the workspace injection, and data sources resolve last so they can
/// see both.
fn load_apply_config(
    file: &Path,
    env_file: Option<&Path>,
    workspace: Option<&str>,
) -> Result<types::ForjarConfig, String> {
    let mut config = parse_and_validate(file)?;
    if let Some(path) = env_file {
        load_env_params(&mut config, path)?;
    }
    inject_workspace_param(&mut config, workspace);
    resolver::resolve_data_sources(&mut config)?;
    Ok(config)
}

/// GH-210 (FJ-129): how many already-converged resources `--force` will
/// re-apply.
///
/// Decides when that count is defined at all: only on a real forced run.
/// Without `--force` nothing is being forced, and a `--dry-run` re-applies
/// nothing, so both report zero rather than paying for the measurement.
/// Exists so that condition sits beside its own name instead of inline in the
/// command body. The *timing* constraint — this must be measured before the
/// apply rewrites the lock — belongs to the call site and is documented there.
fn measure_forced_noops(cfg: &executor::ApplyConfig, dry_run: bool) -> u32 {
    if cfg.force && !dry_run {
        executor::forced_noop_count(cfg)
    } else {
        0
    }
}

/// Persist one apply report per result.
///
/// Decides that a report that cannot be written is a *warning*, never a failed
/// apply: the resources already converged, so losing the audit record must not
/// turn a green run red. Exists to keep that policy in one named place instead
/// of an inline loop in the command body.
fn save_apply_reports(state_dir: &Path, results: &[types::ApplyResult]) {
    for result in results {
        if let Err(e) = state::save_apply_report(state_dir, result) {
            eprintln!("warning: cannot save apply report: {e}");
        }
    }
}

/// FJ-563: Export the run's spans to an OTLP endpoint after a successful apply.
///
/// Decides the three outcomes of an export: spans sent (announced only under
/// `--verbose`), export failed (a warning — telemetry must never fail the
/// apply), and nothing to send (silent). Exists so that non-blocking policy is
/// stated once, out of the command body.
fn export_otlp_traces(
    state_dir: &Path,
    telemetry_endpoint: Option<&str>,
    config_name: &str,
    verbose: bool,
) {
    let Some(endpoint) = telemetry_endpoint else {
        return;
    };
    match crate::tripwire::otlp_export::export_from_state_dir(state_dir, endpoint, config_name) {
        Ok(n) if n > 0 => {
            if verbose {
                eprintln!("OTLP: exported {n} spans to {endpoint}");
            }
        }
        Err(e) => eprintln!("warning: OTLP export failed: {e}"),
        _ => {}
    }
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
    config_path: Option<&Path>,
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
    match super::generation::create_generation(state_dir, config_path) {
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
    let to_remove = super::apply_gates::snapshots_to_remove(entries.len(), keep);
    if to_remove == 0 {
        return;
    }
    entries.sort_by_key(|e| e.file_name());
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

/// FJ-1380: Check convergence budget — warn/fail if apply exceeded time budget.
fn check_convergence_budget(
    config: &types::ForjarConfig,
    dur_apply: std::time::Duration,
) -> Result<(), String> {
    let elapsed = dur_apply.as_secs();
    if let Err(e) =
        super::apply_gates::check_convergence_budget_pure(config.policy.convergence_budget, elapsed)
    {
        eprintln!(
            "ERROR: convergence budget exceeded — budget {}s, actual {elapsed}s",
            config.policy.convergence_budget.unwrap_or(0)
        );
        return Err(e);
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
        // FJ-1378-fix: Pass the machine object so container transports use
        // docker exec instead of checking the host filesystem.
        let findings = match config.machines.get(machine_name.as_str()) {
            Some(m) => crate::tripwire::drift::detect_drift_with_machine(lock, m),
            None => crate::tripwire::drift::detect_drift(lock),
        };
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
    if let Some(msg) =
        super::apply_gates::should_block_on_drift(config.policy.tripwire, force, total_drift)
    {
        if verbose {
            eprintln!("{total_drift} resource(s) drifted — run 'forjar drift' for details");
        }
        return Err(msg);
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
    resource_filter: Option<&str>,
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
        // GH-253: scoped, so `-r` on a non-destructive resource is not blocked
        // by destroys the operator did not select and apply would not perform.
        let (_, _, destroy_count) =
            super::apply_gates::scoped_action_counts(&plan.changes, resource_filter);
        if let Some(msg) = super::apply_gates::should_block_destructive(
            destroy_count,
            confirm_destructive,
            dry_run,
            yes,
        ) {
            eprintln!(
                "WARNING: {destroy_count} resource(s) will be DESTROYED. Use --yes to confirm."
            );
            return Err(msg);
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
        let (to_create, to_update, to_destroy) =
            super::apply_gates::scoped_action_counts(&preview_plan.changes, resource_filter);
        let n_changes = to_create + to_update + to_destroy;
        if n_changes > 0 {
            eprint!(
                "Apply {n_changes} change(s) ({to_create} create, {to_update} update, {to_destroy} destroy)? [y/N] "
            );
            let mut answer = String::new();
            std::io::stdin()
                .read_line(&mut answer)
                .map_err(|e| format!("stdin error: {e}"))?;
            if !answer.trim().eq_ignore_ascii_case("y") {
                return Err("aborted by user".to_string());
            }
        }
    }

    Ok(())
}

/// FJ-220 + FJ-3200: Check policy rules and block apply if any error-severity violations exist.
fn check_policy_violations(config: &types::ForjarConfig) -> Result<(), String> {
    if config.policies.is_empty() {
        return Ok(());
    }
    let result = parser::evaluate_policies_full(config);
    if !result.has_blocking_violations() {
        // Print warnings if any
        for v in &result.violations {
            eprintln!("  [WARN] {}: {}", v.resource_id, v.rule_message);
        }
        return Ok(());
    }
    for v in &result.violations {
        let sev = if v.is_blocking() { "DENY" } else { "WARN" };
        eprintln!("  [{sev}] {}: {}", v.resource_id, v.rule_message);
    }
    Err(format!(
        "policy violations block apply ({} error(s))",
        result.error_count()
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
    let should_fail = super::apply_gates::security_gate_should_block(
        &threshold,
        crit,
        high,
        med,
        findings.len(),
    )?;
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
