//! forjar#1000: `forjar check` for a `github_runner`.
//!
//! The question is not answered by a script on the seat but by asking GitHub
//! whether a runner of that name is listed, with what labels and group. That
//! needs the App key, which only the control host has, so the resource is
//! checked here and never gets a check script.

use super::*;
use crate::resources::github_runner::{self, Verdict};

/// What `check` concluded for one runner: status, detail, and whether it passed.
fn judge(config: &types::ForjarConfig, resource: &types::Resource) -> (&'static str, String, bool) {
    let resolved = match resolver::resolve_resource_templates_with_secrets(
        resource,
        &config.params,
        &config.machines,
        &config.secrets,
    ) {
        Ok(r) => r,
        Err(e) => return ("error", e, false),
    };
    match github_runner::check(&resolved) {
        Ok(Verdict::Equal) => ("pass", String::new(), true),
        Ok(Verdict::Absent) => (
            "pending",
            "pending: not registered yet; apply would register it".to_string(),
            false,
        ),
        Ok(Verdict::Drift(d)) => ("fail", format!("drift: {d}"), false),
        Err(e) => ("error", e, false),
    }
}

/// Check one `github_runner` on every machine it targets, folding the outcome
/// into `tally`.
pub(super) fn check_runner(
    ctx: &CheckCtx<'_>,
    config: &types::ForjarConfig,
    resource_id: &str,
    resource: &types::Resource,
    tally: &mut CheckTally,
) {
    let mut verdict: Option<(&'static str, String, bool)> = None;
    for machine_name in resource.machine.iter() {
        let machine = ctx.machines.get(machine_name).unwrap_or(&ctx.localhost);
        if skip_machine(machine_name, ctx.machine_filter, resource, machine) {
            tally.skip += 1;
            continue;
        }
        let (status, detail, passed) = verdict.get_or_insert_with(|| judge(config, resource));
        if !ctx.json {
            if *passed {
                println!("  ok {resource_id} ({machine_name})");
            } else {
                println!("  FAIL {resource_id} ({machine_name}) — {detail}");
            }
        }
        let mut result = make_check_result(resource_id, machine_name, status, None, detail.clone());
        result.in_state = resource_in_state(&ctx.state, machine_name, resource_id);
        report_unrecorded(result.in_state, resource_id, machine_name, ctx.json);
        if *passed {
            tally.pass += 1;
        } else {
            tally.fail += 1;
        }
        tally.results.push(result);
    }
}

/// Check one resource that passed the filters: a `github_runner` is asked of
/// GitHub, anything else is probed on its machines with its check script.
pub(super) fn check_one(
    ctx: &CheckCtx<'_>,
    config: &types::ForjarConfig,
    resource_id: &str,
    resource: &types::Resource,
    tally: &mut CheckTally,
) -> Result<(), String> {
    if resource.resource_type == types::ResourceType::GithubRunner {
        check_runner(ctx, config, resource_id, resource, tally);
        return Ok(());
    }
    let resolved =
        resolver::resolve_resource_templates(resource, &config.params, &config.machines)?;
    match check_script_or_skip(resource_id, resource, &resolved, ctx.json) {
        Some(script) => check_resource_machines(ctx, resource, resource_id, &script, tally),
        None => tally.skip += 1,
    }
    Ok(())
}
