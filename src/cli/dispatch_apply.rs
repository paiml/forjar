//! Apply command dispatch — pre-checks, execution, and post-processing.

#[allow(unused_imports)]
use crate::core::types::ProvenanceEvent;
#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
#[allow(unused_imports)]
use crate::transport;
#[allow(unused_imports)]
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::commands::*;
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::apply::*;
use super::apply_variants::*;
use super::plan::*;
use super::check::*;
use super::drift::*;
use super::snapshot::*;
use super::workspace::*;
use super::dispatch_notify::{NotifyOpts, send_apply_notifications};


/// Run a shell script and return error if it fails.
fn run_script_check(script: &str) -> Result<(), String> {
    let output = std::process::Command::new("bash")
        .arg("-c")
        .arg(script)
        .output();
    match output {
        Ok(o) if !o.status.success() => {
            Err(String::from_utf8_lossy(&o.stderr).trim().to_string())
        }
        Err(e) => Err(e.to_string()),
        _ => Ok(()),
    }
}

/// Run a pre-script (file path, not inline).
fn run_pre_script(script: &Path) -> Result<(), String> {
    let status = std::process::Command::new("bash")
        .arg(script)
        .status()
        .map_err(|e| format!("Failed to run pre-script: {}", e))?;
    if !status.success() {
        return Err(format!(
            "Pre-script {} exited with code {}",
            script.display(), status.code().unwrap_or(-1)
        ));
    }
    Ok(())
}

/// Send a webhook notification (fire-and-forget).
fn send_webhook_before(url: &str, file: &Path) {
    let payload = format!(r#"{{"event":"apply_start","config":"{}"}}"#, file.display());
    let _ = std::process::Command::new("curl")
        .args(["-s", "-X", "POST", "-H", "Content-Type: application/json", "-d", &payload, url])
        .output();
}

/// Run post-flight script (warn on failure, don't abort).
fn run_post_flight(script: &str) {
    if let Ok(o) = std::process::Command::new("bash").arg("-c").arg(script).output() {
        if !o.status.success() {
            eprintln!("Post-flight warning: {}", String::from_utf8_lossy(&o.stderr).trim());
        }
    }
}

/// Check cost limit against planned changes.
fn check_cost_limit(file: &Path, sd: &Path, machine: Option<&str>, tag: Option<&str>, limit: usize) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let order = resolver::build_execution_order(&config)?;
    let locks = load_machine_locks(&config, sd, machine)?;
    let plan = planner::plan(&config, &order, &locks, tag);
    let change_count = plan.changes.iter().filter(|c| c.action != types::PlanAction::NoOp).count();
    if change_count > limit {
        return Err(format!(
            "Cost limit exceeded: {} changes planned, limit is {}. Use --cost-limit {} or higher to proceed.",
            change_count, limit, change_count
        ));
    }
    Ok(())
}

