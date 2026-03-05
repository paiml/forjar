//! CLI Args structs for lock_core-related commands.

use std::path::PathBuf;

/// CLI arguments for the `lock` command.
#[derive(clap::Args, Debug)]
pub struct LockArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// FJ-211: Load param overrides from external YAML file
    #[arg(long)]
    pub env_file: Option<PathBuf>,

    /// FJ-210: Use workspace (overrides state dir to state/<workspace>/)
    #[arg(short = 'w', long)]
    pub workspace: Option<String>,

    /// Verify existing lock matches config (exit 1 on mismatch)
    #[arg(long)]
    pub verify: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// CLI arguments for `lock prune`.
#[derive(clap::Args, Debug)]
pub struct LockPruneArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Actually remove entries (default: dry-run)
    #[arg(long)]
    pub yes: bool,
}

/// CLI arguments for `lock info`.
#[derive(clap::Args, Debug)]
pub struct LockInfoArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// CLI arguments for `lock compact`.
#[derive(clap::Args, Debug)]
pub struct LockCompactArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Actually compact (default: dry-run showing what would be removed)
    #[arg(long)]
    pub yes: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// CLI arguments for `lock gc`.
#[derive(clap::Args, Debug)]
pub struct LockGcArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Actually remove entries (default: dry-run)
    #[arg(long)]
    pub yes: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// CLI arguments for `lock export`.
#[derive(clap::Args, Debug)]
pub struct LockExportArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Output format: json, yaml, csv
    #[arg(long, default_value = "json")]
    pub format: String,

    /// Target specific machine
    #[arg(short, long)]
    pub machine: Option<String>,
}

/// CLI arguments for `lock verify`.
#[derive(clap::Args, Debug)]
pub struct LockVerifyArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// CLI arguments for `lock diff`.
#[derive(clap::Args, Debug)]
pub struct LockDiffArgs {
    /// First state directory (older)
    pub from: PathBuf,

    /// Second state directory (newer)
    pub to: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// CLI arguments for `lock merge`.
#[derive(clap::Args, Debug)]
pub struct LockMergeArgs {
    /// First state directory
    pub from: PathBuf,

    /// Second state directory (takes precedence on conflicts)
    pub to: PathBuf,

    /// Output directory for merged state
    #[arg(long, default_value = "state")]
    pub output: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// CLI arguments for `lock rebase`.
#[derive(clap::Args, Debug)]
pub struct LockRebaseArgs {
    /// Source state directory
    pub from: PathBuf,

    /// Target config file
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Output state directory
    #[arg(long, default_value = "state")]
    pub output: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// CLI arguments for `lock sign`.
#[derive(clap::Args, Debug)]
pub struct LockSignArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Signing key (path to key file or inline)
    #[arg(long)]
    pub key: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// CLI arguments for `lock verify-sig`.
#[derive(clap::Args, Debug)]
pub struct LockVerifySigArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Signing key to verify against
    #[arg(long)]
    pub key: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// CLI arguments for `lock compact-all`.
#[derive(clap::Args, Debug)]
pub struct LockCompactAllArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Skip confirmation
    #[arg(long)]
    pub yes: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// CLI arguments for `lock audit-trail`.
#[derive(clap::Args, Debug)]
pub struct LockAuditTrailArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Target specific machine
    #[arg(short, long)]
    pub machine: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// CLI arguments for `lock rotate-keys`.
#[derive(clap::Args, Debug)]
pub struct LockRotateKeysArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Old signing key
    #[arg(long)]
    pub old_key: String,

    /// New signing key
    #[arg(long)]
    pub new_key: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// CLI arguments for `lock backup`.
#[derive(clap::Args, Debug)]
pub struct LockBackupArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}
