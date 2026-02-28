//\! CLI Args structs for apply-related commands.

use std::path::PathBuf;


#[derive(clap::Args, Debug)]
pub struct ApplyArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Target specific machine
    #[arg(short, long)]
    pub machine: Option<String>,

    /// Target specific resource
    #[arg(short, long)]
    pub resource: Option<String>,

    /// Filter to resources with this tag
    #[arg(short, long)]
    pub tag: Option<String>,

    /// FJ-281: Filter to resources in this group
    #[arg(short, long)]
    pub group: Option<String>,

    /// Force re-apply all resources
    #[arg(long)]
    pub force: bool,

    /// Show what would be executed without running
    #[arg(long)]
    pub dry_run: bool,

    /// Skip provenance tracing (faster, less safe)
    #[arg(long)]
    pub no_tripwire: bool,

    /// Override a parameter (KEY=VALUE)
    #[arg(short, long = "param", value_name = "KEY=VALUE")]
    pub params: Vec<String>,

    /// Git commit state after successful apply
    #[arg(long)]
    pub auto_commit: bool,

    /// Timeout per transport operation (seconds)
    #[arg(long)]
    pub timeout: Option<u64>,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Output apply results as JSON
    #[arg(long)]
    pub json: bool,

    /// FJ-211: Load param overrides from external YAML file
    #[arg(long)]
    pub env_file: Option<PathBuf>,

    /// FJ-210: Use workspace (overrides state dir to state/<workspace>/)
    #[arg(short = 'w', long)]
    pub workspace: Option<String>,

    /// FJ-226: Run check scripts instead of apply scripts (exit 2 = changes needed)
    #[arg(long)]
    pub check: bool,

    /// FJ-262: Print per-resource timing report after apply
    #[arg(long)]
    pub report: bool,

    /// FJ-266: Force-remove stale state lock before apply
    #[arg(long)]
    pub force_unlock: bool,

    /// FJ-270: Output mode — 'events' for newline-delimited JSON events
    #[arg(long)]
    pub output: Option<String>,

    /// FJ-272: Show progress counter [N/total] during apply
    #[arg(long)]
    pub progress: bool,

    /// FJ-276: Show timing breakdown after apply
    #[arg(long)]
    pub timing: bool,

    /// FJ-283: Retry failed resources up to N times with exponential backoff
    #[arg(long, default_value = "0")]
    pub retry: u32,

    /// FJ-286: Skip confirmation prompt (CI mode)
    #[arg(long)]
    pub yes: bool,

    /// FJ-290: Enable parallel wave execution (overrides policy.parallel_resources)
    #[arg(long)]
    pub parallel: bool,

    /// FJ-304: Per-resource timeout in seconds (kill script if exceeded)
    #[arg(long)]
    pub resource_timeout: Option<u64>,

    /// FJ-310: Auto-rollback to previous state on any resource failure
    #[arg(long)]
    pub rollback_on_failure: bool,

    /// FJ-313: Max concurrent resources per parallel wave
    #[arg(long)]
    pub max_parallel: Option<usize>,

    /// FJ-317: POST JSON results to webhook URL after apply
    #[arg(long)]
    pub notify: Option<String>,

    /// FJ-331: Apply only resources matching glob pattern (e.g., web-*)
    #[arg(long)]
    pub subset: Option<String>,

    /// FJ-335: Require confirmation for destructive (destroy/remove) actions
    #[arg(long)]
    pub confirm_destructive: bool,

    /// FJ-342: Snapshot state before apply (auto-create named backup)
    #[arg(long)]
    pub backup: bool,

    /// FJ-345: Exclude resources matching glob pattern from apply
    #[arg(long)]
    pub exclude: Option<String>,

    /// FJ-347: Force sequential execution (no parallel waves)
    #[arg(long)]
    pub sequential: bool,

    /// FJ-350: Show what would change without generating scripts (faster than dry-run)
    #[arg(long)]
    pub diff_only: bool,

    /// FJ-353: Post apply results to Slack webhook URL
    #[arg(long)]
    pub notify_slack: Option<String>,

    /// FJ-356: Abort apply if resource change count exceeds limit
    #[arg(long)]
    pub cost_limit: Option<usize>,

    /// FJ-360: Show generated scripts before execution
    #[arg(long)]
    pub preview: bool,

    /// FJ-362: Boolean tag filter expression (e.g., "web AND NOT staging")
    #[arg(long)]
    pub tag_filter: Option<String>,

    /// FJ-365: Write generated scripts to directory for manual review
    #[arg(long)]
    pub output_scripts: Option<PathBuf>,

    /// FJ-370: Resume from last failed resource (checkpoint recovery)
    #[arg(long)]
    pub resume: bool,

    /// FJ-373: Interactive per-resource confirmation before execution
    #[arg(long)]
    pub confirm: bool,

    /// FJ-377: Allow N failures before stopping (override jidoka)
    #[arg(long)]
    pub max_failures: Option<usize>,

    /// FJ-380: Limit concurrent SSH connections
    #[arg(long)]
    pub rate_limit: Option<usize>,

    /// FJ-383: Add metadata labels to apply run (KEY=VALUE)
    #[arg(long = "label", value_name = "KEY=VALUE")]
    pub labels: Vec<String>,

    /// FJ-386: Execute a previously saved plan file
    #[arg(long)]
    pub plan_file: Option<PathBuf>,

    /// FJ-393: Send apply results via email
    #[arg(long)]
    pub notify_email: Option<String>,

    /// FJ-396: Skip specific resource during apply
    #[arg(long)]
    pub skip: Option<String>,

    /// FJ-403: Named snapshot before apply
    #[arg(long)]
    pub snapshot_before: Option<String>,

    /// FJ-406: Global concurrency limit across all machines
    #[arg(long)]
    pub concurrency: Option<usize>,

    /// FJ-410: POST to webhook before apply starts
    #[arg(long)]
    pub webhook_before: Option<String>,

    /// FJ-413: Auto-rollback to named snapshot on failure
    #[arg(long)]
    pub rollback_snapshot: Option<String>,

    /// FJ-420: Delay between retry attempts in seconds
    #[arg(long)]
    pub retry_delay: Option<u64>,

    /// FJ-423: Apply only resources matching any of the given tags
    #[arg(long, value_delimiter = ',')]
    pub tags: Vec<String>,

    /// FJ-426: Write detailed apply log to file
    #[arg(long)]
    pub log_file: Option<PathBuf>,

    /// FJ-430: Attach a comment to the apply run in event log
    #[arg(long)]
    pub comment: Option<String>,

    /// FJ-433: Apply only resources whose config hash changed since last apply
    #[arg(long)]
    pub only_changed: bool,

    /// FJ-436: Run a script before apply starts (pre-flight check)
    #[arg(long)]
    pub pre_script: Option<PathBuf>,

    /// FJ-440: Output dry-run results as structured JSON
    #[arg(long)]
    pub dry_run_json: bool,

    /// FJ-443: POST structured results to any webhook URL
    #[arg(long)]
    pub notify_webhook: Option<String>,

    /// FJ-446: Run a script after apply completes (post-flight)
    #[arg(long)]
    pub post_script: Option<PathBuf>,

    /// FJ-450: Require explicit approval before destructive changes
    #[arg(long)]
    pub approval_required: bool,

    /// FJ-453: Apply to N% of machines first, then rest (gradual rollout)
    #[arg(long)]
    pub canary_percent: Option<u32>,

    /// FJ-456: Schedule apply for later execution (cron expression)
    #[arg(long)]
    pub schedule: Option<String>,

    /// FJ-460: Apply using named environment config overlay
    #[arg(long, name = "env-name")]
    pub env_name: Option<String>,

    /// FJ-463: Show unified diff of what would change
    #[arg(long)]
    pub dry_run_diff: bool,

    /// FJ-466: Send apply events to PagerDuty
    #[arg(long)]
    pub notify_pagerduty: Option<String>,

    /// FJ-470: Process resources in batches of N (memory-bounded execution)
    #[arg(long)]
    pub batch_size: Option<usize>,

    /// FJ-473: Send apply results to Microsoft Teams webhook
    #[arg(long)]
    pub notify_teams: Option<String>,

    /// FJ-476: Abort apply if drift detected before execution
    #[arg(long)]
    pub abort_on_drift: bool,

    /// FJ-480: Show one-line summary of what would change per machine
    #[arg(long)]
    pub dry_run_summary: bool,

    /// FJ-483: Send apply results to Discord webhook
    #[arg(long)]
    pub notify_discord: Option<String>,

    /// FJ-486: Auto-rollback if more than N resources fail
    #[arg(long)]
    pub rollback_on_threshold: Option<usize>,

    /// FJ-490: Expose apply metrics on HTTP port for Prometheus scraping
    #[arg(long)]
    pub metrics_port: Option<u16>,

    /// FJ-493: Send apply alerts to OpsGenie
    #[arg(long)]
    pub notify_opsgenie: Option<String>,

    /// FJ-496: Pause apply after N consecutive failures (circuit breaker)
    #[arg(long)]
    pub circuit_breaker: Option<usize>,

    /// FJ-500: Require named approvers before apply proceeds
    #[arg(long)]
    pub require_approval: Option<String>,

    /// FJ-503: Send apply events to Datadog
    #[arg(long)]
    pub notify_datadog: Option<String>,

    /// FJ-506: Restrict applies to defined maintenance windows (cron expression)
    #[arg(long)]
    pub change_window: Option<String>,

    /// FJ-510: Apply to single machine first as canary before fleet
    #[arg(long)]
    pub canary_machine: Option<String>,

    /// FJ-513: Send apply events to New Relic
    #[arg(long)]
    pub notify_newrelic: Option<String>,

    /// FJ-516: Abort apply if it exceeds time limit (seconds)
    #[arg(long)]
    pub max_duration: Option<u64>,

    /// FJ-520: Send apply annotations to Grafana
    #[arg(long)]
    pub notify_grafana: Option<String>,

    /// FJ-523: Apply at most N resources per minute (throttle)
    #[arg(long)]
    pub rate_limit_resources: Option<usize>,

    /// FJ-526: Save intermediate state during long applies (seconds)
    #[arg(long)]
    pub checkpoint_interval: Option<u64>,

    /// FJ-530: Send apply events to VictorOps/Splunk On-Call
    #[arg(long)]
    pub notify_victorops: Option<String>,

    /// FJ-533: Blue/green deployment with machine pairs
    #[arg(long)]
    pub blue_green: Option<String>,

    /// FJ-536: Show estimated cost without applying
    #[arg(long)]
    pub dry_run_cost: bool,

    /// FJ-540: Send Adaptive Card to MS Teams
    #[arg(long)]
    pub notify_msteams_adaptive: Option<String>,

    /// FJ-543: Progressive rollout (apply to N% of machines)
    #[arg(long)]
    pub progressive: Option<u8>,

    /// FJ-546: POST for approval before applying (GitOps gate)
    #[arg(long)]
    pub approval_webhook: Option<String>,

    /// FJ-550: POST incident to PagerDuty/Opsgenie with full context
    #[arg(long)]
    pub notify_incident: Option<String>,

    /// FJ-556: Require named sign-off before apply proceeds
    #[arg(long)]
    pub sign_off: Option<String>,

    /// FJ-560: Publish apply events to AWS SNS topic
    #[arg(long)]
    pub notify_sns: Option<String>,

    /// FJ-563: POST OpenTelemetry spans for apply execution
    #[arg(long)]
    pub telemetry_endpoint: Option<String>,

    /// FJ-566: Attach runbook URL to apply for audit trail
    #[arg(long)]
    pub runbook: Option<String>,

    /// FJ-570: Publish apply events to Google Cloud Pub/Sub
    #[arg(long)]
    pub notify_pubsub: Option<String>,

    /// FJ-573: Fleet-wide rollout strategy (parallel, rolling, canary)
    #[arg(long)]
    pub fleet_strategy: Option<String>,

    /// FJ-576: Run validation script before apply proceeds
    #[arg(long)]
    pub pre_check: Option<String>,

    /// FJ-580: Publish to AWS EventBridge for event-driven workflows
    #[arg(long)]
    pub notify_eventbridge: Option<String>,

    /// FJ-583: Show execution graph without applying
    #[arg(long)]
    pub dry_run_graph: bool,

    /// FJ-586: Run validation script after apply completes
    #[arg(long)]
    pub post_check: Option<String>,

    /// FJ-590: Publish apply events to Apache Kafka
    #[arg(long)]
    pub notify_kafka: Option<String>,

    /// FJ-593: Retry failed resources up to N times
    #[arg(long)]
    pub max_retries: Option<u32>,

    /// FJ-596: Auto-rollback if issues detected within window
    #[arg(long)]
    pub rollback_window: Option<String>,

    /// FJ-600: Publish apply events to Azure Service Bus
    #[arg(long)]
    pub notify_azure_servicebus: Option<String>,

    /// FJ-603: Timeout for interactive approval prompts
    #[arg(long)]
    pub approval_timeout: Option<String>,

    /// FJ-606: Run pre-flight validation script before apply
    #[arg(long)]
    pub pre_flight: Option<String>,

    /// FJ-610: Enhanced GCP Pub/Sub notification with ordering keys
    #[arg(long)]
    pub notify_gcp_pubsub_v2: Option<String>,

    /// FJ-613: Create named checkpoint before apply
    #[arg(long)]
    pub checkpoint: Option<String>,

    /// FJ-616: Run post-flight validation script after apply
    #[arg(long)]
    pub post_flight: Option<String>,

    /// FJ-620: Publish events to RabbitMQ
    #[arg(long)]
    pub notify_rabbitmq: Option<String>,

    /// FJ-623: Require named approval gate before apply
    #[arg(long)]
    pub gate: Option<String>,

    /// FJ-630: Publish events to NATS messaging
    #[arg(long)]
    pub notify_nats: Option<String>,

    /// FJ-633: Verbose dry-run showing all planned commands
    #[arg(long)]
    pub dry_run_verbose: bool,

    /// FJ-636: Explain what each step will do before executing
    #[arg(long)]
    pub explain: bool,

    /// FJ-640: Publish apply events to MQTT broker for IoT integration
    #[arg(long)]
    pub notify_mqtt: Option<String>,

    /// FJ-643: Custom confirmation message before apply
    #[arg(long)]
    pub confirmation_message: Option<String>,

    /// FJ-646: Only show summary, no per-resource output
    #[arg(long)]
    pub summary_only: bool,

    /// FJ-650: Publish events to Redis pub/sub
    #[arg(long)]
    pub notify_redis: Option<String>,

    /// FJ-660: Publish events to AMQP exchange
    #[arg(long)]
    pub notify_amqp: Option<String>,

    /// FJ-663: Run command before each resource apply
    #[arg(long)]
    pub pre_apply_hook: Option<String>,

    /// FJ-666: Only apply resources matching glob pattern
    #[arg(long)]
    pub resource_filter: Option<String>,

    /// FJ-670: Publish events to STOMP destination
    #[arg(long)]
    pub notify_stomp: Option<String>,

    /// FJ-673: Run command after each resource apply
    #[arg(long)]
    pub post_apply_hook: Option<String>,

    /// FJ-676: Output shell scripts instead of executing
    #[arg(long)]
    pub dry_run_shell: bool,

    /// FJ-680: Publish events to ZeroMQ socket
    #[arg(long)]
    pub notify_zeromq: Option<String>,

    /// FJ-683: Apply single resource first as canary
    #[arg(long)]
    pub canary_resource: Option<String>,

    /// FJ-686: Per-resource timeout override in seconds
    #[arg(long)]
    pub timeout_per_resource: Option<u64>,
    /// FJ-690: Publish events to gRPC endpoint
    #[arg(long)]
    pub notify_grpc: Option<String>,
    /// FJ-693: Skip resources whose hash hasn't changed
    #[arg(long)]
    pub skip_unchanged: bool,
    /// FJ-696: Exponential backoff factor for retries
    #[arg(long)]
    pub retry_backoff: Option<f64>,
    /// FJ-700: Publish events to AWS SQS queue
    #[arg(long)]
    pub notify_sqs: Option<String>,
    /// FJ-703: Save plan output to file
    #[arg(long)]
    pub plan_output_file: Option<String>,
    /// FJ-706: Set execution priority for specific resources (name=priority)
    #[arg(long)]
    pub resource_priority: Vec<String>,
    /// FJ-713: Time window for apply operations in seconds
    #[arg(long)]
    pub apply_window: Option<u64>,
    /// FJ-716: Stop all machines on first machine failure
    #[arg(long)]
    pub fail_fast_machine: bool,
    /// FJ-720: Publish events to Mattermost webhook
    #[arg(long)]
    pub notify_mattermost: Option<String>,
    /// FJ-723: Cooldown between resource applies in seconds
    #[arg(long)]
    pub cooldown: Option<u64>,
    /// FJ-726: Exclude specific machine from apply
    #[arg(long)]
    pub exclude_machine: Option<String>,
    /// FJ-730: Publish events to ntfy.sh topic
    #[arg(long)]
    pub notify_ntfy: Option<String>,
    /// FJ-736: Apply only to specific machine
    #[arg(long)]
    pub only_machine: Option<String>,
    /// FJ-744: Custom headers for webhook notifications (JSON string)
    #[arg(long)]
    pub notify_webhook_headers: Option<String>,
    /// FJ-752: Append structured JSON events to a local file
    #[arg(long)]
    pub notify_log: Option<std::path::PathBuf>,

    /// FJ-760: Run arbitrary command as notification handler
    #[arg(long)]
    pub notify_exec: Option<String>,

    /// FJ-768: Write one-line status to a file (for monitoring)
    #[arg(long)]
    pub notify_file: Option<std::path::PathBuf>,

    /// FJ-776: Print structured JSON notification to stdout
    #[arg(long)]
    pub notify_json: bool,

    /// FJ-784: Send apply results to Slack webhook URL
    #[arg(long)]
    pub notify_slack_webhook: Option<String>,

    /// FJ-792: Send apply results to Telegram bot
    #[arg(long)]
    pub notify_telegram: Option<String>,

    /// FJ-800: Enhanced webhook with retry and custom headers
    #[arg(long)]
    pub notify_webhook_v2: Option<String>,

    /// FJ-816: Discord webhook with rich embeds for apply results
    #[arg(long)]
    pub notify_discord_webhook: Option<String>,

    /// FJ-824: MS Teams webhook with adaptive card for apply results
    #[arg(long)]
    pub notify_teams_webhook: Option<String>,

    /// FJ-832: Slack Block Kit rich notifications
    #[arg(long)]
    pub notify_slack_blocks: Option<String>,

    /// FJ-840: Custom notification template support
    #[arg(long)]
    pub notify_custom_template: Option<String>,

    /// FJ-848: Custom webhook with configurable headers
    #[arg(long)]
    pub notify_custom_webhook: Option<String>,

    /// FJ-856: Custom HTTP headers for webhook notifications
    #[arg(long)]
    pub notify_custom_headers: Option<String>,

    /// FJ-864: Custom JSON template for webhook notifications
    #[arg(long)]
    pub notify_custom_json: Option<String>,

    /// FJ-872: Filter notifications by resource type or status
    #[arg(long)]
    pub notify_custom_filter: Option<String>,

    /// FJ-880: Retry failed notifications with exponential backoff
    #[arg(long)]
    pub notify_custom_retry: Option<String>,
    /// FJ-888: Transform notification payload via template
    #[arg(long)]
    pub notify_custom_transform: Option<String>,
    /// FJ-896: Batch multiple resource notifications into single payload
    #[arg(long)]
    pub notify_custom_batch: Option<String>,
    /// FJ-904: Deduplicate repeated notifications
    #[arg(long)]
    pub notify_custom_deduplicate: Option<String>,
    /// FJ-912: Throttle notification rate per time window
    #[arg(long)]
    pub notify_custom_throttle: Option<String>,
    /// FJ-920: Aggregate multiple events into summary notification
    #[arg(long)]
    pub notify_custom_aggregate: Option<String>,
    /// FJ-928: Assign priority levels to notifications based on severity
    #[arg(long)]
    pub notify_custom_priority: Option<String>,
    /// FJ-936: Route notifications to different channels based on resource type
    #[arg(long)]
    pub notify_custom_routing: Option<String>,
    /// FJ-944: Deduplicate notifications within a time window
    #[arg(long)]
    pub notify_custom_dedup_window: Option<String>,
    /// FJ-952: Rate-limit notification delivery per channel
    #[arg(long)]
    pub notify_custom_rate_limit: Option<String>,
    /// FJ-960: Exponential backoff for failed notification retries
    #[arg(long)]
    pub notify_custom_backoff: Option<String>,
    /// FJ-968: Circuit breaker pattern for notification failures
    #[arg(long)]
    pub notify_custom_circuit_breaker: Option<String>,
    /// FJ-976: Route failed notifications to a dead-letter queue
    #[arg(long)]
    pub notify_custom_dead_letter: Option<String>,
    /// FJ-984: Escalate notifications based on failure severity
    #[arg(long)]
    pub notify_custom_escalation: Option<String>,
    /// FJ-992: Correlate notifications by resource group and time window
    #[arg(long)]
    pub notify_custom_correlation: Option<String>,
    /// FJ-1000: Sample notifications at a configurable rate
    #[arg(long)]
    pub notify_custom_sampling: Option<String>,
    /// FJ-1016: Aggregate notifications into a periodic digest
    #[arg(long)]
    pub notify_custom_digest: Option<String>,
    /// FJ-1020: Filter notifications by severity level
    #[arg(long)]
    pub notify_custom_severity_filter: Option<String>,
}

