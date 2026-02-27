//! CLI Args structs for lock_ops-related commands.

use std::path::PathBuf;


#[derive(clap::Args, Debug)]
pub struct LockVerifyChainArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}


#[derive(clap::Args, Debug)]
pub struct LockStatsArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}


#[derive(clap::Args, Debug)]
pub struct LockAuditArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}


#[derive(clap::Args, Debug)]
pub struct LockCompressArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}


#[derive(clap::Args, Debug)]
pub struct LockDefragArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}


#[derive(clap::Args, Debug)]
pub struct LockNormalizeArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}


#[derive(clap::Args, Debug)]
pub struct LockValidateArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}


#[derive(clap::Args, Debug)]
pub struct LockVerifyHmacArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}


#[derive(clap::Args, Debug)]
pub struct LockArchiveArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}


#[derive(clap::Args, Debug)]
pub struct LockSnapshotArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}


#[derive(clap::Args, Debug)]
pub struct LockRepairArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}


#[derive(clap::Args, Debug)]
pub struct LockHistoryArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Maximum entries to show
    #[arg(long, default_value = "20")]
    pub limit: usize,
}


#[derive(clap::Args, Debug)]
pub struct LockIntegrityArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}


#[derive(clap::Args, Debug)]
pub struct LockRehashArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}


#[derive(clap::Args, Debug)]
pub struct LockRestoreArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Snapshot name to restore from
    #[arg(long)]
    pub name: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}


#[derive(clap::Args, Debug)]
pub struct LockVerifySchemaArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}


#[derive(clap::Args, Debug)]
pub struct LockTagArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Tag name
    #[arg(long)]
    pub name: String,

    /// Tag value
    #[arg(long)]
    pub value: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}


#[derive(clap::Args, Debug)]
pub struct LockMigrateArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Source schema version
    #[arg(long)]
    pub from_version: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

