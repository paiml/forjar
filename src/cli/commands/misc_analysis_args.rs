//! CLI Args structs for analysis/audit commands (extract, security, sbom, cbom, prove, etc.)

/// FJ-1403: Least-privilege execution analysis.
#[derive(clap::Args, Debug)]
pub struct PrivilegeAnalysisArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: std::path::PathBuf,

    /// Target specific machine
    #[arg(short, long)]
    pub machine: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// FJ-1404: SLSA provenance attestation generation.
#[derive(clap::Args, Debug)]
pub struct ProvenanceArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: std::path::PathBuf,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: std::path::PathBuf,

    /// Target specific machine
    #[arg(short, long)]
    pub machine: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// FJ-1405: Merkle DAG configuration lineage.
#[derive(clap::Args, Debug)]
pub struct LineageArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: std::path::PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// FJ-1384: Extract resources matching tag/group/glob into sub-config.
#[derive(clap::Args, Debug)]
pub struct ExtractArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: std::path::PathBuf,

    /// Filter to resources with this tag
    #[arg(long)]
    pub tags: Option<String>,

    /// Filter to resources in this resource_group
    #[arg(long)]
    pub group: Option<String>,

    /// Filter to resource IDs matching glob pattern (e.g., "web-*")
    #[arg(long)]
    pub glob: Option<String>,

    /// Output file (default: stdout)
    #[arg(short, long)]
    pub output: Option<std::path::PathBuf>,

    /// Output as JSON instead of YAML
    #[arg(long)]
    pub json: bool,
}

/// FJ-1390: Static IaC security scanner — detect security smells in configs.
#[derive(clap::Args, Debug)]
pub struct SecurityScanArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: std::path::PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Fail on findings at or above this severity (critical, high, medium, low)
    #[arg(long)]
    pub fail_on: Option<String>,
}

/// FJ-1395: Generate SBOM (Software Bill of Materials) for managed infrastructure.
#[derive(clap::Args, Debug)]
pub struct SbomArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: std::path::PathBuf,

    /// State directory (for hash lookups)
    #[arg(long, default_value = "state")]
    pub state_dir: std::path::PathBuf,

    /// Output as SPDX JSON (default: text table)
    #[arg(long)]
    pub json: bool,
}

/// FJ-1400: Cryptographic Bill of Materials (CBOM) generation.
#[derive(clap::Args, Debug)]
pub struct CbomArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: std::path::PathBuf,

    /// State directory (for hash lookups)
    #[arg(long, default_value = "state")]
    pub state_dir: std::path::PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// FJ-1406: Self-contained recipe bundle packaging.
#[derive(clap::Args, Debug)]
pub struct BundleArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: std::path::PathBuf,

    /// Output archive path (default: dry-run manifest only)
    #[arg(short, long)]
    pub output: Option<std::path::PathBuf>,

    /// Include state directory in bundle
    #[arg(long)]
    pub include_state: bool,

    /// Verify an existing bundle manifest against filesystem
    #[arg(long)]
    pub verify: bool,
}

/// FJ-1407: Generate model card from config + state.
#[derive(clap::Args, Debug)]
pub struct ModelCardArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: std::path::PathBuf,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: std::path::PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// FJ-1408: Agent-specific SBOM generation.
#[derive(clap::Args, Debug)]
pub struct AgentSbomArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: std::path::PathBuf,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: std::path::PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// FJ-1409: Training reproducibility proof certificate.
#[derive(clap::Args, Debug)]
pub struct ReproProofArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: std::path::PathBuf,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: std::path::PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// FJ-1401: Convergence proof from arbitrary state.
#[derive(clap::Args, Debug)]
pub struct ProveArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: std::path::PathBuf,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: std::path::PathBuf,

    /// Machine to prove convergence for (default: all)
    #[arg(short, long)]
    pub machine: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// FJ-1410: Data freshness monitoring — detect stale artifacts.
#[derive(clap::Args, Debug)]
pub struct DataFreshnessArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: std::path::PathBuf,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: std::path::PathBuf,

    /// Maximum artifact age in hours (default: 24)
    #[arg(long)]
    pub max_age: Option<u64>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// FJ-1411: Declarative data validation checks.
#[derive(clap::Args, Debug)]
pub struct DataValidateArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: std::path::PathBuf,

    /// Target specific resource
    #[arg(short, long)]
    pub resource: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// FJ-1412: Training checkpoint management.
#[derive(clap::Args, Debug)]
pub struct CheckpointArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: std::path::PathBuf,

    /// Target specific machine
    #[arg(short, long)]
    pub machine: Option<String>,

    /// Run garbage collection on old checkpoints
    #[arg(long)]
    pub gc: bool,

    /// Number of checkpoints to keep (with --gc)
    #[arg(long, default_value = "5")]
    pub keep: usize,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// FJ-1413: Dataset versioning and lineage tracking.
#[derive(clap::Args, Debug)]
pub struct DatasetLineageArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: std::path::PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// FJ-1414: Data sovereignty tagging and compliance.
#[derive(clap::Args, Debug)]
pub struct SovereigntyArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: std::path::PathBuf,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: std::path::PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// FJ-1415: Cost estimation and resource budgeting.
#[derive(clap::Args, Debug)]
pub struct CostEstimateArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: std::path::PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// FJ-1416: Model evaluation pipeline.
#[derive(clap::Args, Debug)]
pub struct ModelEvalArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: std::path::PathBuf,

    /// Target specific resource
    #[arg(short, long)]
    pub resource: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}
