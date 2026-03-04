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
