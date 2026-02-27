//! CLI Commands enum and sub-command enums.

use clap::Subcommand;
use std::path::PathBuf;


#[derive(Subcommand, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Commands {
    /// Initialize a new forjar project
    Init {
        /// Directory to initialize (default: current)
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Validate forjar.yaml without connecting to machines
    Validate {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// FJ-282: Extended validation — check machine refs, paths, deps, templates
        #[arg(long)]
        strict: bool,

        /// Output validation result as JSON
        #[arg(long)]
        json: bool,

        /// FJ-330: Show fully expanded config after template resolution
        #[arg(long)]
        dry_expand: bool,

        /// FJ-381: Validate against specific schema version
        #[arg(long)]
        schema_version: Option<String>,

        /// FJ-391: Validate all cross-references, machine existence, and param usage
        #[arg(long)]
        exhaustive: bool,

        /// FJ-401: Validate against external policy rules file
        #[arg(long)]
        policy_file: Option<PathBuf>,

        /// FJ-411: Test SSH connectivity to all machines during validation
        #[arg(long)]
        check_connectivity: bool,

        /// FJ-421: Verify all template variables resolve
        #[arg(long)]
        check_templates: bool,

        /// FJ-431: Verify dependency ordering matches resource declaration order
        #[arg(long)]
        strict_deps: bool,

        /// FJ-441: Scan config for hardcoded secrets or credentials
        #[arg(long)]
        check_secrets: bool,

        /// FJ-451: Verify all resources produce idempotent scripts
        #[arg(long)]
        check_idempotency: bool,

        /// FJ-461: Verify all resources have drift detection configured
        #[arg(long)]
        check_drift_coverage: bool,

        /// FJ-471: Detect indirect circular dependencies via transitive closure
        #[arg(long)]
        check_cycles_deep: bool,

        /// FJ-481: Enforce resource naming conventions (kebab-case, prefix rules)
        #[arg(long)]
        check_naming: bool,

        /// FJ-491: Detect resources targeting the same path/port/name on same machine
        #[arg(long)]
        check_overlaps: bool,

        /// FJ-501: Enforce resource count limits per machine/type
        #[arg(long)]
        check_limits: bool,

        /// FJ-511: Warn on resources with high dependency fan-out
        #[arg(long)]
        check_complexity: bool,

        /// FJ-521: Scan for insecure permissions, ports, or user configs
        #[arg(long)]
        check_security: bool,

        /// FJ-531: Warn on deprecated resource fields or types
        #[arg(long)]
        check_deprecation: bool,

        /// FJ-541: Score drift risk based on resource volatility
        #[arg(long)]
        check_drift_risk: bool,

        /// FJ-551: Validate against compliance policy (CIS, SOC2)
        #[arg(long)]
        check_compliance: Option<String>,

        /// FJ-561: Check resources for platform-specific assumptions
        #[arg(long)]
        check_portability: bool,

        /// FJ-571: Validate resource counts don't exceed per-machine limits
        #[arg(long)]
        check_resource_limits: bool,

        /// FJ-581: Detect resources not referenced by any dependency chain
        #[arg(long)]
        check_unused: bool,

        /// FJ-591: Validate all depends_on references resolve correctly
        #[arg(long)]
        check_dependencies: bool,

        /// FJ-601: Validate resource ownership/mode fields are secure
        #[arg(long)]
        check_permissions: bool,

        /// FJ-611: Deep idempotency analysis with simulation
        #[arg(long)]
        check_idempotency_deep: bool,

        /// FJ-621: Verify machines are reachable before apply
        #[arg(long)]
        check_machine_reachability: bool,

        /// FJ-631: Detect circular template/param references
        #[arg(long)]
        check_circular_refs: bool,

        /// FJ-641: Enforce naming conventions across resources
        #[arg(long)]
        check_naming_conventions: bool,

        /// FJ-661: Ensure all resources have consistent ownership
        #[arg(long)]
        check_owner_consistency: bool,

        /// FJ-671: Detect overlapping file paths across resources
        #[arg(long)]
        check_path_conflicts: bool,

        /// FJ-681: Validate service dependency chains are satisfiable
        #[arg(long)]
        check_service_deps: bool,
        /// FJ-691: Validate all template variables are defined
        #[arg(long)]
        check_template_vars: bool,
        /// FJ-701: Validate file mode consistency across resources
        #[arg(long)]
        check_mode_consistency: bool,
        /// FJ-711: Validate user/group consistency across resources
        #[arg(long)]
        check_group_consistency: bool,
        /// FJ-721: Validate mount point paths don't conflict
        #[arg(long)]
        check_mount_points: bool,
    },

    /// Show execution plan (diff desired vs current)
    Plan {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Target specific machine
        #[arg(short, long)]
        machine: Option<String>,

        /// Target specific resource
        #[arg(short, long)]
        resource: Option<String>,

        /// Filter to resources with this tag
        #[arg(short, long)]
        tag: Option<String>,

        /// FJ-281: Filter to resources in this group
        #[arg(short, long)]
        group: Option<String>,

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Output plan as JSON
        #[arg(long)]
        json: bool,

        /// Write generated scripts to directory for auditing
        #[arg(long)]
        output_dir: Option<PathBuf>,

        /// FJ-211: Load param overrides from external YAML file
        #[arg(long)]
        env_file: Option<PathBuf>,

        /// FJ-210: Use workspace (overrides state dir to state/<workspace>/)
        #[arg(short = 'w', long)]
        workspace: Option<String>,

        /// FJ-255: Suppress content diff in plan output
        #[arg(long)]
        no_diff: bool,

        /// FJ-285: Plan single resource and its transitive dependencies
        #[arg(long)]
        target: Option<String>,

        /// FJ-312: Show estimated change cost per resource type
        #[arg(long)]
        cost: bool,

        /// FJ-333: Hypothetical param override — show plan as if param had this value
        #[arg(long = "what-if", value_name = "KEY=VALUE")]
        what_if: Vec<String>,
    },

    /// Converge infrastructure to desired state
    Apply {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Target specific machine
        #[arg(short, long)]
        machine: Option<String>,

        /// Target specific resource
        #[arg(short, long)]
        resource: Option<String>,

        /// Filter to resources with this tag
        #[arg(short, long)]
        tag: Option<String>,

        /// FJ-281: Filter to resources in this group
        #[arg(short, long)]
        group: Option<String>,

        /// Force re-apply all resources
        #[arg(long)]
        force: bool,

        /// Show what would be executed without running
        #[arg(long)]
        dry_run: bool,

        /// Skip provenance tracing (faster, less safe)
        #[arg(long)]
        no_tripwire: bool,

        /// Override a parameter (KEY=VALUE)
        #[arg(short, long = "param", value_name = "KEY=VALUE")]
        params: Vec<String>,

        /// Git commit state after successful apply
        #[arg(long)]
        auto_commit: bool,

        /// Timeout per transport operation (seconds)
        #[arg(long)]
        timeout: Option<u64>,

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Output apply results as JSON
        #[arg(long)]
        json: bool,

        /// FJ-211: Load param overrides from external YAML file
        #[arg(long)]
        env_file: Option<PathBuf>,

        /// FJ-210: Use workspace (overrides state dir to state/<workspace>/)
        #[arg(short = 'w', long)]
        workspace: Option<String>,

        /// FJ-226: Run check scripts instead of apply scripts (exit 2 = changes needed)
        #[arg(long)]
        check: bool,

        /// FJ-262: Print per-resource timing report after apply
        #[arg(long)]
        report: bool,

        /// FJ-266: Force-remove stale state lock before apply
        #[arg(long)]
        force_unlock: bool,

        /// FJ-270: Output mode — 'events' for newline-delimited JSON events
        #[arg(long)]
        output: Option<String>,

        /// FJ-272: Show progress counter [N/total] during apply
        #[arg(long)]
        progress: bool,

        /// FJ-276: Show timing breakdown after apply
        #[arg(long)]
        timing: bool,

        /// FJ-283: Retry failed resources up to N times with exponential backoff
        #[arg(long, default_value = "0")]
        retry: u32,

        /// FJ-286: Skip confirmation prompt (CI mode)
        #[arg(long)]
        yes: bool,

        /// FJ-290: Enable parallel wave execution (overrides policy.parallel_resources)
        #[arg(long)]
        parallel: bool,

        /// FJ-304: Per-resource timeout in seconds (kill script if exceeded)
        #[arg(long)]
        resource_timeout: Option<u64>,

        /// FJ-310: Auto-rollback to previous state on any resource failure
        #[arg(long)]
        rollback_on_failure: bool,

        /// FJ-313: Max concurrent resources per parallel wave
        #[arg(long)]
        max_parallel: Option<usize>,

        /// FJ-317: POST JSON results to webhook URL after apply
        #[arg(long)]
        notify: Option<String>,

        /// FJ-331: Apply only resources matching glob pattern (e.g., web-*)
        #[arg(long)]
        subset: Option<String>,

        /// FJ-335: Require confirmation for destructive (destroy/remove) actions
        #[arg(long)]
        confirm_destructive: bool,

        /// FJ-342: Snapshot state before apply (auto-create named backup)
        #[arg(long)]
        backup: bool,

        /// FJ-345: Exclude resources matching glob pattern from apply
        #[arg(long)]
        exclude: Option<String>,

        /// FJ-347: Force sequential execution (no parallel waves)
        #[arg(long)]
        sequential: bool,

        /// FJ-350: Show what would change without generating scripts (faster than dry-run)
        #[arg(long)]
        diff_only: bool,

        /// FJ-353: Post apply results to Slack webhook URL
        #[arg(long)]
        notify_slack: Option<String>,

        /// FJ-356: Abort apply if resource change count exceeds limit
        #[arg(long)]
        cost_limit: Option<usize>,

        /// FJ-360: Show generated scripts before execution
        #[arg(long)]
        preview: bool,

        /// FJ-362: Boolean tag filter expression (e.g., "web AND NOT staging")
        #[arg(long)]
        tag_filter: Option<String>,

        /// FJ-365: Write generated scripts to directory for manual review
        #[arg(long)]
        output_scripts: Option<PathBuf>,

        /// FJ-370: Resume from last failed resource (checkpoint recovery)
        #[arg(long)]
        resume: bool,

        /// FJ-373: Interactive per-resource confirmation before execution
        #[arg(long)]
        confirm: bool,

        /// FJ-377: Allow N failures before stopping (override jidoka)
        #[arg(long)]
        max_failures: Option<usize>,

        /// FJ-380: Limit concurrent SSH connections
        #[arg(long)]
        rate_limit: Option<usize>,

        /// FJ-383: Add metadata labels to apply run (KEY=VALUE)
        #[arg(long = "label", value_name = "KEY=VALUE")]
        labels: Vec<String>,

        /// FJ-386: Execute a previously saved plan file
        #[arg(long)]
        plan_file: Option<PathBuf>,

        /// FJ-393: Send apply results via email
        #[arg(long)]
        notify_email: Option<String>,

        /// FJ-396: Skip specific resource during apply
        #[arg(long)]
        skip: Option<String>,

        /// FJ-403: Named snapshot before apply
        #[arg(long)]
        snapshot_before: Option<String>,

        /// FJ-406: Global concurrency limit across all machines
        #[arg(long)]
        concurrency: Option<usize>,

        /// FJ-410: POST to webhook before apply starts
        #[arg(long)]
        webhook_before: Option<String>,

        /// FJ-413: Auto-rollback to named snapshot on failure
        #[arg(long)]
        rollback_snapshot: Option<String>,

        /// FJ-420: Delay between retry attempts in seconds
        #[arg(long)]
        retry_delay: Option<u64>,

        /// FJ-423: Apply only resources matching any of the given tags
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,

        /// FJ-426: Write detailed apply log to file
        #[arg(long)]
        log_file: Option<PathBuf>,

        /// FJ-430: Attach a comment to the apply run in event log
        #[arg(long)]
        comment: Option<String>,

        /// FJ-433: Apply only resources whose config hash changed since last apply
        #[arg(long)]
        only_changed: bool,

        /// FJ-436: Run a script before apply starts (pre-flight check)
        #[arg(long)]
        pre_script: Option<PathBuf>,

        /// FJ-440: Output dry-run results as structured JSON
        #[arg(long)]
        dry_run_json: bool,

        /// FJ-443: POST structured results to any webhook URL
        #[arg(long)]
        notify_webhook: Option<String>,

        /// FJ-446: Run a script after apply completes (post-flight)
        #[arg(long)]
        post_script: Option<PathBuf>,

        /// FJ-450: Require explicit approval before destructive changes
        #[arg(long)]
        approval_required: bool,

        /// FJ-453: Apply to N% of machines first, then rest (gradual rollout)
        #[arg(long)]
        canary_percent: Option<u32>,

        /// FJ-456: Schedule apply for later execution (cron expression)
        #[arg(long)]
        schedule: Option<String>,

        /// FJ-460: Apply using named environment config overlay
        #[arg(long, name = "env-name")]
        env_name: Option<String>,

        /// FJ-463: Show unified diff of what would change
        #[arg(long)]
        dry_run_diff: bool,

        /// FJ-466: Send apply events to PagerDuty
        #[arg(long)]
        notify_pagerduty: Option<String>,

        /// FJ-470: Process resources in batches of N (memory-bounded execution)
        #[arg(long)]
        batch_size: Option<usize>,

        /// FJ-473: Send apply results to Microsoft Teams webhook
        #[arg(long)]
        notify_teams: Option<String>,

        /// FJ-476: Abort apply if drift detected before execution
        #[arg(long)]
        abort_on_drift: bool,

        /// FJ-480: Show one-line summary of what would change per machine
        #[arg(long)]
        dry_run_summary: bool,

        /// FJ-483: Send apply results to Discord webhook
        #[arg(long)]
        notify_discord: Option<String>,

        /// FJ-486: Auto-rollback if more than N resources fail
        #[arg(long)]
        rollback_on_threshold: Option<usize>,

        /// FJ-490: Expose apply metrics on HTTP port for Prometheus scraping
        #[arg(long)]
        metrics_port: Option<u16>,

        /// FJ-493: Send apply alerts to OpsGenie
        #[arg(long)]
        notify_opsgenie: Option<String>,

        /// FJ-496: Pause apply after N consecutive failures (circuit breaker)
        #[arg(long)]
        circuit_breaker: Option<usize>,

        /// FJ-500: Require named approvers before apply proceeds
        #[arg(long)]
        require_approval: Option<String>,

        /// FJ-503: Send apply events to Datadog
        #[arg(long)]
        notify_datadog: Option<String>,

        /// FJ-506: Restrict applies to defined maintenance windows (cron expression)
        #[arg(long)]
        change_window: Option<String>,

        /// FJ-510: Apply to single machine first as canary before fleet
        #[arg(long)]
        canary_machine: Option<String>,

        /// FJ-513: Send apply events to New Relic
        #[arg(long)]
        notify_newrelic: Option<String>,

        /// FJ-516: Abort apply if it exceeds time limit (seconds)
        #[arg(long)]
        max_duration: Option<u64>,

        /// FJ-520: Send apply annotations to Grafana
        #[arg(long)]
        notify_grafana: Option<String>,

        /// FJ-523: Apply at most N resources per minute (throttle)
        #[arg(long)]
        rate_limit_resources: Option<usize>,

        /// FJ-526: Save intermediate state during long applies (seconds)
        #[arg(long)]
        checkpoint_interval: Option<u64>,

        /// FJ-530: Send apply events to VictorOps/Splunk On-Call
        #[arg(long)]
        notify_victorops: Option<String>,

        /// FJ-533: Blue/green deployment with machine pairs
        #[arg(long)]
        blue_green: Option<String>,

        /// FJ-536: Show estimated cost without applying
        #[arg(long)]
        dry_run_cost: bool,

        /// FJ-540: Send Adaptive Card to MS Teams
        #[arg(long)]
        notify_msteams_adaptive: Option<String>,

        /// FJ-543: Progressive rollout (apply to N% of machines)
        #[arg(long)]
        progressive: Option<u8>,

        /// FJ-546: POST for approval before applying (GitOps gate)
        #[arg(long)]
        approval_webhook: Option<String>,

        /// FJ-550: POST incident to PagerDuty/Opsgenie with full context
        #[arg(long)]
        notify_incident: Option<String>,

        /// FJ-556: Require named sign-off before apply proceeds
        #[arg(long)]
        sign_off: Option<String>,

        /// FJ-560: Publish apply events to AWS SNS topic
        #[arg(long)]
        notify_sns: Option<String>,

        /// FJ-563: POST OpenTelemetry spans for apply execution
        #[arg(long)]
        telemetry_endpoint: Option<String>,

        /// FJ-566: Attach runbook URL to apply for audit trail
        #[arg(long)]
        runbook: Option<String>,

        /// FJ-570: Publish apply events to Google Cloud Pub/Sub
        #[arg(long)]
        notify_pubsub: Option<String>,

        /// FJ-573: Fleet-wide rollout strategy (parallel, rolling, canary)
        #[arg(long)]
        fleet_strategy: Option<String>,

        /// FJ-576: Run validation script before apply proceeds
        #[arg(long)]
        pre_check: Option<String>,

        /// FJ-580: Publish to AWS EventBridge for event-driven workflows
        #[arg(long)]
        notify_eventbridge: Option<String>,

        /// FJ-583: Show execution graph without applying
        #[arg(long)]
        dry_run_graph: bool,

        /// FJ-586: Run validation script after apply completes
        #[arg(long)]
        post_check: Option<String>,

        /// FJ-590: Publish apply events to Apache Kafka
        #[arg(long)]
        notify_kafka: Option<String>,

        /// FJ-593: Retry failed resources up to N times
        #[arg(long)]
        max_retries: Option<u32>,

        /// FJ-596: Auto-rollback if issues detected within window
        #[arg(long)]
        rollback_window: Option<String>,

        /// FJ-600: Publish apply events to Azure Service Bus
        #[arg(long)]
        notify_azure_servicebus: Option<String>,

        /// FJ-603: Timeout for interactive approval prompts
        #[arg(long)]
        approval_timeout: Option<String>,

        /// FJ-606: Run pre-flight validation script before apply
        #[arg(long)]
        pre_flight: Option<String>,

        /// FJ-610: Enhanced GCP Pub/Sub notification with ordering keys
        #[arg(long)]
        notify_gcp_pubsub_v2: Option<String>,

        /// FJ-613: Create named checkpoint before apply
        #[arg(long)]
        checkpoint: Option<String>,

        /// FJ-616: Run post-flight validation script after apply
        #[arg(long)]
        post_flight: Option<String>,

        /// FJ-620: Publish events to RabbitMQ
        #[arg(long)]
        notify_rabbitmq: Option<String>,

        /// FJ-623: Require named approval gate before apply
        #[arg(long)]
        gate: Option<String>,

        /// FJ-630: Publish events to NATS messaging
        #[arg(long)]
        notify_nats: Option<String>,

        /// FJ-633: Verbose dry-run showing all planned commands
        #[arg(long)]
        dry_run_verbose: bool,

        /// FJ-636: Explain what each step will do before executing
        #[arg(long)]
        explain: bool,

        /// FJ-640: Publish apply events to MQTT broker for IoT integration
        #[arg(long)]
        notify_mqtt: Option<String>,

        /// FJ-643: Custom confirmation message before apply
        #[arg(long)]
        confirmation_message: Option<String>,

        /// FJ-646: Only show summary, no per-resource output
        #[arg(long)]
        summary_only: bool,

        /// FJ-650: Publish events to Redis pub/sub
        #[arg(long)]
        notify_redis: Option<String>,

        /// FJ-660: Publish events to AMQP exchange
        #[arg(long)]
        notify_amqp: Option<String>,

        /// FJ-663: Run command before each resource apply
        #[arg(long)]
        pre_apply_hook: Option<String>,

        /// FJ-666: Only apply resources matching glob pattern
        #[arg(long)]
        resource_filter: Option<String>,

        /// FJ-670: Publish events to STOMP destination
        #[arg(long)]
        notify_stomp: Option<String>,

        /// FJ-673: Run command after each resource apply
        #[arg(long)]
        post_apply_hook: Option<String>,

        /// FJ-676: Output shell scripts instead of executing
        #[arg(long)]
        dry_run_shell: bool,

        /// FJ-680: Publish events to ZeroMQ socket
        #[arg(long)]
        notify_zeromq: Option<String>,

        /// FJ-683: Apply single resource first as canary
        #[arg(long)]
        canary_resource: Option<String>,

        /// FJ-686: Per-resource timeout override in seconds
        #[arg(long)]
        timeout_per_resource: Option<u64>,
        /// FJ-690: Publish events to gRPC endpoint
        #[arg(long)]
        notify_grpc: Option<String>,
        /// FJ-693: Skip resources whose hash hasn't changed
        #[arg(long)]
        skip_unchanged: bool,
        /// FJ-696: Exponential backoff factor for retries
        #[arg(long)]
        retry_backoff: Option<f64>,
        /// FJ-700: Publish events to AWS SQS queue
        #[arg(long)]
        notify_sqs: Option<String>,
        /// FJ-703: Save plan output to file
        #[arg(long)]
        plan_output_file: Option<String>,
        /// FJ-706: Set execution priority for specific resources (name=priority)
        #[arg(long)]
        resource_priority: Vec<String>,
        /// FJ-713: Time window for apply operations in seconds
        #[arg(long)]
        apply_window: Option<u64>,
        /// FJ-716: Stop all machines on first machine failure
        #[arg(long)]
        fail_fast_machine: bool,
        /// FJ-720: Publish events to Mattermost webhook
        #[arg(long)]
        notify_mattermost: Option<String>,
        /// FJ-723: Cooldown between resource applies in seconds
        #[arg(long)]
        cooldown: Option<u64>,
        /// FJ-726: Exclude specific machine from apply
        #[arg(long)]
        exclude_machine: Option<String>,
    },

    /// Detect unauthorized changes (tripwire)
    Drift {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Target specific machine
        #[arg(short, long)]
        machine: Option<String>,

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Exit non-zero on any drift (for CI/cron)
        #[arg(long)]
        tripwire: bool,

        /// Run command on drift detection
        #[arg(long)]
        alert_cmd: Option<String>,

        /// Auto-remediate: re-apply drifted resources to restore desired state
        #[arg(long)]
        auto_remediate: bool,

        /// Show what would be checked without connecting to machines
        #[arg(long)]
        dry_run: bool,

        /// Output drift report as JSON
        #[arg(long)]
        json: bool,

        /// FJ-211: Load param overrides from external YAML file
        #[arg(long)]
        env_file: Option<PathBuf>,

        /// FJ-210: Use workspace (overrides state dir to state/<workspace>/)
        #[arg(short = 'w', long)]
        workspace: Option<String>,
    },

    /// Show current state from lock files
    Status {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Target specific machine
        #[arg(short, long)]
        machine: Option<String>,

        /// Output status as JSON
        #[arg(long)]
        json: bool,

        /// Config file — enriches JSON with resource_group, tags, depends_on
        #[arg(short, long)]
        file: Option<PathBuf>,

        /// One-line summary for dashboards
        #[arg(long)]
        summary: bool,

        /// FJ-314: Watch mode — refresh every N seconds
        #[arg(long)]
        watch: Option<u64>,

        /// FJ-336: Show only resources not updated in N days
        #[arg(long)]
        stale: Option<u64>,

        /// FJ-346: Show aggregate health score (0-100)
        #[arg(long)]
        health: bool,

        /// FJ-355: Show detailed drift report with field-level diffs
        #[arg(long)]
        drift_details: bool,

        /// FJ-364: Show convergence timeline with timestamps
        #[arg(long)]
        timeline: bool,

        /// FJ-372: Show resources changed since a git commit
        #[arg(long)]
        changes_since: Option<String>,

        /// FJ-376: Group output by dimension (machine, type, or status)
        #[arg(long)]
        summary_by: Option<String>,

        /// FJ-382: Expose metrics in Prometheus exposition format
        #[arg(long)]
        prometheus: bool,

        /// FJ-387: Show resources whose lock is older than duration (e.g., 7d, 24h)
        #[arg(long)]
        expired: Option<String>,

        /// FJ-392: Show resource count by status (converged/failed/drifted)
        #[arg(long)]
        count: bool,

        /// FJ-397: Output format: table (default), json, csv
        #[arg(long)]
        format: Option<String>,

        /// FJ-402: Detect anomalous resource states from historical patterns
        #[arg(long)]
        anomalies: bool,

        /// FJ-407: Diff current state against a named snapshot
        #[arg(long)]
        diff_from: Option<String>,

        /// FJ-412: Group status output by resource type
        #[arg(long)]
        resources_by_type: bool,

        /// FJ-417: Show only machine-level summary (no resource details)
        #[arg(long)]
        machines_only: bool,

        /// FJ-422: Show resources not updated in any recent apply
        #[arg(long)]
        stale_resources: bool,

        /// FJ-427: Custom health score threshold (default: 80)
        #[arg(long)]
        health_threshold: Option<u32>,

        /// FJ-432: Output status as newline-delimited JSON (NDJSON)
        #[arg(long)]
        json_lines: bool,

        /// FJ-437: Show only resources changed within duration (e.g., 1h, 7d)
        #[arg(long)]
        since: Option<String>,

        /// FJ-442: Export status report to file (JSON/CSV/YAML)
        #[arg(long)]
        export: Option<PathBuf>,

        /// FJ-452: Minimal one-line-per-machine output for large fleets
        #[arg(long)]
        compact: bool,

        /// FJ-457: Show resources in alert state (failed, drifted, or stale)
        #[arg(long)]
        alerts: bool,

        /// FJ-462: Diff current lock against a saved lock snapshot
        #[arg(long)]
        diff_lock: Option<PathBuf>,

        /// FJ-467: Check compliance against named policy
        #[arg(long)]
        compliance: Option<String>,

        /// FJ-472: Show resource status distribution as ASCII histogram
        #[arg(long)]
        histogram: bool,

        /// FJ-477: Show health score weighted by dependency position
        #[arg(long)]
        dependency_health: bool,

        /// FJ-482: Show most frequently failing resources
        #[arg(long)]
        top_failures: bool,

        /// FJ-487: Show convergence percentage over time
        #[arg(long)]
        convergence_rate: bool,

        /// FJ-492: One-line per-machine drift count and percentage
        #[arg(long)]
        drift_summary: bool,

        /// FJ-497: Show age of each resource since last successful apply
        #[arg(long)]
        resource_age: bool,

        /// FJ-502: Show SLA compliance based on convergence timing
        #[arg(long)]
        sla_report: bool,

        /// FJ-507: Generate full compliance report for named policy
        #[arg(long)]
        compliance_report: Option<String>,

        /// FJ-512: Show mean time to recovery per resource
        #[arg(long)]
        mttr: bool,

        /// FJ-517: Show status trend over last N applies
        #[arg(long)]
        trend: Option<usize>,

        /// FJ-522: Predict next failure based on historical patterns
        #[arg(long)]
        prediction: bool,

        /// FJ-527: Show resource utilization vs limits per machine
        #[arg(long)]
        capacity: bool,

        /// FJ-532: Estimate resource cost based on type counts
        #[arg(long)]
        cost_estimate: bool,

        /// FJ-537: Show resources not applied within configurable window
        #[arg(long)]
        staleness_report: Option<String>,

        /// FJ-542: Composite health score (0-100) across all machines
        #[arg(long)]
        health_score: bool,

        /// FJ-547: One-line per machine summary for dashboards
        #[arg(long)]
        executive_summary: bool,

        /// FJ-552: Full audit trail with who/what/when for each change
        #[arg(long)]
        audit_trail: bool,

        /// FJ-562: Show resource dependency graph from live state
        #[arg(long)]
        resource_graph: bool,

        /// FJ-567: Show drift rate over time (changes per day/week)
        #[arg(long)]
        drift_velocity: bool,

        /// FJ-572: Aggregated fleet summary across all machines
        #[arg(long)]
        fleet_overview: bool,

        /// FJ-577: Per-machine health details with resource breakdown
        #[arg(long)]
        machine_health: bool,

        /// FJ-582: Compare running config against declared config
        #[arg(long)]
        config_drift: bool,

        /// FJ-587: Show average time to convergence per resource
        #[arg(long)]
        convergence_time: bool,

        /// FJ-592: Show per-resource status changes over time
        #[arg(long)]
        resource_timeline: bool,

        /// FJ-597: Aggregated error summary across all machines
        #[arg(long)]
        error_summary: bool,

        /// FJ-602: Show security-relevant resource states
        #[arg(long)]
        security_posture: bool,

        /// FJ-612: Estimate resource cost based on type and count
        #[arg(long)]
        resource_cost: bool,

        /// FJ-617: Predict likely drift based on historical patterns
        #[arg(long)]
        drift_forecast: bool,

        /// FJ-622: Show CI/CD pipeline integration status
        #[arg(long)]
        pipeline_status: bool,

        /// FJ-627: Show runtime dependency graph from lock files
        #[arg(long)]
        resource_dependencies: bool,

        /// FJ-632: Comprehensive diagnostic report with recommendations
        #[arg(long)]
        diagnostic: bool,

        /// FJ-642: Show resource uptime based on convergence history
        #[arg(long)]
        uptime: bool,

        /// FJ-647: AI-powered recommendations based on state analysis
        #[arg(long)]
        recommendations: bool,

        /// FJ-657: Per-machine resource count and health summary
        #[arg(long)]
        machine_summary: bool,

        /// FJ-662: Show how often each resource changes
        #[arg(long)]
        change_frequency: bool,

        /// FJ-667: Show age of each lock file entry
        #[arg(long)]
        lock_age: bool,

        /// FJ-672: Show resources failed since a given timestamp
        #[arg(long)]
        failed_since: Option<String>,

        /// FJ-677: Verify BLAKE3 hashes in lock match computed hashes
        #[arg(long)]
        hash_verify: bool,

        /// FJ-682: Show estimated resource sizes
        #[arg(long)]
        resource_size: bool,

        /// FJ-687: Show drift details for all machines at once
        #[arg(long)]
        drift_details_all: bool,
        /// FJ-692: Show duration of last apply per resource
        #[arg(long)]
        last_apply_duration: bool,
        /// FJ-697: Show hash of current config for change detection
        #[arg(long)]
        config_hash: bool,
        /// FJ-707: Show convergence trend over time
        #[arg(long)]
        convergence_history: bool,
        /// FJ-712: Show resource input fields per resource
        #[arg(long)]
        resource_inputs: bool,
        /// FJ-717: Show drift trend over time
        #[arg(long)]
        drift_trend: bool,
        /// FJ-722: Show only failed resources across machines
        #[arg(long)]
        failed_resources: bool,
        /// FJ-727: Show count per resource type
        #[arg(long)]
        resource_types_summary: bool,
    },

    /// Show apply history from event logs
    History {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Show history for specific machine
        #[arg(short, long)]
        machine: Option<String>,

        /// Show last N applies (default: 10)
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// FJ-284: Show only events from the last duration (e.g., 24h, 7d, 30m)
        #[arg(long)]
        since: Option<String>,

        /// FJ-357: Show change history for a specific resource
        #[arg(long)]
        resource: Option<String>,
    },

    /// Remove all managed resources (reverse order)
    Destroy {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Target specific machine
        #[arg(short, long)]
        machine: Option<String>,

        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,
    },

    /// Import existing infrastructure from a machine into forjar.yaml
    Import {
        /// Machine address (IP, hostname, or 'localhost')
        #[arg(short, long)]
        addr: String,

        /// SSH user
        #[arg(short, long, default_value = "root")]
        user: String,

        /// Machine name (used as key in machines section)
        #[arg(short, long)]
        name: Option<String>,

        /// Output file
        #[arg(short, long, default_value = "forjar.yaml")]
        output: PathBuf,

        /// What to scan
        #[arg(long, value_delimiter = ',', default_value = "packages,files,services")]
        scan: Vec<String>,
    },

    /// Show fully resolved config (recipes expanded, templates resolved)
    Show {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Show specific resource only
        #[arg(short, long)]
        resource: Option<String>,

        /// Output as JSON instead of YAML
        #[arg(long)]
        json: bool,
    },

    /// Show resource dependency graph
    Graph {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Output format: mermaid (default) or dot
        #[arg(long, default_value = "mermaid")]
        format: String,

        /// Filter to specific machine
        #[arg(short, long)]
        machine: Option<String>,

        /// Filter to specific resource group
        #[arg(short, long)]
        group: Option<String>,

        /// FJ-354: Show transitive dependents of a resource (impact analysis)
        #[arg(long)]
        affected: Option<String>,

        /// FJ-375: Highlight the longest dependency chain
        #[arg(long)]
        critical_path: bool,

        /// FJ-385: Show reverse dependency graph
        #[arg(long)]
        reverse: bool,

        /// FJ-394: Limit graph traversal depth
        #[arg(long)]
        depth: Option<usize>,

        /// FJ-404: Group resources by machine in graph output
        #[arg(long)]
        cluster: bool,

        /// FJ-414: Show resources with no dependencies and no dependents
        #[arg(long)]
        orphans: bool,

        /// FJ-424: Show graph statistics (nodes, edges, depth, width)
        #[arg(long)]
        stats: bool,

        /// FJ-434: Output graph as JSON adjacency list
        #[arg(long, name = "json")]
        json_output: bool,

        /// FJ-444: Highlight a resource and its transitive deps in graph output
        #[arg(long)]
        highlight: Option<String>,

        /// FJ-454: Show graph with a resource and its subtree removed
        #[arg(long)]
        prune: Option<String>,

        /// FJ-464: Show graph organized by dependency layers (depth levels)
        #[arg(long)]
        layers: bool,

        /// FJ-474: Identify resources with the most dependents (bottleneck analysis)
        #[arg(long)]
        critical_resources: bool,

        /// FJ-484: Show edge weights based on dependency strength
        #[arg(long)]
        weight: bool,

        /// FJ-494: Extract and display a resource's dependency subgraph
        #[arg(long)]
        subgraph: Option<String>,

        /// FJ-504: Show blast radius of changing a resource
        #[arg(long)]
        impact_radius: Option<String>,

        /// FJ-514: Output resource dependency matrix (CSV/JSON)
        #[arg(long)]
        dependency_matrix: bool,

        /// FJ-524: Highlight resources with most changes/failures (heat map)
        #[arg(long)]
        hotspots: bool,

        /// FJ-534: Show resource application order as ASCII timeline
        #[arg(long)]
        timeline_graph: bool,

        /// FJ-544: Simulate removing a resource, show impact
        #[arg(long)]
        what_if: Option<String>,

        /// FJ-554: Show all resources affected by a change to target
        #[arg(long)]
        blast_radius: Option<String>,

        /// FJ-564: Show direct + indirect impact of changing a resource
        #[arg(long)]
        change_impact: Option<String>,

        /// FJ-574: Show graph colored/grouped by resource type
        #[arg(long)]
        resource_types: bool,

        /// FJ-584: Show resources grouped by topological depth level
        #[arg(long)]
        topological_levels: bool,

        /// FJ-594: Show exact execution order with timing estimates
        #[arg(long)]
        execution_order: bool,

        /// FJ-604: Highlight resources crossing security boundaries
        #[arg(long)]
        security_boundaries: bool,

        /// FJ-614: Show resource age based on last apply timestamp
        #[arg(long)]
        resource_age: bool,

        /// FJ-624: Show which resources can execute in parallel
        #[arg(long)]
        parallel_groups: bool,

        /// FJ-634: Show longest dependency chain (critical path analysis)
        #[arg(long)]
        critical_chain: bool,

        /// FJ-644: Show max dependency depth per resource
        #[arg(long)]
        dependency_depth: bool,

        /// FJ-654: Find resources with no dependents or dependencies
        #[arg(long)]
        orphan_detection: bool,

        /// FJ-664: Visualize dependencies across machines
        #[arg(long)]
        cross_machine_deps: bool,

        /// FJ-674: Group resources by machine in graph output
        #[arg(long)]
        machine_groups: bool,

        /// FJ-684: Identify tightly-coupled resource clusters
        #[arg(long)]
        resource_clusters: bool,
        /// FJ-694: Show resource fan-out metrics
        #[arg(long)]
        fan_out: bool,
        /// FJ-704: Show leaf resources (no dependents)
        #[arg(long)]
        leaf_resources: bool,
        /// FJ-714: Show reverse dependency graph
        #[arg(long)]
        reverse_deps: bool,
        /// FJ-724: Show depth-first traversal order
        #[arg(long)]
        depth_first: bool,
    },

    /// Run check scripts to verify pre-conditions without applying
    Check {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Target specific machine
        #[arg(short, long)]
        machine: Option<String>,

        /// Target specific resource
        #[arg(short, long)]
        resource: Option<String>,

        /// Filter to resources with this tag
        #[arg(long)]
        tag: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Compare two state snapshots (show what changed between applies)
    Diff {
        /// First state directory (older)
        from: PathBuf,

        /// Second state directory (newer)
        to: PathBuf,

        /// Filter to specific machine
        #[arg(short, long)]
        machine: Option<String>,

        /// FJ-291: Filter to specific resource
        #[arg(short, long)]
        resource: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Format (normalize) a forjar.yaml config file
    Fmt {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Check formatting without writing (exit non-zero if unformatted)
        #[arg(long)]
        check: bool,
    },

    /// Lint config for best practices (beyond validation)
    Lint {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// FJ-221: Enable built-in policy rules (no_root_owner, require_tags, etc.)
        #[arg(long)]
        strict: bool,

        /// FJ-332: Auto-fix common lint issues (normalize quotes, sort keys)
        #[arg(long)]
        fix: bool,

        /// FJ-374: Custom lint rules from YAML file
        #[arg(long)]
        rules: Option<PathBuf>,
    },

    /// Rollback to a previous config revision from git history
    Rollback {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Git revision to rollback to (default: HEAD~1)
        #[arg(short = 'n', long, default_value = "1")]
        revision: u32,

        /// Target specific machine
        #[arg(short, long)]
        machine: Option<String>,

        /// Show what would change without applying
        #[arg(long)]
        dry_run: bool,

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,
    },

    /// Detect anomalous resource behavior from event history
    Anomaly {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Target specific machine
        #[arg(short, long)]
        machine: Option<String>,

        /// Minimum events to consider (ignore resources with fewer)
        #[arg(long, default_value = "3")]
        min_events: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// View trace provenance data from apply runs (FJ-050)
    Trace {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Target specific machine
        #[arg(short, long)]
        machine: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Migrate Docker resources to pepita kernel isolation (FJ-044)
    Migrate {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Write migrated config to file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Start MCP server (pforge integration, FJ-063)
    Mcp {
        /// Export tool schemas as JSON instead of starting server
        #[arg(long)]
        schema: bool,
    },

    /// Run performance benchmarks (spec §9 targets)
    Bench {
        /// Number of iterations per benchmark (default: 1000)
        #[arg(long, default_value = "1000")]
        iterations: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// List all resources in state with type, status, hash prefix (FJ-214)
    #[command(name = "state-list")]
    StateList {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Filter to specific machine
        #[arg(short, long)]
        machine: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Rename a resource in state without re-applying (FJ-212)
    #[command(name = "state-mv")]
    StateMv {
        /// Current resource ID
        old_id: String,

        /// New resource ID
        new_id: String,

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Target machine (required if multiple machines have this resource)
        #[arg(short, long)]
        machine: Option<String>,
    },

    /// Remove a resource from state without destroying it on the machine (FJ-213)
    #[command(name = "state-rm")]
    StateRm {
        /// Resource ID to remove
        resource_id: String,

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Target machine (required if multiple machines have this resource)
        #[arg(short, long)]
        machine: Option<String>,

        /// Skip dependency check and force removal
        #[arg(long)]
        force: bool,
    },

    /// Show computed output values from forjar.yaml (FJ-215)
    Output {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Specific output key to show (omit for all)
        key: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-220: Evaluate policy rules against config
    #[command(name = "policy")]
    Policy {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-210: Manage workspaces (isolated state directories)
    #[command(subcommand)]
    Workspace(WorkspaceCmd),

    /// FJ-200: Manage age-encrypted secrets
    #[command(subcommand)]
    Secrets(SecretsCmd),

    /// FJ-251: Pre-flight system checker
    Doctor {
        /// Path to forjar.yaml (optional — checks system basics without it)
        #[arg(short, long)]
        file: Option<PathBuf>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// FJ-287: Auto-fix common issues (create state dir, remove stale locks)
        #[arg(long)]
        fix: bool,

        /// FJ-343: Test SSH connectivity to all machines
        #[arg(long)]
        network: bool,
    },

    /// FJ-253: Generate shell completions
    Completion {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: CompletionShell,
    },

    /// FJ-264: Export JSON Schema for forjar.yaml
    Schema,

    /// FJ-267: Watch config for changes and auto-plan
    Watch {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Polling interval in seconds
        #[arg(long, default_value = "2")]
        interval: u64,

        /// Auto-apply on change (requires --yes)
        #[arg(long)]
        apply: bool,

        /// Confirm auto-apply (required with --apply)
        #[arg(long)]
        yes: bool,
    },

    /// FJ-271: Show full resolution chain for a resource
    Explain {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Resource ID to explain
        resource: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-277: Show resolved environment info
    Env {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-273: Run check scripts for all resources, report pass/fail table
    Test {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Target specific machine
        #[arg(short, long)]
        machine: Option<String>,

        /// Target specific resource
        #[arg(short, long)]
        resource: Option<String>,

        /// Filter to resources with this tag
        #[arg(short, long)]
        tag: Option<String>,

        /// FJ-281: Filter to resources in this group
        #[arg(short, long)]
        group: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-256: Generate lock file without applying
    Lock {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// FJ-211: Load param overrides from external YAML file
        #[arg(long)]
        env_file: Option<PathBuf>,

        /// FJ-210: Use workspace (overrides state dir to state/<workspace>/)
        #[arg(short = 'w', long)]
        workspace: Option<String>,

        /// Verify existing lock matches config (exit 1 on mismatch)
        #[arg(long)]
        verify: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-260: Manage state snapshots
    #[command(subcommand)]
    Snapshot(SnapshotCmd),

    /// FJ-326: List all machines with connection status
    Inventory {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-327: Re-run only previously failed resources
    RetryFailed {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Override a parameter (KEY=VALUE)
        #[arg(long = "param", value_name = "KEY=VALUE")]
        params: Vec<String>,

        /// Timeout per transport operation (seconds)
        #[arg(long)]
        timeout: Option<u64>,
    },

    /// FJ-324: Rolling deployment — apply N machines at a time
    Rolling {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Number of machines to apply concurrently
        #[arg(long, default_value = "1")]
        batch_size: usize,

        /// Override a parameter (KEY=VALUE)
        #[arg(long = "param", value_name = "KEY=VALUE")]
        params: Vec<String>,

        /// Timeout per transport operation (seconds)
        #[arg(long)]
        timeout: Option<u64>,
    },

    /// FJ-325: Canary deployment — apply to one machine first, confirm, then rest
    Canary {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Machine to use as canary (apply first)
        #[arg(short, long)]
        machine: String,

        /// Auto-proceed after canary success (skip confirmation)
        #[arg(long)]
        auto_proceed: bool,

        /// Override a parameter (KEY=VALUE)
        #[arg(long = "param", value_name = "KEY=VALUE")]
        params: Vec<String>,

        /// Timeout per transport operation (seconds)
        #[arg(long)]
        timeout: Option<u64>,
    },

    /// FJ-341: Show full audit trail — who applied what, when, from which config
    Audit {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Target specific machine
        #[arg(short, long)]
        machine: Option<String>,

        /// Show last N entries (default: 20)
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-344: One-line-per-resource plan output for large configs
    #[command(name = "plan-compact")]
    PlanCompact {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Target specific machine
        #[arg(short, long)]
        machine: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-351: Validate infrastructure against policy rules
    Compliance {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-352: Export state to external formats (terraform, ansible, csv)
    Export {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Output format: terraform, ansible, csv
        #[arg(long, default_value = "csv")]
        format: String,

        /// Target specific machine
        #[arg(short, long)]
        machine: Option<String>,

        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// FJ-361: Analyze config and suggest improvements
    Suggest {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-363: Compare two config files and show differences
    Compare {
        /// First config file
        file1: PathBuf,

        /// Second config file
        file2: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-366: Remove lock entries for resources no longer in config
    #[command(name = "lock-prune")]
    LockPrune {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Actually remove entries (default: dry-run)
        #[arg(long)]
        yes: bool,
    },

    /// FJ-367: Compare environments (workspaces) for drift
    #[command(name = "env-diff")]
    EnvDiff {
        /// First workspace name
        env1: String,

        /// Second workspace name
        env2: String,

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// FJ-371: Expand a recipe template to stdout without applying
    Template {
        /// Path to recipe YAML file
        recipe: PathBuf,

        /// Variable overrides (KEY=VALUE)
        #[arg(short, long = "var", value_name = "KEY=VALUE")]
        vars: Vec<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-384: Show lock file metadata
    #[command(name = "lock-info")]
    LockInfo {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-395: Compact lock file — remove historical entries, keep latest per resource
    #[command(name = "lock-compact")]
    LockCompact {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Actually compact (default: dry-run showing what would be removed)
        #[arg(long)]
        yes: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// FJ-425: Garbage collect orphaned lock entries with no matching config
    #[command(name = "lock-gc")]
    LockGc {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Actually remove entries (default: dry-run)
        #[arg(long)]
        yes: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-415: Export lock file in alternative format
    #[command(name = "lock-export")]
    LockExport {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Output format: json, yaml, csv
        #[arg(long, default_value = "json")]
        format: String,

        /// Target specific machine
        #[arg(short, long)]
        machine: Option<String>,
    },

    /// FJ-405: Verify lock file integrity (BLAKE3 checksums)
    #[command(name = "lock-verify")]
    LockVerify {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-435: Compare two lock files and show resource-level differences
    LockDiff {
        /// First state directory (older)
        from: PathBuf,

        /// Second state directory (newer)
        to: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-445: Merge two lock files (multi-team workflow)
    #[command(name = "lock-merge")]
    LockMerge {
        /// First state directory
        from: PathBuf,

        /// Second state directory (takes precedence on conflicts)
        to: PathBuf,

        /// Output directory for merged state
        #[arg(long, default_value = "state")]
        output: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-455: Rebase lock file from one config version to another
    #[command(name = "lock-rebase")]
    LockRebase {
        /// Source state directory
        from: PathBuf,

        /// Target config file
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Output state directory
        #[arg(long, default_value = "state")]
        output: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-465: Cryptographically sign lock file with BLAKE3
    #[command(name = "lock-sign")]
    LockSign {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Signing key (path to key file or inline)
        #[arg(long)]
        key: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-475: Verify lock file signature against signing key
    #[command(name = "lock-verify-sig")]
    LockVerifySig {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Signing key to verify against
        #[arg(long)]
        key: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-485: Compact all machine lock files in one operation
    #[command(name = "lock-compact-all")]
    LockCompactAll {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Skip confirmation
        #[arg(long)]
        yes: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-495: Show full audit trail of lock file changes with timestamps
    #[command(name = "lock-audit-trail")]
    LockAuditTrail {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Target specific machine
        #[arg(short, long)]
        machine: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-505: Rotate all lock file signing keys
    #[command(name = "lock-rotate-keys")]
    LockRotateKeys {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Old signing key
        #[arg(long)]
        old_key: String,

        /// New signing key
        #[arg(long)]
        new_key: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-515: Create timestamped backup of all lock files
    #[command(name = "lock-backup")]
    LockBackup {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-535: Verify full chain of custody from lock signatures
    #[command(name = "lock-verify-chain")]
    LockVerifyChain {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-545: Show lock file statistics (sizes, ages, resource counts)
    #[command(name = "lock-stats")]
    LockStats {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-555: Verify lock file integrity and show tampering evidence
    LockAudit {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-565: Compress old lock files with zstd
    LockCompress {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-575: Defragment lock files (reorder resources alphabetically)
    LockDefrag {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-585: Normalize lock file format (consistent key ordering, whitespace)
    LockNormalize {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-595: Validate lock file schema and cross-references
    LockValidate {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-605: Verify lock file HMAC signatures
    #[command(name = "lock-verify-hmac")]
    LockVerifyHmac {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-615: Archive old lock files to compressed storage
    #[command(name = "lock-archive")]
    LockArchive {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-625: Create point-in-time lock file snapshot with metadata
    #[command(name = "lock-snapshot")]
    LockSnapshot {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-635: Attempt automatic repair of corrupted lock files
    #[command(name = "lock-repair")]
    LockRepair {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-645: Show lock file change history with diffs
    #[command(name = "lock-history")]
    LockHistory {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Maximum entries to show
        #[arg(long, default_value = "20")]
        limit: usize,
    },

    /// FJ-675: Check lock file structural integrity
    #[command(name = "lock-integrity")]
    LockIntegrity {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-685: Rehash all lock file entries with current BLAKE3
    #[command(name = "lock-rehash")]
    LockRehash {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-695: Restore lock state from a named snapshot
    #[command(name = "lock-restore")]
    LockRestore {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Snapshot name to restore from
        #[arg(long)]
        name: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-705: Verify lock file schema version compatibility
    #[command(name = "lock-verify-schema")]
    LockVerifySchema {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-715: Add metadata tags to lock files
    #[command(name = "lock-tag")]
    LockTag {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Tag name
        #[arg(long)]
        name: String,

        /// Tag value
        #[arg(long)]
        value: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// FJ-725: Migrate lock file schema between versions
    #[command(name = "lock-migrate")]
    LockMigrate {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Source schema version
        #[arg(long)]
        from_version: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}


/// FJ-260: Snapshot subcommands — named state checkpoints.
#[derive(Subcommand, Debug)]
pub enum SnapshotCmd {
    /// Save current state as a named snapshot
    Save {
        /// Snapshot name
        name: String,

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,
    },

    /// List available snapshots
    List {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Restore state from a named snapshot
    Restore {
        /// Snapshot name to restore
        name: String,

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Skip confirmation
        #[arg(long)]
        yes: bool,
    },

    /// Delete a named snapshot
    Delete {
        /// Snapshot name to delete
        name: String,

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,
    },
}


/// Shell types for completion generation.
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum CompletionShell {
    Bash,
    Zsh,
    Fish,
}


/// FJ-210: Workspace subcommands.
#[derive(Subcommand, Debug)]
pub enum WorkspaceCmd {
    /// Create a new workspace
    New {
        /// Workspace name
        name: String,
    },

    /// List all workspaces
    List,

    /// Select (activate) a workspace
    Select {
        /// Workspace name to activate
        name: String,
    },

    /// Delete a workspace and its state
    Delete {
        /// Workspace name to delete
        name: String,

        /// Skip confirmation
        #[arg(long)]
        yes: bool,
    },

    /// Show current active workspace
    Current,
}


/// FJ-200: Secrets subcommands — age-encrypted secret management.
#[derive(Subcommand, Debug)]
pub enum SecretsCmd {
    /// Encrypt a value with age recipients
    Encrypt {
        /// Plaintext value to encrypt
        value: String,

        /// Age recipient public key (age1...)
        #[arg(short, long, required = true)]
        recipient: Vec<String>,
    },

    /// Decrypt an ENC[age,...] marker
    Decrypt {
        /// Encrypted marker (ENC[age,...])
        value: String,

        /// Path to age identity file
        #[arg(short, long)]
        identity: Option<PathBuf>,
    },

    /// Generate a new age identity (keypair)
    Keygen,

    /// Decrypt and display all secrets in a forjar.yaml
    View {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Path to age identity file
        #[arg(short, long)]
        identity: Option<PathBuf>,
    },

    /// Re-encrypt all ENC[age,...] markers with new recipients
    Rekey {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Path to current age identity file (for decryption)
        #[arg(short, long)]
        identity: Option<PathBuf>,

        /// New recipient public keys (age1...)
        #[arg(short, long, required = true)]
        recipient: Vec<String>,
    },

    /// FJ-201: Rotate all secrets — decrypt and re-encrypt with new keys
    Rotate {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Path to current age identity file (for decryption)
        #[arg(short, long)]
        identity: Option<PathBuf>,

        /// New recipient public keys (age1...)
        #[arg(short, long, required = true)]
        recipient: Vec<String>,

        /// Re-encrypt in place (required flag to prevent accidents)
        #[arg(long)]
        re_encrypt: bool,

        /// State directory for audit logging
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,
    },
}

