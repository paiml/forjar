//! Apply command output formatting helpers.

use super::apply_drift::GateScope;
use super::apply_helpers::*;
use super::helpers::*;
use crate::core::{state, types};
use std::path::Path;

pub(super) fn count_results(results: &[types::ApplyResult]) -> (u32, u32, u32) {
    let mut converged = 0u32;
    let mut unchanged = 0u32;
    let mut failed = 0u32;
    for result in results {
        converged += result.resources_converged;
        unchanged += result.resources_unchanged;
        failed += result.resources_failed;
    }
    (converged, unchanged, failed)
}

/// Handle dry-run output.
///
/// PMAT-160: ONE plan, scoped by every selector the apply honours (`-m`, `-t`,
/// `-r`, `-g`), feeds both the text body and `--json` — see `apply_dry_run`.
/// The JSON branch used to plan on its own and skipped even the machine
/// filter the text branch applied, so the two bodies could disagree.
pub(super) fn apply_dry_run_output(
    config: &types::ForjarConfig,
    state_dir: &Path,
    scope: &GateScope<'_>,
    json: bool,
) -> Result<(), String> {
    let plan = super::apply_dry_run::scoped_dry_run_plan(config, state_dir, scope)?;
    if json {
        let output = super::apply_dry_run::render_dry_run_json(&plan);
        println!(
            "{}",
            serde_json::to_string_pretty(&output).map_err(|e| format!("JSON error: {e}"))?
        );
    } else {
        print!("{}", super::apply_dry_run::render_dry_run_actions(&plan));
    }
    Ok(())
}

/// Print events-mode output.
pub(super) fn print_events_output(results: &[types::ApplyResult]) -> Result<(), String> {
    for result in results {
        for r in &result.resource_reports {
            let event = serde_json::json!({
                "event": if r.status == "converged" { "resource_converged" }
                         else if r.status == "failed" { "resource_failed" }
                         else { "resource_unchanged" },
                "machine": result.machine,
                "resource": r.resource_id,
                "type": r.resource_type,
                "status": r.status,
                "duration_seconds": r.duration_seconds,
                "hash": r.hash,
                "error": r.error,
            });
            println!("{}", serde_json::to_string(&event).unwrap_or_default());
        }
        let complete = serde_json::json!({
            "event": "apply_complete",
            "machine": result.machine,
            "converged": result.resources_converged,
            "unchanged": result.resources_unchanged,
            "failed": result.resources_failed,
            "duration_seconds": result.total_duration.as_secs_f64(),
        });
        println!("{}", serde_json::to_string(&complete).unwrap_or_default());
    }
    Ok(())
}

/// Print per-resource report table.
pub(super) fn print_resource_report(results: &[types::ApplyResult]) {
    println!();
    println!("{}", bold("Resource Report"));
    println!(
        "{:<30} {:<10} {:<12} {:>10}",
        bold("RESOURCE"),
        bold("TYPE"),
        bold("STATUS"),
        bold("DURATION")
    );
    println!("{}", dim(&"-".repeat(66)));
    for result in results {
        for r in &result.resource_reports {
            let status_colored = match r.status.as_str() {
                "converged" => green(&r.status),
                "failed" => red(&r.status),
                _ => r.status.clone(),
            };
            println!(
                "{:<30} {:<10} {:<12} {:>9.3}s",
                r.resource_id, r.resource_type, status_colored, r.duration_seconds
            );
        }
    }
}

/// Print timing breakdown.
pub(super) fn print_timing(
    dur_parse: std::time::Duration,
    dur_apply: std::time::Duration,
    dur_total: std::time::Duration,
) {
    println!();
    println!("{}", bold("Timing Breakdown"));
    println!("{}", dim(&"-".repeat(40)));
    println!(
        "  {:<20} {:>10.3}s",
        "Parse + resolve",
        dur_parse.as_secs_f64()
    );
    println!("  {:<20} {:>10.3}s", "Apply", dur_apply.as_secs_f64());
    println!("{}", dim(&"-".repeat(40)));
    println!("  {:<20} {:>10.3}s", bold("Total"), dur_total.as_secs_f64());
}

