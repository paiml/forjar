use super::apply::*;
use super::apply_variants::*;
use super::check::*;
use super::commands::*;
use super::dispatch_apply::*;
use super::dispatch_notify::{send_apply_notifications, NotifyOpts};
use super::drift::*;
use super::helpers_time::*;
use super::plan::*;
use super::snapshot::*;
use super::workspace::*;

/// Dispatch the Apply command variant.
pub(crate) fn dispatch_apply_cmd(cmd: Commands, verbose: bool) -> Result<(), String> {
    let Commands::Apply(args) = cmd else {
        unreachable!()
    };
    let verbose = verbose || args.trace;

    if let Some(r) = apply_early_exits(&args) {
        return r;
    }
    apply_pre_checks(&args)?;
    if let Some(r) = apply_mode_exits(&args, verbose) {
        return r;
    }
    apply_backups(&args);
    apply_execute(&args, verbose)
}

/// Early exits for dry-run and canary modes.
fn apply_early_exits(args: &ApplyArgs) -> Option<Result<(), String>> {
    if args.dry_run_verbose {
        return Some(cmd_apply_dry_run_graph(&args.file));
    }
    if args.dry_run_graph {
        return Some(cmd_apply_dry_run_graph(&args.file));
    }
    if args.dry_run_cost {
        let sd = resolve_state_dir(&args.state_dir, args.workspace.as_deref());
        return Some(cmd_apply_dry_run_cost(
            &args.file,
            &sd,
            args.machine.as_deref(),
        ));
    }
    if let Some(ref cm) = args.canary_machine {
        let sd = resolve_state_dir(&args.state_dir, args.workspace.as_deref());
        return Some(cmd_apply_canary_machine(
            &args.file,
            &sd,
            cm,
            &args.params,
            args.timeout,
        ));
    }
    None
}

/// Pre-apply hooks: confirmation, pre-flight, drift abort, pre-script, webhooks.
fn apply_pre_checks(args: &ApplyArgs) -> Result<(), String> {
    if let Some(ref msg) = args.confirmation_message {
        println!("Confirmation required: {msg}");
        println!("Proceeding with apply...");
    }
    if let Some(ref script) = args.pre_flight {
        run_script_check(script).map_err(|e| format!("Pre-flight check failed: {e}"))?;
    }
    if args.abort_on_drift {
        let sd = resolve_state_dir(&args.state_dir, args.workspace.as_deref());
        let drift_result = cmd_drift(
            &args.file,
            &sd,
            args.machine.as_deref(),
            true,
            None,
            false,
            false,
            false,
            false,
            args.env_file.as_deref(),
        );
        if drift_result.is_err() {
            return Err(
                "Aborting apply: drift detected. Resolve drift before applying.".to_string(),
            );
        }
    }
    if let Some(ref script) = args.pre_script {
        run_pre_script(script)?;
    }
    if let Some(ref url) = args.webhook_before {
        send_webhook_before(url, &args.file);
    }
    Ok(())
}

/// Mode-specific exits: preview, output_scripts, diff_only, check, refresh, plan_file.
fn apply_mode_exits(args: &ApplyArgs, verbose: bool) -> Option<Result<(), String>> {
    if args.preview {
        let sd = resolve_state_dir(&args.state_dir, args.workspace.as_deref());
        return Some(cmd_plan(
            &args.file,
            &sd,
            args.machine.as_deref(),
            args.resource.as_deref(),
            args.tag.as_deref(),
            false,
            true,
            None,
            args.env_file.as_deref(),
            args.workspace.as_deref(),
            false,
            None,
            false,
            &[],
            None,
            false,
        ));
    }
    if let Some(ref dir) = args.output_scripts {
        let sd = resolve_state_dir(&args.state_dir, args.workspace.as_deref());
        return Some(cmd_plan(
            &args.file,
            &sd,
            args.machine.as_deref(),
            args.resource.as_deref(),
            args.tag.as_deref(),
            false,
            false,
            Some(dir),
            args.env_file.as_deref(),
            args.workspace.as_deref(),
            false,
            None,
            false,
            &[],
            None,
            false,
        ));
    }
    if args.diff_only {
        let sd = resolve_state_dir(&args.state_dir, args.workspace.as_deref());
        return Some(cmd_plan(
            &args.file,
            &sd,
            args.machine.as_deref(),
            args.resource.as_deref(),
            args.tag.as_deref(),
            args.json,
            verbose,
            None,
            args.env_file.as_deref(),
            args.workspace.as_deref(),
            false,
            None,
            false,
            &[],
            None,
            false,
        ));
    }
    if args.check {
        return Some(cmd_check(
            &args.file,
            args.machine.as_deref(),
            args.resource.as_deref(),
            args.tag.as_deref(),
            args.json,
            verbose,
        ));
    }
    if args.refresh_only {
        let sd = resolve_state_dir(&args.state_dir, args.workspace.as_deref());
        return Some(cmd_refresh_only(
            &args.file,
            &sd,
            args.machine.as_deref(),
            verbose,
            args.timeout,
            args.env_file.as_deref(),
            args.workspace.as_deref(),
        ));
    }
    if let Some(ref pf) = args.plan_file {
        let sd = resolve_state_dir(&args.state_dir, args.workspace.as_deref());
        return Some(cmd_apply_from_plan(
            &args.file,
            &sd,
            pf,
            verbose,
            args.env_file.as_deref(),
            args.workspace.as_deref(),
        ));
    }
    None
}

