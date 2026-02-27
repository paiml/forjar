//! Apply command.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::apply_helpers::*;
use super::workspace::*;
use super::apply_output::*;


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
            config.name, config.machines.len(), config.resources.len()
        );
    }
    if no_tripwire {
        config.policy.tripwire = false;
    }
    apply_param_overrides(&mut config, param_overrides)?;

    apply_filters(&mut config, subset, exclude, verbose)?;
    apply_pre_validate(
        &config, state_dir, machine_filter, tag_filter,
        confirm_destructive, dry_run, yes, verbose,
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

    print_apply_summary(&config, &results, total_converged, total_unchanged, total_failed, dur_apply, json)?;

    if report {
        print_resource_report(&results);
    }
    if timing {
        print_timing(dur_parse, dur_apply, t_total.elapsed());
    }
    if total_failed > 0 {
        return Err(format!("{} resource(s) failed", total_failed));
    }

    apply_post_actions(
        state_dir, &config, &results,
        total_converged, auto_commit, verbose, notify, &t_total,
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
        config.resources.retain(|id, _| simple_glob_match(pattern, id));
        if config.resources.is_empty() {
            return Err(format!("no resources match subset pattern '{}'", pattern));
        }
        if verbose {
            eprintln!("Subset filter '{}': {} resources selected", pattern, config.resources.len());
        }
    }
    if let Some(pattern) = exclude {
        let before = config.resources.len();
        config.resources.retain(|id, _| !simple_glob_match(pattern, id));
        if verbose {
            eprintln!(
                "Exclude filter '{}': removed {} resources ({} remaining)",
                pattern, before - config.resources.len(), config.resources.len()
            );
        }
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
    yes: bool,
    verbose: bool,
) -> Result<(), String> {
    // FJ-335: Confirm destructive actions
    if confirm_destructive && !dry_run && !yes {
        let order = resolver::build_execution_order(config)?;
        let cd_locks = load_machine_locks(config, state_dir, machine_filter)?;
        let plan = planner::plan(config, &order, &cd_locks, tag_filter);
        let destroy_count = plan.changes.iter()
            .filter(|p| p.action == types::PlanAction::Destroy)
            .count();
        if destroy_count > 0 {
            eprintln!("WARNING: {} resource(s) will be DESTROYED. Use --yes to confirm.", destroy_count);
            return Err(format!("{} destructive action(s) blocked by --confirm-destructive", destroy_count));
        }
    }

    // FJ-220: Evaluate policy rules before apply
    if !config.policies.is_empty() {
        let violations = parser::evaluate_policies(config);
        let has_deny = violations.iter().any(|v| {
            matches!(v.severity, types::PolicyRuleType::Deny | types::PolicyRuleType::Require)
        });
        if has_deny {
            for v in &violations {
                let sev = match v.severity {
                    types::PolicyRuleType::Deny | types::PolicyRuleType::Require => "DENY",
                    types::PolicyRuleType::Warn => "WARN",
                };
                eprintln!("  [{}] {}: {}", sev, v.resource_id, v.rule_message);
            }
            return Err(format!(
                "policy violations block apply ({} denied)",
                violations.iter()
                    .filter(|v| matches!(v.severity, types::PolicyRuleType::Deny | types::PolicyRuleType::Require))
                    .count()
            ));
        }
    }

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
            std::io::stdin().read_line(&mut answer).map_err(|e| format!("stdin error: {}", e))?;
            if !answer.trim().eq_ignore_ascii_case("y") {
                return Err("aborted by user".to_string());
            }
        }
    }

    Ok(())
}
