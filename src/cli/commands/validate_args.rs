//! CLI Args structs for validate-related commands.

use std::path::PathBuf;


#[derive(clap::Args, Debug)]
pub struct ValidateArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// FJ-282: Extended validation — check machine refs, paths, deps, templates
    #[arg(long)]
    pub strict: bool,

    /// Output validation result as JSON
    #[arg(long)]
    pub json: bool,

    /// FJ-330: Show fully expanded config after template resolution
    #[arg(long)]
    pub dry_expand: bool,

    /// FJ-381: Validate against specific schema version
    #[arg(long)]
    pub schema_version: Option<String>,

    /// FJ-391: Validate all cross-references, machine existence, and param usage
    #[arg(long)]
    pub exhaustive: bool,

    /// FJ-401: Validate against external policy rules file
    #[arg(long)]
    pub policy_file: Option<PathBuf>,

    /// FJ-411: Test SSH connectivity to all machines during validation
    #[arg(long)]
    pub check_connectivity: bool,

    /// FJ-421: Verify all template variables resolve
    #[arg(long)]
    pub check_templates: bool,

    /// FJ-431: Verify dependency ordering matches resource declaration order
    #[arg(long)]
    pub strict_deps: bool,

    /// FJ-441: Scan config for hardcoded secrets or credentials
    #[arg(long)]
    pub check_secrets: bool,

    /// FJ-451: Verify all resources produce idempotent scripts
    #[arg(long)]
    pub check_idempotency: bool,

    /// FJ-461: Verify all resources have drift detection configured
    #[arg(long)]
    pub check_drift_coverage: bool,

    /// FJ-471: Detect indirect circular dependencies via transitive closure
    #[arg(long)]
    pub check_cycles_deep: bool,

    /// FJ-481: Enforce resource naming conventions (kebab-case, prefix rules)
    #[arg(long)]
    pub check_naming: bool,

    /// FJ-491: Detect resources targeting the same path/port/name on same machine
    #[arg(long)]
    pub check_overlaps: bool,

    /// FJ-501: Enforce resource count limits per machine/type
    #[arg(long)]
    pub check_limits: bool,

    /// FJ-511: Warn on resources with high dependency fan-out
    #[arg(long)]
    pub check_complexity: bool,

    /// FJ-521: Scan for insecure permissions, ports, or user configs
    #[arg(long)]
    pub check_security: bool,

    /// FJ-531: Warn on deprecated resource fields or types
    #[arg(long)]
    pub check_deprecation: bool,

    /// FJ-541: Score drift risk based on resource volatility
    #[arg(long)]
    pub check_drift_risk: bool,

    /// FJ-551: Validate against compliance policy (CIS, SOC2)
    #[arg(long)]
    pub check_compliance: Option<String>,

    /// FJ-561: Check resources for platform-specific assumptions
    #[arg(long)]
    pub check_portability: bool,

    /// FJ-571: Validate resource counts don't exceed per-machine limits
    #[arg(long)]
    pub check_resource_limits: bool,

    /// FJ-581: Detect resources not referenced by any dependency chain
    #[arg(long)]
    pub check_unused: bool,

    /// FJ-591: Validate all depends_on references resolve correctly
    #[arg(long)]
    pub check_dependencies: bool,

    /// FJ-601: Validate resource ownership/mode fields are secure
    #[arg(long)]
    pub check_permissions: bool,

    /// FJ-611: Deep idempotency analysis with simulation
    #[arg(long)]
    pub check_idempotency_deep: bool,

    /// FJ-621: Verify machines are reachable before apply
    #[arg(long)]
    pub check_machine_reachability: bool,

    /// FJ-631: Detect circular template/param references
    #[arg(long)]
    pub check_circular_refs: bool,

    /// FJ-641: Enforce naming conventions across resources
    #[arg(long)]
    pub check_naming_conventions: bool,

    /// FJ-661: Ensure all resources have consistent ownership
    #[arg(long)]
    pub check_owner_consistency: bool,

    /// FJ-671: Detect overlapping file paths across resources
    #[arg(long)]
    pub check_path_conflicts: bool,

    /// FJ-681: Validate service dependency chains are satisfiable
    #[arg(long)]
    pub check_service_deps: bool,

    /// FJ-691: Validate all template variables are defined
    #[arg(long)]
    pub check_template_vars: bool,

    /// FJ-701: Validate file mode consistency across resources
    #[arg(long)]
    pub check_mode_consistency: bool,

    /// FJ-711: Validate user/group consistency across resources
    #[arg(long)]
    pub check_group_consistency: bool,

    /// FJ-721: Validate mount point paths don't conflict
    #[arg(long)]
    pub check_mount_points: bool,

    /// FJ-731: Validate cron schedule expressions
    #[arg(long)]
    pub check_cron_syntax: bool,

    /// FJ-741: Verify all {{env.*}} references have matching env vars
    #[arg(long)]
    pub check_env_refs: bool,

    /// FJ-745: Enforce resource naming regex pattern
    #[arg(long)]
    pub check_resource_names: Option<String>,

    /// FJ-749: Warn if resource count exceeds threshold per machine
    #[arg(long)]
    pub check_resource_count: Option<usize>,

    /// FJ-753: Detect duplicate file paths across resources on same machine
    #[arg(long)]
    pub check_duplicate_paths: bool,

    /// FJ-757: Detect circular dependency chains
    #[arg(long)]
    pub check_circular_deps: bool,

    /// FJ-761: Verify all machine references in resources exist
    #[arg(long)]
    pub check_machine_refs: bool,

    /// FJ-765: Verify consistent package providers per machine
    #[arg(long)]
    pub check_provider_consistency: bool,

    /// FJ-769: Verify state field values are valid for each resource type
    #[arg(long)]
    pub check_state_values: bool,

    /// FJ-773: Detect machines defined but not referenced by any resource
    #[arg(long)]
    pub check_unused_machines: bool,

    /// FJ-777: Verify resource tags follow naming conventions
    #[arg(long)]
    pub check_tag_consistency: bool,

    /// FJ-781: Verify all depends_on targets reference existing resources
    #[arg(long)]
    pub check_dependency_exists: bool,

    /// FJ-785: Detect resources targeting the same file path on the same machine
    #[arg(long)]
    pub check_path_conflicts_strict: bool,

    /// FJ-789: Detect duplicate resource names across groups
    #[arg(long)]
    pub check_duplicate_names: bool,

    /// FJ-793: Verify resource groups are non-empty
    #[arg(long)]
    pub check_resource_groups: bool,

    /// FJ-797: Detect resources not reachable from any root
    #[arg(long)]
    pub check_orphan_resources: bool,

    /// FJ-801: Verify resource compatibility with machine architecture
    #[arg(long)]
    pub check_machine_arch: bool,

    /// FJ-805: Detect resources with conflicting health indicators
    #[arg(long)]
    pub check_resource_health_conflicts: bool,

    /// FJ-809: Detect resources with overlapping scope on same machine
    #[arg(long)]
    pub check_resource_overlap: bool,

    /// FJ-813: Enforce tag conventions (required tags, naming rules)
    #[arg(long)]
    pub check_resource_tags: bool,

    /// FJ-817: Verify state fields match resource type constraints
    #[arg(long)]
    pub check_resource_state_consistency: bool,

    /// FJ-821: Verify all depends_on targets actually exist
    #[arg(long)]
    pub check_resource_dependencies_complete: bool,

    /// FJ-825: Verify machines are reachable (dry-run connectivity check)
    #[arg(long)]
    pub check_machine_connectivity: bool,

    /// FJ-829: Enforce regex naming pattern for resources
    #[arg(long)]
    pub check_resource_naming_pattern: Option<String>,

    /// FJ-833: Verify providers match resource types
    #[arg(long)]
    pub check_resource_provider_support: bool,

    /// FJ-837: Verify secret references exist and are valid
    #[arg(long)]
    pub check_resource_secret_refs: bool,

    /// FJ-841: Check resources have idempotency markers
    #[arg(long)]
    pub check_resource_idempotency_hints: bool,

    /// FJ-845: Warn if dependency chain exceeds threshold
    #[arg(long)]
    pub check_resource_dependency_depth: Option<usize>,

    /// FJ-849: Verify resources match machine capabilities
    #[arg(long)]
    pub check_resource_machine_affinity: bool,

    /// FJ-853: Score drift risk per resource based on type + deps
    #[arg(long)]
    pub check_resource_drift_risk: bool,

    /// FJ-857: Verify all resources have required tags
    #[arg(long)]
    pub check_resource_tag_coverage: bool,

    /// FJ-861: Verify lifecycle hook references are valid
    #[arg(long)]
    pub check_resource_lifecycle_hooks: bool,

    /// FJ-865: Verify provider version compatibility
    #[arg(long)]
    pub check_resource_provider_version: bool,
}