/// Dispatch the Apply command variant.
#[allow(clippy::too_many_arguments)]
pub(crate) fn dispatch_apply_cmd(cmd: Commands, verbose: bool) -> Result<(), String> {
    let Commands::Apply(ApplyArgs {
        file, machine, resource, tag, group, force, dry_run, no_tripwire,
        params, auto_commit, timeout, state_dir, json, env_file, workspace,
        check, report, force_unlock, output, progress, timing, retry, yes,
        parallel, resource_timeout, rollback_on_failure, max_parallel, notify,
        subset, confirm_destructive, backup, exclude, sequential, diff_only,
        notify_slack, cost_limit, preview,
        tag_filter: _tag_filter, output_scripts,
        resume: _resume, confirm: _confirm, max_failures: _max_failures,
        rate_limit: _rate_limit, labels: _labels, plan_file: _plan_file,
        notify_email, skip: _skip, snapshot_before,
        concurrency: _concurrency, webhook_before,
        rollback_snapshot: _rollback_snapshot, retry_delay: _retry_delay,
        tags: _tags, log_file: _log_file, comment: _comment,
        only_changed: _only_changed, pre_script,
        dry_run_json: _dry_run_json, notify_webhook, post_script,
        approval_required: _approval_required, canary_percent: _canary_percent,
        schedule: _schedule, env_name: _env_name, dry_run_diff: _dry_run_diff,
        notify_pagerduty, batch_size: _batch_size,
        notify_teams, abort_on_drift, dry_run_summary: _dry_run_summary,
        notify_discord, rollback_on_threshold: _rollback_on_threshold,
        metrics_port: _metrics_port, notify_opsgenie,
        circuit_breaker: _circuit_breaker, require_approval: _require_approval,
        notify_datadog, change_window: _change_window, canary_machine,
        notify_newrelic, max_duration: _max_duration, notify_grafana,
        rate_limit_resources: _rate_limit_resources,
        checkpoint_interval: _checkpoint_interval, notify_victorops,
        blue_green: _blue_green, dry_run_cost, notify_msteams_adaptive,
        progressive: _progressive, approval_webhook: _approval_webhook,
        notify_incident, sign_off: _sign_off, notify_sns,
        telemetry_endpoint: _telemetry_endpoint, runbook: _runbook,
        notify_pubsub, fleet_strategy: _fleet_strategy,
        pre_check: _pre_check, notify_eventbridge, dry_run_graph,
        post_check: _post_check, notify_kafka,
        max_retries: _max_retries, rollback_window: _rollback_window,
        notify_azure_servicebus, approval_timeout: _approval_timeout,
        pre_flight, notify_gcp_pubsub_v2, checkpoint: _checkpoint,
        post_flight, notify_rabbitmq, gate: _gate, notify_nats,
        dry_run_verbose, explain: _explain, notify_mqtt,
        confirmation_message, summary_only: _summary_only,
        notify_redis, notify_amqp, pre_apply_hook: _pre_apply_hook,
        resource_filter: _resource_filter, notify_stomp,
        post_apply_hook: _post_apply_hook, dry_run_shell: _dry_run_shell,
        notify_zeromq, canary_resource: _canary_resource,
        timeout_per_resource: _timeout_per_resource, notify_grpc,
        skip_unchanged: _skip_unchanged, retry_backoff: _retry_backoff,
        notify_sqs, plan_output_file: _plan_output_file,
        resource_priority: _resource_priority, apply_window: _apply_window,
        fail_fast_machine: _fail_fast_machine, notify_mattermost,
        cooldown: _cooldown, exclude_machine: _exclude_machine,
        notify_ntfy, only_machine: _only_machine,
        notify_webhook_headers: _notify_webhook_headers,
        notify_log: _notify_log,
        notify_exec: _notify_exec,
        notify_file: _notify_file,
        notify_json: _notify_json,
        notify_slack_webhook: _notify_slack_webhook,
        notify_telegram: _notify_telegram,
        notify_webhook_v2: _notify_webhook_v2,
    }) = cmd
    else {
        unreachable!()
    };

    if let Some(ref msg) = confirmation_message {
        println!("Confirmation required: {}", msg);
        println!("Proceeding with apply...");
    }
    if dry_run_verbose { return cmd_apply_dry_run_graph(&file); }
    if let Some(ref script) = pre_flight {
        run_script_check(script).map_err(|e| format!("Pre-flight check failed: {}", e))?;
    }
    if dry_run_graph { return cmd_apply_dry_run_graph(&file); }
    if dry_run_cost {
        let sd = resolve_state_dir(&state_dir, workspace.as_deref());
        return cmd_apply_dry_run_cost(&file, &sd, machine.as_deref());
    }
    if let Some(ref cm) = canary_machine {
        let sd = resolve_state_dir(&state_dir, workspace.as_deref());
        return cmd_apply_canary_machine(&file, &sd, cm, &params, timeout);
    }
    if abort_on_drift {
        let sd = resolve_state_dir(&state_dir, workspace.as_deref());
        let drift_result = cmd_drift(&file, &sd, machine.as_deref(), true, None, false, false, false, false, env_file.as_deref());
        if drift_result.is_err() {
            return Err("Aborting apply: drift detected. Resolve drift before applying.".to_string());
        }
    }
    if let Some(ref script) = pre_script { run_pre_script(script)?; }
    if let Some(ref url) = webhook_before { send_webhook_before(url, &file); }
    if preview {
        let sd = resolve_state_dir(&state_dir, workspace.as_deref());
        return cmd_plan(&file, &sd, machine.as_deref(), resource.as_deref(), tag.as_deref(), false, true, None, env_file.as_deref(), workspace.as_deref(), false, None, false, &[]);
    }
    if let Some(ref dir) = output_scripts {
        let sd = resolve_state_dir(&state_dir, workspace.as_deref());
        return cmd_plan(&file, &sd, machine.as_deref(), resource.as_deref(), tag.as_deref(), false, false, Some(dir), env_file.as_deref(), workspace.as_deref(), false, None, false, &[]);
    }
    if backup {
        let sd = resolve_state_dir(&state_dir, workspace.as_deref());
        let _ = cmd_snapshot_save(&format!("pre-apply-{}", chrono_now_compact()), &sd);
    }
    if let Some(ref snap_name) = snapshot_before {
        let sd = resolve_state_dir(&state_dir, workspace.as_deref());
        let _ = cmd_snapshot_save(snap_name, &sd);
    }
    if diff_only {
        let sd = resolve_state_dir(&state_dir, workspace.as_deref());
        return cmd_plan(&file, &sd, machine.as_deref(), resource.as_deref(), tag.as_deref(), json, verbose, None, env_file.as_deref(), workspace.as_deref(), false, None, false, &[]);
    }
    if check {
        return cmd_check(&file, machine.as_deref(), resource.as_deref(), tag.as_deref(), json, verbose);
    }
    let sd = resolve_state_dir(&state_dir, workspace.as_deref());

    if let Some(limit) = cost_limit {
        check_cost_limit(&file, &sd, machine.as_deref(), tag.as_deref(), limit)?;
    }

    let result = cmd_apply(
        &file, &sd, machine.as_deref(), resource.as_deref(), tag.as_deref(),
        group.as_deref(), force, dry_run, no_tripwire, &params, auto_commit, timeout,
        json, verbose, env_file.as_deref(), workspace.as_deref(), report, force_unlock,
        output.as_deref(), progress, timing, retry, yes, parallel, resource_timeout,
        rollback_on_failure, max_parallel, notify.as_deref(), subset.as_deref(),
        confirm_destructive, exclude.as_deref(), sequential,
    );

    let opts = NotifyOpts {
        slack: notify_slack.as_deref(),
        email: notify_email.as_deref(),
        webhook: notify_webhook.as_deref(),
        teams: notify_teams.as_deref(),
        discord: notify_discord.as_deref(),
        opsgenie: notify_opsgenie.as_deref(),
        datadog: notify_datadog.as_deref(),
        newrelic: notify_newrelic.as_deref(),
        grafana: notify_grafana.as_deref(),
        victorops: notify_victorops.as_deref(),
        msteams_adaptive: notify_msteams_adaptive.as_deref(),
        incident: notify_incident.as_deref(),
        sns: notify_sns.as_deref(),
        pubsub: notify_pubsub.as_deref(),
        eventbridge: notify_eventbridge.as_deref(),
        kafka: notify_kafka.as_deref(),
        azure_servicebus: notify_azure_servicebus.as_deref(),
        gcp_pubsub_v2: notify_gcp_pubsub_v2.as_deref(),
        rabbitmq: notify_rabbitmq.as_deref(),
        nats: notify_nats.as_deref(),
        mqtt: notify_mqtt.as_deref(),
        redis: notify_redis.as_deref(),
        amqp: notify_amqp.as_deref(),
        stomp: notify_stomp.as_deref(),
        zeromq: notify_zeromq.as_deref(),
        grpc: notify_grpc.as_deref(),
        sqs: notify_sqs.as_deref(),
        mattermost: notify_mattermost.as_deref(),
        ntfy: notify_ntfy.as_deref(),
        pagerduty: notify_pagerduty.as_deref(),
    };
    send_apply_notifications(&opts, &result, &file);

    if let Some(ref script) = post_script {
        let _ = std::process::Command::new("bash").arg(script).status();
    }
    if let Some(ref script) = post_flight { run_post_flight(script); }

    result
}