/// Post-apply actions: state update, auto-commit, hooks, notifications.
#[allow(clippy::too_many_arguments)]
pub(super) fn apply_post_actions(
    state_dir: &Path,
    config: &types::ForjarConfig,
    results: &[types::ApplyResult],
    total_converged: u32,
    auto_commit: bool,
    verbose: bool,
    notify: Option<&str>,
    t_total: &std::time::Instant,
) -> Result<(), String> {
    let total_failed: u32 = results.iter().map(|r| r.resources_failed).sum();
    let total_unchanged: u32 = results.iter().map(|r| r.resources_unchanged).sum();

    let machine_results: Vec<_> = results
        .iter()
        .map(|r| {
            (
                r.machine.clone(),
                (r.resources_converged + r.resources_unchanged + r.resources_failed) as usize,
                r.resources_converged as usize,
                r.resources_failed as usize,
            )
        })
        .collect();
    state::update_global_lock(state_dir, &config.name, &machine_results)?;

    // FJ-1260: Persist resolved outputs for cross-stack data flow
    if !config.outputs.is_empty() {
        let resolved = state::resolve_outputs(config);
        state::persist_outputs(state_dir, &config.name, &resolved, config.secrets.ephemeral)?;
    }

    // FJ-1200: Run post-apply check blocks
    if !config.checks.is_empty() && total_failed == 0 {
        run_check_blocks(config, verbose);
    }

    if auto_commit && total_converged > 0 {
        git_commit_state(state_dir, &config.name, total_converged)?;
    }

    if let Some(ref hook) = config.policy.post_apply {
        if let Err(e) = run_hook("post_apply", hook, verbose) {
            eprintln!("Warning: {e}");
        }
    }

    // FJ-225: Notification hooks
    run_notify_hooks(config, results);

    // FJ-317: Webhook notification
    if let Some(url) = notify {
        send_apply_webhook(
            url,
            config,
            results,
            total_converged,
            total_failed,
            total_unchanged,
            t_total,
            verbose,
        );
    }

    Ok(())
}

/// FJ-225: Run the per-machine notification hook for each apply result.
///
/// A machine with failures takes `notify.on_failure`, everything else takes
/// `notify.on_success`; an unconfigured hook is a no-op for that machine.
fn run_notify_hooks(config: &types::ForjarConfig, results: &[types::ApplyResult]) {
    for result in results {
        let converged_str = result.resources_converged.to_string();
        let unchanged_str = result.resources_unchanged.to_string();
        let failed_str = result.resources_failed.to_string();
        let vars: Vec<(&str, &str)> = vec![
            ("machine", &result.machine),
            ("converged", &converged_str),
            ("unchanged", &unchanged_str),
            ("failed", &failed_str),
        ];
        if result.resources_failed > 0 {
            if let Some(ref cmd) = config.policy.notify.on_failure {
                run_notify(cmd, &vars);
            }
        } else if let Some(ref cmd) = config.policy.notify.on_success {
            run_notify(cmd, &vars);
        }
    }
}

