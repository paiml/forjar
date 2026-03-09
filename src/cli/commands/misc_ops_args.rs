//! CLI Args structs for misc-related commands (ops/deploy).

use super::CompletionShell;
use std::path::PathBuf;

/// CLI arguments for the `doctor` command.
#[derive(clap::Args, Debug)]
pub struct DoctorArgs {
    /// Path to forjar.yaml (optional — checks system basics without it)
    #[arg(short, long)]
    pub file: Option<PathBuf>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// FJ-287: Auto-fix common issues (create state dir, remove stale locks)
    #[arg(long)]
    pub fix: bool,

    /// FJ-343: Test SSH connectivity to all machines
    #[arg(long)]
    pub network: bool,
}

/// CLI arguments for the `completion` command.
#[derive(clap::Args, Debug)]
pub struct CompletionArgs {
    /// Shell to generate completions for
    #[arg(value_enum)]
    pub shell: CompletionShell,
}

/// CLI arguments for the `watch` command.
#[derive(clap::Args, Debug)]
pub struct WatchArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Polling interval in seconds
    #[arg(long, default_value = "2")]
    pub interval: u64,

    /// Auto-apply on change (requires --yes)
    #[arg(long)]
    pub apply: bool,

    /// Confirm auto-apply (required with --apply)
    #[arg(long)]
    pub yes: bool,
}

/// CLI arguments for the `explain` command.
#[derive(clap::Args, Debug)]
pub struct ExplainArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Resource ID to explain
    pub resource: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// CLI arguments for the `env` command.
#[derive(clap::Args, Debug)]
pub struct EnvArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// CLI arguments for the `test` command.
#[derive(clap::Args, Debug)]
pub struct TestArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Target specific machine
    #[arg(short, long)]
    pub machine: Option<String>,

    /// Target specific resource
    #[arg(short, long)]
    pub resource: Option<String>,

    /// Filter to resources with this tag
    #[arg(short, long)]
    pub tag: Option<String>,

    /// FJ-281: Filter to resources in this group
    #[arg(short, long)]
    pub group: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Sandbox backend: pepita, container, chroot
    #[arg(long, default_value = "pepita")]
    pub sandbox: String,

    /// Parallelism level for test execution
    #[arg(long, default_value = "4")]
    pub parallel: usize,

    /// Enable pairwise preservation matrix (convergence tests)
    #[arg(long)]
    pub pairs: bool,

    /// Max mutations per resource (mutation tests)
    #[arg(long, default_value = "50")]
    pub mutations: usize,
}

/// CLI arguments for the `inventory` command.
#[derive(clap::Args, Debug)]
pub struct InventoryArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// CLI arguments for the `retry-failed` command.
#[derive(clap::Args, Debug)]
pub struct RetryFailedArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Override a parameter (KEY=VALUE)
    #[arg(long = "param", value_name = "KEY=VALUE")]
    pub params: Vec<String>,

    /// Timeout per transport operation (seconds)
    #[arg(long)]
    pub timeout: Option<u64>,
}

/// CLI arguments for the `rolling` command.
#[derive(clap::Args, Debug)]
pub struct RollingArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Number of machines to apply concurrently
    #[arg(long, default_value = "1")]
    pub batch_size: usize,

    /// Override a parameter (KEY=VALUE)
    #[arg(long = "param", value_name = "KEY=VALUE")]
    pub params: Vec<String>,

    /// Timeout per transport operation (seconds)
    #[arg(long)]
    pub timeout: Option<u64>,
}

/// CLI arguments for the `canary` command.
#[derive(clap::Args, Debug)]
pub struct CanaryArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Machine to use as canary (apply first)
    #[arg(short, long)]
    pub machine: String,

    /// Auto-proceed after canary success (skip confirmation)
    #[arg(long)]
    pub auto_proceed: bool,

    /// Override a parameter (KEY=VALUE)
    #[arg(long = "param", value_name = "KEY=VALUE")]
    pub params: Vec<String>,

    /// Timeout per transport operation (seconds)
    #[arg(long)]
    pub timeout: Option<u64>,
}

/// CLI arguments for the `audit` command.
#[derive(clap::Args, Debug)]
pub struct AuditArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Target specific machine
    #[arg(short, long)]
    pub machine: Option<String>,

    /// Show last N entries (default: 20)
    #[arg(short = 'n', long, default_value = "20")]
    pub limit: usize,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// CLI arguments for the `compliance` command.
#[derive(clap::Args, Debug)]
pub struct ComplianceArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// CLI arguments for the `export` command.
#[derive(clap::Args, Debug)]
pub struct ExportArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Output format: terraform, ansible, csv
    #[arg(long, default_value = "csv")]
    pub format: String,

    /// Target specific machine
    #[arg(short, long)]
    pub machine: Option<String>,

    /// Output file (default: stdout)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

/// CLI arguments for the `suggest` command.
#[derive(clap::Args, Debug)]
pub struct SuggestArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// CLI arguments for the `compare` command.
#[derive(clap::Args, Debug)]
pub struct CompareArgs {
    /// First config file
    pub file1: PathBuf,

    /// Second config file
    pub file2: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// CLI arguments for the `env-diff` command.
#[derive(clap::Args, Debug)]
pub struct EnvDiffArgs {
    /// First workspace name
    pub env1: String,

    /// Second workspace name
    pub env2: String,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// CLI arguments for the `template` command.
#[derive(clap::Args, Debug)]
pub struct TemplateArgs {
    /// Path to recipe YAML file
    pub recipe: PathBuf,

    /// Variable overrides (KEY=VALUE)
    #[arg(short = 'V', long = "var", value_name = "KEY=VALUE")]
    pub vars: Vec<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// CLI arguments for the `bootstrap` command (FJ-49).
#[derive(clap::Args, Debug)]
pub struct BootstrapArgs {
    /// Machine address (IP or hostname)
    #[arg(long)]
    pub addr: String,

    /// SSH user
    #[arg(long, default_value = "root")]
    pub user: String,

    /// Read password from stdin (for SSH key copy and sudo setup)
    #[arg(long)]
    pub password_stdin: bool,

    /// SSH public key to copy (default: ~/.ssh/id_ed25519.pub)
    #[arg(long)]
    pub ssh_key: Option<String>,

    /// Set machine hostname
    #[arg(long)]
    pub hostname: Option<String>,

    /// Skip key copy if SSH key auth already works
    #[arg(long)]
    pub skip_key_if_working: bool,
}

/// FJ-3107: CLI arguments for `forjar trigger` — manual event trigger.
#[derive(clap::Args, Debug)]
pub struct TriggerArgs {
    /// Rulebook name to trigger
    pub rulebook: String,

    /// Path to rulebook config
    #[arg(short, long, default_value = "forjar-rules.yaml")]
    pub file: PathBuf,

    /// Key=value payload fields
    #[arg(long, value_parser = parse_kv)]
    pub payload: Vec<(String, String)>,

    /// Dry-run: show what would fire without executing
    #[arg(long)]
    pub dry_run: bool,

    /// JSON output
    #[arg(long)]
    pub json: bool,
}

fn parse_kv(s: &str) -> Result<(String, String), String> {
    let (k, v) = s
        .split_once('=')
        .ok_or_else(|| format!("invalid key=value: {s}"))?;
    Ok((k.to_string(), v.to_string()))
}
