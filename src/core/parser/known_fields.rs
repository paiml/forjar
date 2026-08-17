//! FJ-2500: Known YAML field sets for unknown-field detection.
//!
//! Each constant lists the valid keys for one struct (using serde rename
//! where applicable). Consumed by `unknown_fields` to flag typos with
//! "did you mean?" suggestions.

pub(crate) const CONFIG_FIELDS: &[&str] = &[
    "version",
    "name",
    "description",
    "params",
    "machines",
    "resources",
    "policy",
    "outputs",
    "policies",
    "data",
    "includes",
    "checks",
    "moved",
    "secrets",
    "environments",
    "dist",
];

pub(crate) const RESOURCE_FIELDS: &[&str] = &[
    "type",
    "machine",
    "state",
    "depends_on",
    "provider",
    "packages",
    "version",
    "path",
    "content",
    "source",
    "target",
    "owner",
    "group",
    "mode",
    "name",
    "enabled",
    "restart_on",
    "triggers",
    "fstype",
    "options",
    "uid",
    "shell",
    "home",
    "groups",
    "ssh_authorized_keys",
    "system_user",
    "schedule",
    "command",
    "image",
    "ports",
    "environment",
    "volumes",
    "restart",
    "protocol",
    "port",
    "action",
    "from",
    "recipe",
    "inputs",
    "arch",
    "tags",
    "resource_group",
    "when",
    "count",
    "for_each",
    "chroot_dir",
    "namespace_uid",
    "namespace_gid",
    "seccomp",
    "netns",
    "cpuset",
    "memory_limit",
    "overlay_lower",
    "overlay_upper",
    "overlay_work",
    "overlay_merged",
    "format",
    "quantization",
    "checksum",
    "cache_dir",
    "gpu_backend",
    "driver_version",
    "cuda_version",
    "rocm_version",
    "devices",
    "persistence_mode",
    "compute_mode",
    "gpu_memory_limit_mb",
    "task_mode",
    "task_inputs",
    "output_artifacts",
    "output_equivalence",
    "phony",
    "completion_check",
    "timeout",
    "working_dir",
    "stages",
    "cache",
    "gpu_device",
    "restart_delay",
    "quality_gate",
    "health_check",
    "restart_policy",
    "pre_apply",
    "post_apply",
    "lifecycle",
    "sudo",
    "store",
    "script",
    "gather",
    "scatter",
    "repo",
    "tag",
    "asset_pattern",
    "binary",
    "install_dir",
    "build_machine",
    "overlay_ip",
    "overlay_iface",
    "overlay_hosts",
    "overlay_firewall",
    // FJ-036: disk_budget
    "budget_high_watermark_pct",
    "budget_target_free_pct",
    "budget_critical_free_gb",
    "budget_schedule",
    "budget_reclaim",
    // FJ-037: backup_sync
    "backup_source",
    "backup_remote",
    "backup_schedule",
    "backup_verify_pct",
    "backup_daily_cap_gb",
    "backup_bandwidth_limit",
    "backup_remote_type",
    "backup_remote_config",
    "backup_token",
];

pub(crate) const MACHINE_FIELDS: &[&str] = &[
    "hostname",
    "addr",
    "user",
    "arch",
    "ssh_key",
    "roles",
    "transport",
    "container",
    "pepita",
    "cost",
    "allowed_operators",
];

pub(crate) const POLICY_FIELDS: &[&str] = &[
    "failure",
    "parallel_machines",
    "tripwire",
    "lock_file",
    "parallel_resources",
    "pre_apply",
    "post_apply",
    "serial",
    "max_fail_percentage",
    "ssh_retries",
    "convergence_budget",
    "snapshot_generations",
    "security_gate",
    "deny_paths",
    "notify",
    "logs",
];

pub(crate) const NOTIFY_FIELDS: &[&str] = &["on_success", "on_failure", "on_drift"];

pub(crate) const CONTAINER_FIELDS: &[&str] = &[
    "runtime",
    "image",
    "name",
    "ephemeral",
    "privileged",
    "init",
    "gpus",
    "devices",
    "group_add",
    "env",
    "volumes",
];

pub(crate) const PEPITA_FIELDS: &[&str] = &[
    "rootfs",
    "memory_mb",
    "cpus",
    "network",
    "filesystem",
    "ephemeral",
];

pub(crate) const DATASOURCE_FIELDS: &[&str] = &[
    "type",
    "value",
    "default",
    "state_dir",
    "config",
    "outputs",
    "max_staleness",
];

pub(crate) const POLICY_RULE_FIELDS: &[&str] = &[
    "type",
    "message",
    "resource_type",
    "tag",
    "field",
    "condition_field",
    "condition_value",
    "id",
    "severity",
    "remediation",
    "compliance",
    "max_count",
    "min_count",
];

pub(crate) const OUTPUT_FIELDS: &[&str] = &["value", "description"];

pub(crate) const CHECK_FIELDS: &[&str] = &["machine", "command", "expect_exit", "description"];

pub(crate) const MOVED_FIELDS: &[&str] = &["from", "to"];

pub(crate) const LIFECYCLE_FIELDS: &[&str] =
    &["prevent_destroy", "create_before_destroy", "ignore_drift"];

// -- Recipe known fields --

pub(crate) const RECIPE_FILE_FIELDS: &[&str] = &["recipe", "resources"];

pub(crate) const RECIPE_META_FIELDS: &[&str] =
    &["name", "version", "description", "inputs", "requires"];

pub(crate) const RECIPE_INPUT_FIELDS: &[&str] =
    &["type", "description", "default", "min", "max", "choices"];

pub(crate) const RECIPE_REQUIREMENT_FIELDS: &[&str] = &["recipe"];
