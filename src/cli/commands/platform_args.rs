//! CLI argument structs for platform features:
//! remote state, recipe registry, service catalog,
//! multi-config apply, stack dependency graph.

use clap::Parser;
use std::path::PathBuf;

/// Remote state backend operations.
#[derive(Parser, Debug)]
pub struct StateBackendArgs {
    /// State directory
    #[clap(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Filter by key prefix
    #[clap(long)]
    pub prefix: Option<String>,

    /// JSON output
    #[clap(long)]
    pub json: bool,
}

/// Recipe registry listing.
#[derive(Parser, Debug)]
pub struct RegistryListArgs {
    /// Registry directory
    #[clap(long)]
    pub registry_dir: Option<PathBuf>,

    /// JSON output
    #[clap(long)]
    pub json: bool,
}

/// Service catalog listing.
#[derive(Parser, Debug)]
pub struct CatalogListArgs {
    /// Catalog directory
    #[clap(long)]
    pub catalog_dir: Option<PathBuf>,

    /// Filter by category
    #[clap(long)]
    pub category: Option<String>,

    /// JSON output
    #[clap(long)]
    pub json: bool,
}

/// Multi-config apply ordering.
#[derive(Parser, Debug)]
pub struct MultiConfigArgs {
    /// Config files to analyze
    #[clap(short, long, required = true)]
    pub file: Vec<PathBuf>,

    /// JSON output
    #[clap(long)]
    pub json: bool,
}

/// Stack dependency graph.
#[derive(Parser, Debug)]
pub struct StackGraphArgs {
    /// Config files to analyze
    #[clap(short, long, required = true)]
    pub file: Vec<PathBuf>,

    /// JSON output
    #[clap(long)]
    pub json: bool,
}

/// Infrastructure query.
#[derive(Parser, Debug)]
pub struct InfraQueryArgs {
    /// Config file
    #[clap(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// State directory
    #[clap(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Search pattern
    #[clap(long)]
    pub pattern: Option<String>,

    /// Filter by resource type
    #[clap(long, name = "type")]
    pub resource_type: Option<String>,

    /// Filter by machine
    #[clap(long)]
    pub machine: Option<String>,

    /// Filter by tag
    #[clap(long)]
    pub tag: Option<String>,

    /// Show detailed output
    #[clap(long)]
    pub details: bool,

    /// Live mode (SSH probe)
    #[clap(long)]
    pub live: bool,

    /// JSON output
    #[clap(long)]
    pub json: bool,
}

/// Recipe signing.
#[derive(Parser, Debug)]
pub struct RecipeSignArgs {
    /// Recipe file to sign/verify
    pub recipe: PathBuf,

    /// Verify only (don't sign)
    #[clap(long)]
    pub verify: bool,

    /// Signer identity
    #[clap(long)]
    pub signer: Option<String>,

    /// Post-quantum dual signing
    #[clap(long)]
    pub pq: bool,

    /// JSON output
    #[clap(long)]
    pub json: bool,
}

/// Preservation checking.
#[derive(Parser, Debug)]
pub struct PreservationArgs {
    /// Config file
    #[clap(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// JSON output
    #[clap(long)]
    pub json: bool,
}

/// Parallel multi-stack apply.
#[derive(Parser, Debug)]
pub struct ParallelStackArgs {
    /// Config files
    #[clap(short, long, required = true)]
    pub file: Vec<PathBuf>,

    /// Maximum parallel stacks
    #[clap(long, default_value = "4")]
    pub max_parallel: usize,

    /// JSON output
    #[clap(long)]
    pub json: bool,
}

/// Saga-pattern multi-stack apply.
#[derive(Parser, Debug)]
pub struct SagaArgs {
    /// Config files
    #[clap(short, long, required = true)]
    pub file: Vec<PathBuf>,

    /// State directory
    #[clap(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// JSON output
    #[clap(long)]
    pub json: bool,
}

/// Pull agent / hybrid push-pull.
#[derive(Parser, Debug)]
pub struct PullAgentArgs {
    /// Config file
    #[clap(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// State directory
    #[clap(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Poll interval in seconds (pull mode)
    #[clap(long, default_value = "60")]
    pub interval: u64,

    /// Auto-apply when drift detected
    #[clap(long)]
    pub auto_apply: bool,

    /// Maximum iterations (default: unlimited in pull mode, 1 in push)
    #[clap(long)]
    pub max_iterations: Option<u64>,

    /// Pull mode (daemon loop); default is push (one-shot)
    #[clap(long)]
    pub pull: bool,

    /// JSON output
    #[clap(long)]
    pub json: bool,
}

/// Agent recipe registry.
#[derive(Parser, Debug)]
pub struct AgentRegistryArgs {
    /// Registry directory
    #[clap(long)]
    pub registry_dir: Option<PathBuf>,

    /// Filter by category
    #[clap(long)]
    pub category: Option<String>,

    /// JSON output
    #[clap(long)]
    pub json: bool,
}

/// FJ-2200: CLI arguments for `contracts`.
#[derive(clap::Args, Debug)]
pub struct ContractsArgs {
    /// Show contract coverage report
    #[arg(long)]
    pub coverage: bool,

    /// Path to forjar.yaml (for handler-level analysis)
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// JSON output
    #[arg(long)]
    pub json: bool,
}

/// FJ-2300: CLI arguments for `logs`.
#[derive(clap::Args, Debug)]
pub struct LogsArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Filter by machine
    #[arg(short, long)]
    pub machine: Option<String>,

    /// Filter by run ID
    #[arg(long)]
    pub run: Option<String>,

    /// Show only failures
    #[arg(long)]
    pub failures: bool,

    /// Follow mode — stream live during apply
    #[arg(long)]
    pub follow: bool,

    /// Garbage-collect old logs
    #[arg(long)]
    pub gc: bool,

    /// JSON output
    #[arg(long)]
    pub json: bool,
}

/// FJ-2104: CLI arguments for `build`.
#[derive(clap::Args, Debug)]
pub struct BuildArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Resource to build (must be type: image)
    #[arg(long)]
    pub resource: String,

    /// Load into local Docker daemon after build
    #[arg(long)]
    pub load: bool,

    /// Push to registry after build
    #[arg(long)]
    pub push: bool,

    /// JSON output
    #[arg(long)]
    pub json: bool,
}

/// FJ-2101: CLI arguments for `oci-pack`.
#[derive(clap::Args, Debug)]
pub struct OciPackArgs {
    /// Directory to pack into an OCI image
    pub dir: PathBuf,

    /// Image tag (name:tag)
    #[arg(long)]
    pub tag: String,

    /// Output directory for OCI layout
    #[arg(long, default_value = "oci-output")]
    pub output: PathBuf,

    /// JSON output
    #[arg(long)]
    pub json: bool,
}

/// FJ-2001: CLI arguments for `query`.
#[derive(clap::Args, Debug)]
pub struct QueryArgs {
    /// Search query (e.g., "bash", "nginx")
    pub query: String,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Filter by resource type
    #[arg(long, name = "type")]
    pub resource_type: Option<String>,

    /// Show history
    #[arg(long)]
    pub history: bool,

    /// Show drift status
    #[arg(long)]
    pub drift: bool,

    /// JSON output
    #[arg(long)]
    pub json: bool,

    /// CSV output
    #[arg(long)]
    pub csv: bool,
}
