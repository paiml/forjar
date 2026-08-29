//! Mapping the apply CLI's notification flags onto `NotifyOpts`.
//!
//! This is a pure adapter between two shapes of the same information: ~60
//! `--notify-*` flags on `ApplyArgs`, and the borrowed struct the notifier
//! reads. It lives apart from `dispatch_apply_b` because it is the only part of
//! the apply path with no control flow in it — a flat, mechanical field-for-
//! field correspondence that grows every time a transport is added, and that
//! otherwise dwarfs the orchestration it was sitting inside.

use super::commands::ApplyArgs;
use super::dispatch_notify::NotifyOpts;

/// Borrow every `--notify-*` value out of `args` into the notifier's options.
pub(super) fn notify_opts_from_args(args: &ApplyArgs) -> NotifyOpts<'_> {
    NotifyOpts {
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
    }
}
