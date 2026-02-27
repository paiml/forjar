//! Apply command output formatting helpers.

use crate::core::{planner, resolver, state, types};
use std::path::Path;
use super::helpers::*;
use super::helpers_state::*;
use super::apply_helpers::*;
use super::print_helpers::*;

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
pub(super) fn apply_dry_run_output(
    config: &types::ForjarConfig,
    state_dir: &Path,
    machine_filter: Option<&str>,
    tag_filter: Option<&str>,
    json: bool,
) -> Result<(), String> {
    if json {
        let execution_order = resolver::build_execution_order(config)?;
        let plan_locks = load_machine_locks(config, state_dir, machine_filter)?;
        let plan = planner::plan(config, &execution_order, &plan_locks, tag_filter);
        let changes: Vec<serde_json::Value> = plan.changes.iter()
            .map(|c| serde_json::json!({
                "resource": c.resource_id,
                "machine": c.machine,
                "type": c.resource_type.to_string(),
                "action": format!("{:?}", c.action).to_lowercase(),
                "description": c.description,
            }))
            .collect();
        let output = serde_json::json!({
            "dry_run": true,
            "name": plan.name,
            "to_create": plan.to_create,
            "to_update": plan.to_update,
            "to_destroy": plan.to_destroy,
            "unchanged": plan.unchanged,
            "changes": changes,
        });
        println!("{}", serde_json::to_string_pretty(&output).map_err(|e| format!("JSON error: {}", e))?);
    } else {
        println!("Dry run — no changes applied.");
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


/// Print apply summary (JSON or text).
#[allow(clippy::too_many_arguments)]
pub(super) fn print_apply_summary(
    config: &types::ForjarConfig,
    results: &[types::ApplyResult],
    total_converged: u32,
    total_unchanged: u32,
    total_failed: u32,
    dur_apply: std::time::Duration,
    json: bool,
) -> Result<(), String> {
    if json {
        let output = serde_json::json!({
            "name": config.name,
            "machines": results,
            "summary": {
                "total_converged": total_converged,
                "total_unchanged": total_unchanged,
                "total_failed": total_failed,
                "total_duration_seconds": dur_apply.as_secs_f64(),
            }
        });
        println!("{}", serde_json::to_string_pretty(&output).map_err(|e| format!("JSON serialization error: {}", e))?);
    } else {
        for result in results {
            let failed_str = if result.resources_failed > 0 {
                red(&format!("{} failed", result.resources_failed))
            } else {
                format!("{} failed", result.resources_failed)
            };
            println!(
                "{}: {} converged, {} unchanged, {} ({:.1}s)",
                bold(&result.machine),
                green(&result.resources_converged.to_string()),
                result.resources_unchanged, failed_str,
                result.total_duration.as_secs_f64()
            );
        }
        println!();
        if total_failed > 0 {
            println!("{}", red(&format!(
                "Apply completed with errors: {} converged, {} unchanged, {} FAILED",
                total_converged, total_unchanged, total_failed
            )));
        } else {
            println!("{}", green(&format!(
                "Apply complete: {} converged, {} unchanged.", total_converged, total_unchanged
            )));
        }
    }
    Ok(())
}


/// Print per-resource report table.
pub(super) fn print_resource_report(results: &[types::ApplyResult]) {
    println!();
    println!("{}", bold("Resource Report"));
    println!("{:<30} {:<10} {:<12} {:>10}", bold("RESOURCE"), bold("TYPE"), bold("STATUS"), bold("DURATION"));
    println!("{}", dim(&"-".repeat(66)));
    for result in results {
        for r in &result.resource_reports {
            let status_colored = match r.status.as_str() {
                "converged" => green(&r.status),
                "failed" => red(&r.status),
                _ => r.status.clone(),
            };
            println!("{:<30} {:<10} {:<12} {:>9.3}s", r.resource_id, r.resource_type, status_colored, r.duration_seconds);
        }
    }
}


/// Print timing breakdown.
pub(super) fn print_timing(dur_parse: std::time::Duration, dur_apply: std::time::Duration, dur_total: std::time::Duration) {
    println!();
    println!("{}", bold("Timing Breakdown"));
    println!("{}", dim(&"-".repeat(40)));
    println!("  {:<20} {:>10.3}s", "Parse + resolve", dur_parse.as_secs_f64());
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

    let machine_results: Vec<_> = results.iter()
        .map(|r| (
            r.machine.clone(),
            (r.resources_converged + r.resources_unchanged + r.resources_failed) as usize,
            r.resources_converged as usize,
            r.resources_failed as usize,
        ))
        .collect();
    state::update_global_lock(state_dir, &config.name, &machine_results)?;

    if auto_commit && total_converged > 0 {
        git_commit_state(state_dir, &config.name, total_converged)?;
    }

    if let Some(ref hook) = config.policy.post_apply {
        if let Err(e) = run_hook("post_apply", hook, verbose) {
            eprintln!("Warning: {}", e);
        }
    }

    // FJ-225: Notification hooks
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

    // FJ-317: Webhook notification
    if let Some(url) = notify {
        send_apply_webhook(url, config, results, total_converged, total_failed, total_unchanged, t_total, verbose);
    }

    Ok(())
}


/// Send webhook notification for apply results.
#[allow(clippy::too_many_arguments)]
fn send_apply_webhook(
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
    let result = std::process::Command::new("curl")
        .args(["-s", "-X", "POST", "-H", "Content-Type: application/json", "-d", &payload_str, url])
        .output();
    match result {
        Ok(output) if output.status.success() => {
            if verbose {
                eprintln!("Webhook notification sent to {}", url);
            }
        }
        Ok(output) => eprintln!("Warning: webhook POST to {} failed (exit {})", url, output.status),
        Err(e) => eprintln!("Warning: webhook POST failed: {}", e),
    }
}