/// Create pre-apply backup snapshots if requested.
fn apply_backups(args: &ApplyArgs) {
    if args.backup {
        let sd = resolve_state_dir(&args.state_dir, args.workspace.as_deref());
        let _ = cmd_snapshot_save(&format!("pre-apply-{}", chrono_now_compact()), &sd);
    }
    if let Some(ref snap_name) = args.snapshot_before {
        let sd = resolve_state_dir(&args.state_dir, args.workspace.as_deref());
        let _ = cmd_snapshot_save(snap_name, &sd);
    }
}

/// Execute the main apply, notifications, and post-apply hooks.
fn apply_execute(args: &ApplyArgs, verbose: bool) -> Result<(), String> {
    let sd = resolve_state_dir(&args.state_dir, args.workspace.as_deref());

    if let Some(limit) = args.cost_limit {
        check_cost_limit(
            &args.file,
            &sd,
            args.machine.as_deref(),
            args.tag.as_deref(),
            limit,
        )?;
    }

    let result = cmd_apply(
        &args.file,
        &sd,
        args.machine.as_deref(),
        args.resource.as_deref(),
        args.tag.as_deref(),
        args.group.as_deref(),
        args.force,
        args.dry_run,
        args.no_tripwire,
        &args.params,
        args.auto_commit,
        args.timeout,
        args.json,
        verbose,
        args.env_file.as_deref(),
        args.workspace.as_deref(),
        args.report,
        args.force_unlock,
        args.output.as_deref(),
        args.progress,
        args.timing,
        args.retry,
        args.yes,
        args.parallel,
        args.resource_timeout,
        args.rollback_on_failure,
        args.max_parallel,
        args.notify.as_deref(),
        args.subset.as_deref(),
        args.confirm_destructive,
        args.exclude.as_deref(),
        args.sequential,
    );

    // FJ-1240: Encrypt state files after apply
    maybe_encrypt_state(args.encrypt_state, &result, &sd);

    let opts = NotifyOpts {
        slack: args.notify_slack.as_deref(),
        email: args.notify_email.as_deref(),
        webhook: args.notify_webhook.as_deref(),
        teams: args.notify_teams.as_deref(),
        discord: args.notify_discord.as_deref(),
        opsgenie: args.notify_opsgenie.as_deref(),
        datadog: args.notify_datadog.as_deref(),
        newrelic: args.notify_newrelic.as_deref(),
        grafana: args.notify_grafana.as_deref(),
        victorops: args.notify_victorops.as_deref(),
        msteams_adaptive: args.notify_msteams_adaptive.as_deref(),
        incident: args.notify_incident.as_deref(),
        sns: args.notify_sns.as_deref(),
        pubsub: args.notify_pubsub.as_deref(),
        eventbridge: args.notify_eventbridge.as_deref(),
        kafka: args.notify_kafka.as_deref(),
        azure_servicebus: args.notify_azure_servicebus.as_deref(),
        gcp_pubsub_v2: args.notify_gcp_pubsub_v2.as_deref(),
        rabbitmq: args.notify_rabbitmq.as_deref(),
        nats: args.notify_nats.as_deref(),
        mqtt: args.notify_mqtt.as_deref(),
        redis: args.notify_redis.as_deref(),
        amqp: args.notify_amqp.as_deref(),
        stomp: args.notify_stomp.as_deref(),
        zeromq: args.notify_zeromq.as_deref(),
        grpc: args.notify_grpc.as_deref(),
        sqs: args.notify_sqs.as_deref(),
        mattermost: args.notify_mattermost.as_deref(),
        ntfy: args.notify_ntfy.as_deref(),
        pagerduty: args.notify_pagerduty.as_deref(),
        discord_webhook: args.notify_discord_webhook.as_deref(),
        teams_webhook: args.notify_teams_webhook.as_deref(),
        slack_blocks: args.notify_slack_blocks.as_deref(),
        custom_template: args.notify_custom_template.as_deref(),
        custom_webhook: args.notify_custom_webhook.as_deref(),
        custom_headers: args.notify_custom_headers.as_deref(),
        custom_json: args.notify_custom_json.as_deref(),
        custom_filter: args.notify_custom_filter.as_deref(),
        custom_retry: args.notify_custom_retry.as_deref(),
        custom_transform: args.notify_custom_transform.as_deref(),
        custom_batch: args.notify_custom_batch.as_deref(),
        custom_deduplicate: args.notify_custom_deduplicate.as_deref(),
        custom_throttle: args.notify_custom_throttle.as_deref(),
        custom_aggregate: args.notify_custom_aggregate.as_deref(),
        custom_priority: args.notify_custom_priority.as_deref(),
        custom_routing: args.notify_custom_routing.as_deref(),
        custom_dedup_window: args.notify_custom_dedup_window.as_deref(),
        custom_rate_limit: args.notify_custom_rate_limit.as_deref(),
        custom_backoff: args.notify_custom_backoff.as_deref(),
        custom_circuit_breaker: args.notify_custom_circuit_breaker.as_deref(),
        custom_dead_letter: args.notify_custom_dead_letter.as_deref(),
        custom_escalation: args.notify_custom_escalation.as_deref(),
        custom_correlation: args.notify_custom_correlation.as_deref(),
        custom_sampling: args.notify_custom_sampling.as_deref(),
        custom_digest: args.notify_custom_digest.as_deref(),
        custom_severity_filter: args.notify_custom_severity_filter.as_deref(),
    };
    send_apply_notifications(&opts, &result, &args.file);

    if let Some(ref script) = args.post_script {
        let _ = std::process::Command::new("bash").arg(script).status();
    }
    if let Some(ref script) = args.post_flight {
        run_post_flight(script);
    }

    result
}
