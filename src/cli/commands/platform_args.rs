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
