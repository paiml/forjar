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

    // GH-211: refuse before ANY early exit, hook or backup runs. A flag that
    // does nothing must not be able to reach a code path that does something —
    // the whole defect class is "forjar acted while ignoring what it was told".
    super::inert_flags::reject_inert_apply_flags(&args)?;

    // GH-211: a malformed --notify-*-headers value must be refused BEFORE the
    // apply, not silently dropped after it. Dropping it delivers the
    // notification unauthenticated, and the 401 that follows was swallowed too.
    validate_notify_headers(&args)?;

    if let Some(r) = apply_early_exits(&args) {
        return r;
    }
    apply_pre_checks(&args)?;
    // GH-210: --preview shows the scripts and then the apply RUNS. It used to
    // print a plan and return Ok(()) having converged nothing and written no
    // state, while exiting 0.
    if args.preview && !effective_dry_run(&args) {
        super::apply_preview::print_generated_scripts(&args)?;
    }
    if let Some(r) = apply_mode_exits(&args, verbose) {
        return r;
    }
    apply_backups(&args);
    apply_execute(&args, verbose)
}

/// GH-211: reject a `--notify-webhook-headers` value that is not a JSON object.
///
/// `--notify-custom-headers` is deliberately NOT checked here: its legacy
/// `URL|Header: Value` form is still supported, so "not JSON" is a valid value.
pub(super) fn validate_notify_headers(args: &ApplyArgs) -> Result<(), String> {
    if let Some(raw) = args.notify_webhook_headers.as_deref() {
        super::webhook_post::parse_header_json(raw)
            .map_err(|e| format!("--notify-webhook-headers: {e}"))?;
    }
    Ok(())
}

