//! Apply command.

use super::apply_helpers::*;
use super::apply_output::*;
use super::apply_scope::{apply_scope, ApplyScope};
use super::apply_selection::*;
use super::apply_summary::print_apply_summary;
use super::helpers::*;
use super::helpers_state::*;
use super::workspace::*;
use crate::core::{executor, resolver, state, types};
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

    // forjar#404 (CRUX audit E02): OPEN THE SOCKETS BEFORE THE GATES.
    //
    // The pre-apply drift gate two lines below queries every locked resource of
    // every target machine through the transport. `apply_machine` did not start
    // the ControlMaster until twenty frames later, so `build_ssh_args` found no
    // socket and every one of those queries was a full handshake — 306 ms
    // median measured against 6.7 ms multiplexed, 45×, repeated once per locked
    // resource. Held to the end of the function: dropping it early closes the
    // sockets the gate and the executor are about to use.
    let gate_scope = super::apply_drift::GateScope {
        machine: machine_filter,
        resource: resource_filter,
        tag: tag_filter,
        group: group_filter,
    };
    let _ssh_mux =
        super::apply_mux::open_control_masters(&config, &gate_scope, force, dry_run, verbose);

    let observed_drift = super::apply_preflight::apply_pre_validate(
        &config,
        state_dir,
        machine_filter,
        tag_filter,
        resource_filter,
        group_filter,
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

    super::apply_snapshot::maybe_auto_snapshot(&config, state_dir, dry_run, verbose);

    // FJ-1388: Record pre-apply generation for rollback-on-failure. GH-376 made
    // this literal: it is now the generation of the last SUCCESSFUL apply, not
    // one this apply just created holding the same pre-apply state.
    let pre_apply_gen = pre_apply_generation(state_dir);

    // GH-210 (FJ-129): measured BEFORE the apply. Measuring afterwards read a
    // lock the apply had just rewritten, so every resource looked unchanged.
    // Refs #378: the measurement is now a candidate SET, reconciled with the
    // run below — the timing constraint is unchanged.
    let forced_noop_candidates = measure_forced_noops(&cfg, dry_run);

    let t_apply = Instant::now();
    let results = executor::apply(&cfg)?;
    let dur_apply = t_apply.elapsed();

    if dry_run {
        return apply_dry_run_output(&config, state_dir, machine_filter, tag_filter, json);
    }

    let (total_converged, total_unchanged, total_failed) = count_results(&results);
    // Refs #378: a candidate the run skipped or failed is not a forced no-op.
    let forced_noop_count = forced_noops_that_ran(&forced_noop_candidates, &results);
    // GH-336: intersect what drifted BEFORE the run with what the run actually
    // converged. Raw findings would claim repairs for resources the filters
    // excluded or that failed.
    let drift_repaired = super::apply_drift::repaired(&observed_drift, &results);

    save_apply_reports(state_dir, &results);

    // GH-376: record the generation AFTER the host converged, so it pairs the
    // state this apply produced with the config that produced it — the pair
    // `undo` replays. Placed above the `--output events` return so every path
    // records one.
    //
    // UNCONDITIONAL, and that is load-bearing. This was briefly gated on
    // `total_failed == 0`, reasoning that a half-applied host should not become
    // an undo target. Measured against the pre-change parent, that gate meant a
    // stack with ONE persistently failing resource recorded ZERO generations
    // for ever: `generation list` empty, `rollback --generation` dead, `undo`
    // refusing with advice that could not succeed, and `--rollback-on-failure`
    // silently never firing — including on every FIRST apply, with no message.
    // Strictly worse than the defect it was part of fixing.
    //
    // A generation is a RECORD OF WHAT HAPPENED, not a certificate that it went
    // well. The lock it carries describes the partial state faithfully, which
    // is exactly what `--rollback-on-failure` needs to rewind to. Refusing to
    // record is how you lose the ability to recover from the failure.
    super::apply_snapshot::maybe_record_generation(&config, state_dir, dry_run, verbose);

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
        &drift_repaired,
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

/// GH-210 (FJ-129): which already-converged resources `--force` will re-apply.
///
/// Decides when that set is defined at all: only on a real forced run.
/// Without `--force` nothing is being forced, and a `--dry-run` re-applies
/// nothing, so both report nothing rather than paying for the measurement.
/// Exists so that condition sits beside its own name instead of inline in the
/// command body. The *timing* constraint — this must be measured before the
/// apply rewrites the lock — belongs to the call site and is documented there.
fn measure_forced_noops(cfg: &executor::ApplyConfig, dry_run: bool) -> Vec<(String, String)> {
    if cfg.force && !dry_run {
        executor::forced_noop_candidates(cfg)
    } else {
        Vec::new()
    }
}

/// Refs #378: RECONCILE THE PRE-APPLY MEASUREMENT WITH THE RUN.
///
/// A candidate counts only if the run actually re-ran it. Nothing did this, so
/// a resource the shadow plan called a NoOp and the executor then SKIPPED
/// (`-r`, `-g`) or FAILED was still reported as a forced no-op — producing
/// `forced_noop_count > total_converged`, which the summary's own
/// `debug_assert!` aborts a debug build over, and which a release build prints
/// as a `--json` summary that contradicts its own contract plus a note line
/// naming more resources than the run touched.
///
/// The oracle is the one `apply_drift::repaired` already uses for the drift
/// dimension of the very same summary line: `build_resource_reports` derives
/// `status` from the POST-apply lock, so "the lock says converged afterwards"
/// is the sound test for "the run re-ran it". Refs #390-C narrowed
/// `resource_reports` to `converged ∪ failed`, which is what makes the
/// intersection tight — a binary predating that narrowing reports every
/// resource on the machine and would make this look like a no-op change.
fn forced_noops_that_ran(candidates: &[(String, String)], results: &[types::ApplyResult]) -> u32 {
    candidates
        .iter()
        .filter(|(machine, resource_id)| {
            results.iter().any(|r| {
                r.machine == *machine
                    && r.resource_reports
                        .iter()
                        .any(|rr| rr.resource_id == *resource_id && rr.status == "converged")
            })
        })
        .count() as u32
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
