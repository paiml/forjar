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