/// GH-208: every flag in the dry-run family must mean "change nothing".
///
/// `apply_early_exits` intercepts `--dry-run-verbose`, `--dry-run-graph` and
/// `--dry-run-cost`, and `args.dry_run` was passed through to `cmd_apply` — but
/// `--dry-run-shell`, `--dry-run-json`, `--dry-run-summary` and `--dry-run-diff`
/// were handled NOWHERE. They fell through to a REAL apply: files created, state
/// written, and none of their documented output produced. Measured on the
/// published 1.12.3 binary:
///
/// ```text
///   --dry-run          rc=0  a.txt=none     state=none      (correct)
///   --dry-run-shell    rc=0  a.txt=CREATED  state=WRITTEN   (!!)
///   --dry-run-json     rc=0  a.txt=CREATED  state=WRITTEN   (!!)
///   --dry-run-summary  rc=0  a.txt=CREATED  state=WRITTEN   (!!)
///   --dry-run-diff     rc=0  a.txt=CREATED  state=WRITTEN   (!!)
/// ```
///
/// Asking for a preview and getting a mutation is the most dangerous shape a
/// flag can have, so this is deliberately computed ONCE, fail-safe: any member
/// of the family suppresses execution. Adding a new `--dry-run-*` flag without
/// adding it here can only ever be too cautious, never destructive.
pub(super) fn effective_dry_run(args: &ApplyArgs) -> bool {
    args.dry_run
        || args.dry_run_shell
        || args.dry_run_json
        || args.dry_run_summary
        || args.dry_run_diff
        || args.dry_run_cost
        || args.dry_run_graph
        || args.dry_run_verbose
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
    // GH-210: `--preview` is NOT an exit — the scripts have already been
    // printed by `dispatch_apply_cmd` and the apply proceeds below.
    if let Some(ref dir) = args.output_scripts {
        let sd = resolve_state_dir(&args.state_dir, args.workspace.as_deref());
        // GH-210: exporting scripts "for manual review" legitimately replaces
        // the apply — but say so. The shipped version exited 0 with a plan and
        // no state, and the next command failed with "cannot read state dir".
        println!(
            "--output-scripts: scripts written to {}; the apply was SKIPPED (nothing was changed).",
            dir.display()
        );
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
            args.group.as_deref(),
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
            args.group.as_deref(),
        ));
    }
    if args.check {
        let sd = resolve_state_dir(&args.state_dir, args.workspace.as_deref());
        return Some(cmd_check(
            &args.file,
            args.machine.as_deref(),
            args.resource.as_deref(),
            args.tag.as_deref(),
            &sd,
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
            // forjar#370: this branch returns BEFORE `apply_execute`, so the
            // operator gate travels with it — enforced inside the callee.
            args.operator.as_deref(),
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
    // FJ-2300: Operator authorization check
    check_operator_auth(&args.file, args.operator.as_deref())?;

    let base_sd = resolve_state_dir(&args.state_dir, args.workspace.as_deref());
    // FJ-3500: Environment-scoped state directory
    let sd = if let Some(ref env_name) = args.env_name {
        crate::core::types::environment::env_state_dir(&base_sd, env_name)
    } else {
        base_sd
    };

    if let Some(limit) = args.cost_limit {
        check_cost_limit(
            &args.file,
            &sd,
            args.machine.as_deref(),
            args.tag.as_deref(),
            limit,
        )?;
    }

    // FJ-3203: Compliance pack pre-apply gate
    if args.policy_check {
        check_compliance_packs(&args.file, &args.policy_dir, verbose)?;
    }

    // GH-211: the four scope selectors that were declared and never read.
    let scope = super::apply_scope::ApplyScope {
        skip: args.skip.as_deref(),
        only_machine: args.only_machine.as_deref(),
        exclude_machine: args.exclude_machine.as_deref(),
        resource_filter: args.resource_filter.as_deref(),
    };

    let result = cmd_apply_scoped(
        &args.file,
        &sd,
        args.machine.as_deref(),
        args.resource.as_deref(),
        args.tag.as_deref(),
        args.group.as_deref(),
        args.force,
        effective_dry_run(args),
        args.no_tripwire,
        &args.params,
        args.auto_commit,
        args.timeout,
        args.json || args.dry_run_json,
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
        args.telemetry_endpoint.as_deref(),
        args.refresh,
        args.force_tag.as_deref(),
        &[],
        &scope,
    );

    // FJ-1240: Encrypt state files after apply
    maybe_encrypt_state(args.encrypt_state, &result, &sd);

    let opts = NotifyOpts {
        slack: args.notify_slack.as_deref(),
        email: args.notify_email.as_deref(),
        webhook: args.notify_webhook.as_deref(),
        webhook_headers: args.notify_webhook_headers.as_deref(),
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

/// FJ-3203: Check compliance packs as a pre-apply gate.
fn check_compliance_packs(
    file: &std::path::Path,
    policy_dir: &std::path::Path,
    verbose: bool,
) -> Result<(), String> {
    use super::helpers::parse_and_validate;
    use crate::core::compliance_gate::{check_compliance_gate, format_gate_result};

    let config = parse_and_validate(file)?;
    let result = check_compliance_gate(policy_dir, &config, verbose)?;
    if verbose {
        eprintln!("{}", format_gate_result(&result));
    }
    if !result.passed() {
        return Err(format!(
            "compliance gate blocks apply: {} error(s) from {} pack(s)",
            result.error_count, result.packs_evaluated
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests_gh208_dry_run_family {
    use super::*;

    // GH-208: --dry-run-shell/-json/-summary/-diff performed a REAL apply on the
    // published 1.12.3 binary — files created, state written — because only
    // `args.dry_run` reached the execute guard. Asking for a preview and getting
    // a mutation is the most dangerous shape a flag can have.

    fn args_with(f: impl FnOnce(&mut ApplyArgs)) -> ApplyArgs {
        let mut a = ApplyArgs::default();
        f(&mut a);
        a
    }

    #[test]
    fn every_dry_run_flag_suppresses_execution() {
        // Asserted one flag at a time: a table of fn pointers trips the
        // very-complex-type lint and reads no better.
        assert!(
            effective_dry_run(&args_with(|a| a.dry_run = true)),
            "--dry-run"
        );
        assert!(
            effective_dry_run(&args_with(|a| a.dry_run_shell = true)),
            "--dry-run-shell must suppress execution: a flag named dry-run must never mutate"
        );
        assert!(
            effective_dry_run(&args_with(|a| a.dry_run_json = true)),
            "--dry-run-json must suppress execution"
        );
        assert!(
            effective_dry_run(&args_with(|a| a.dry_run_summary = true)),
            "--dry-run-summary must suppress execution"
        );
        assert!(
            effective_dry_run(&args_with(|a| a.dry_run_diff = true)),
            "--dry-run-diff must suppress execution"
        );
        assert!(
            effective_dry_run(&args_with(|a| a.dry_run_cost = true)),
            "--dry-run-cost"
        );
        assert!(
            effective_dry_run(&args_with(|a| a.dry_run_graph = true)),
            "--dry-run-graph"
        );
        assert!(
            effective_dry_run(&args_with(|a| a.dry_run_verbose = true)),
            "--dry-run-verbose"
        );
    }

    #[test]
    fn a_plain_apply_is_not_dry_run() {
        // The guard against "fixed" meaning "never applies anything".
        assert!(!effective_dry_run(&ApplyArgs::default()));
    }
}
