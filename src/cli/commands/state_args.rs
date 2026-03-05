//! CLI Args structs for state-related commands (including undo).

use std::path::PathBuf;

/// FJ-2003: CLI arguments for the `undo` command.
#[derive(clap::Args, Debug)]
pub struct UndoArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Number of generations to undo (default: 1)
    #[arg(long, default_value = "1")]
    pub generations: u32,

    /// Target specific machine
    #[arg(short, long)]
    pub machine: Option<String>,

    /// Show what would change without executing
    #[arg(long)]
    pub dry_run: bool,

    /// Resume a partial undo
    #[arg(long)]
    pub resume: bool,

    /// Confirm undo
    #[arg(long)]
    pub yes: bool,
}

/// CLI arguments for `state list`.
#[derive(clap::Args, Debug)]
pub struct StateListArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Filter to specific machine
    #[arg(short, long)]
    pub machine: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// CLI arguments for `state mv`.
#[derive(clap::Args, Debug)]
pub struct StateMvArgs {
    /// Current resource ID
    pub old_id: String,

    /// New resource ID
    pub new_id: String,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Target machine (required if multiple machines have this resource)
    #[arg(short, long)]
    pub machine: Option<String>,
}

/// CLI arguments for `state rm`.
#[derive(clap::Args, Debug)]
pub struct StateRmArgs {
    /// Resource ID to remove
    pub resource_id: String,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Target machine (required if multiple machines have this resource)
    #[arg(short, long)]
    pub machine: Option<String>,

    /// Skip dependency check and force removal
    #[arg(long)]
    pub force: bool,
}

/// FJ-1280: Reconstruct state at a point in time from event log.
#[derive(clap::Args, Debug)]
pub struct StateReconstructArgs {
    /// Target machine
    #[arg(short, long)]
    pub machine: String,

    /// ISO 8601 timestamp to reconstruct at (e.g., 2026-03-01T14:00:00Z)
    #[arg(long)]
    pub at: String,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}