/// FJ-1200: Run post-apply check blocks.
///
/// Check blocks execute AFTER all resources converge. Each check runs a command
/// on the specified machine and verifies the exit code matches expect_exit (default 0).
/// Failures are warnings — they don't roll back the apply (like OpenTofu).
pub(super) fn run_check_blocks(config: &types::ForjarConfig, verbose: bool) {
    let total = config.checks.len();
    let mut passed = 0usize;
    let mut failed = 0usize;

    if verbose {
        println!();
        println!("{}", bold("Post-apply checks"));
        println!("{}", dim(&"-".repeat(50)));
    }

    for (name, check) in &config.checks {
        let expected_exit = check.expect_exit.unwrap_or(0);
        let machine = match config.machines.get(&check.machine) {
            Some(m) => m,
            None => {
                eprintln!(
                    "warning: check '{}' references unknown machine '{}'",
                    name, check.machine
                );
                failed += 1;
                continue;
            }
        };

        match crate::transport::exec_script(machine, &check.command) {
            Ok(out) => {
                let actual_exit = out.exit_code;
                if actual_exit == expected_exit {
                    passed += 1;
                    if verbose {
                        let desc = check.description.as_deref().unwrap_or(&check.command);
                        println!("  {} {} — {}", green("PASS"), name, desc);
                    }
                } else {
                    failed += 1;
                    let desc = check.description.as_deref().unwrap_or(&check.command);
                    eprintln!(
                        "  {} {} — {} (exit {}, expected {})",
                        red("FAIL"),
                        name,
                        desc,
                        actual_exit,
                        expected_exit
                    );
                }
            }
            Err(e) => {
                failed += 1;
                eprintln!("  {} {} — transport error: {}", red("FAIL"), name, e);
            }
        }
    }

    if failed > 0 {
        eprintln!("warning: {failed}/{total} post-apply checks failed");
    } else if verbose {
        println!("  All {passed}/{total} checks passed.");
    }
}

/// Send webhook notification for apply results.
#[allow(clippy::too_many_arguments)]
/// GH-210: deliver the `--notify` (FJ-317) result webhook for a FAILED apply.
///
/// `--notify` was dispatched only from `apply_post_actions`, which the failure
/// path returns before ever reaching — so a monitoring integration built on it
/// could never alert on the one case it exists for. Measured on 1.12.3 with all
/// three flags on one command line against one receiver:
///
/// ```text
///   failing apply:  HIT /SLACK   HIT /WEBHOOK   (no /NOTIFY, no warning)
///   passing apply:  HIT /NOTIFY  HIT /WEBHOOK
/// ```
///
/// The payload's own `total_failed` field was therefore unreachable. Counts are
/// passed as a tuple so this stays one call at the return site.
pub(super) fn notify_on_failure(
    notify: Option<&str>,
    config: &types::ForjarConfig,
    results: &[types::ApplyResult],
    counts: (u32, u32, u32),
    t_total: &std::time::Instant,
    verbose: bool,
) {
    let Some(url) = notify else { return };
    let (converged, failed, unchanged) = counts;
    send_apply_webhook(
        url, config, results, converged, failed, unchanged, t_total, verbose,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn send_apply_webhook(
    url: &str,
    config: &types::ForjarConfig,
    results: &[types::ApplyResult],
    total_converged: u32,
    total_failed: u32,
    total_unchanged: u32,
    t_total: &std::time::Instant,
    verbose: bool,
) {
    let payload = serde_json::json!({
        "name": config.name,
        "total_converged": total_converged,
        "total_failed": total_failed,
        "total_unchanged": total_unchanged,
        "duration_seconds": t_total.elapsed().as_secs_f64(),
        "results": results.iter().map(|r| serde_json::json!({
            "machine": r.machine,
            "converged": r.resources_converged,
            "failed": r.resources_failed,
            "unchanged": r.resources_unchanged,
            "duration_seconds": r.total_duration.as_secs_f64(),
        })).collect::<Vec<_>>(),
    });
    let payload_str = serde_json::to_string(&payload).unwrap_or_default();
    // GH-210: bounded in time and judged on the HTTP status, not on curl's
    // process exit. A receiver replying 500 rejected the notification and used
    // to produce no warning at all.
    match super::webhook_post::post_json(url, &payload_str, &[]) {
        Ok(()) => {
            if verbose {
                eprintln!("Webhook notification sent to {url}");
            }
        }
        Err(e) => eprintln!("Warning: webhook POST to {url} failed ({e})"),
    }
}
