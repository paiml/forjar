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

/// FJ-2005: CLI arguments for `undo-destroy`.
#[derive(clap::Args, Debug)]
pub struct UndoDestroyArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Target specific machine
    #[arg(short, long)]
    pub machine: Option<String>,

    /// Force re-creation of irreversible resources
    #[arg(long)]
    pub force: bool,

    /// Show what would be recreated without executing
    #[arg(long)]
    pub dry_run: bool,
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

/// FJ-118: CLI arguments for `reseal` — regenerate BLAKE3 sidecar(s) from
/// current lock file contents without running a full apply.
///
/// Use when local `state/<machine>/state.lock.yaml` diverged from its `.b3`
/// sidecar (e.g. old forjar versions that silently dropped sidecar-write
/// errors, or `git checkout` that restored one without the other).
#[derive(clap::Args, Debug)]
pub struct ResealArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Single lock file to reseal (mutually exclusive with --all)
    #[arg(long, conflicts_with = "all")]
    pub file: Option<PathBuf>,

    /// Reseal every lock file under state_dir
    #[arg(long, conflicts_with = "file")]
    pub all: bool,

    /// Target specific machine (reseal only that machine's lock)
    #[arg(short, long, conflicts_with_all = ["file", "all"])]
    pub machine: Option<String>,

    /// Print what would be resealed without writing sidecars
    #[arg(long)]
    pub dry_run: bool,
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
