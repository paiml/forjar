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
}

