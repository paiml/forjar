//! CLI Args structs for state-related commands.

use std::path::PathBuf;

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
