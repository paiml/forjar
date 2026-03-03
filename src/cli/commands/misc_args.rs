//! CLI Args structs for misc-related commands (core).

use std::path::PathBuf;

#[derive(clap::Args, Debug)]
pub struct InitArgs {
    /// Directory to initialize (default: current)
    #[arg(default_value = ".")]
    pub path: PathBuf,
}

#[derive(clap::Args, Debug)]
pub struct DriftArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Target specific machine
    #[arg(short, long)]
    pub machine: Option<String>,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Exit non-zero on any drift (for CI/cron)
    #[arg(long)]
    pub tripwire: bool,

    /// Run command on drift detection
    #[arg(long)]
    pub alert_cmd: Option<String>,

    /// Auto-remediate: re-apply drifted resources to restore desired state
    #[arg(long)]
    pub auto_remediate: bool,

    /// Show what would be checked without connecting to machines
    #[arg(long)]
    pub dry_run: bool,

    /// Output drift report as JSON
    #[arg(long)]
    pub json: bool,

    /// FJ-211: Load param overrides from external YAML file
    #[arg(long)]
    pub env_file: Option<PathBuf>,

    /// FJ-210: Use workspace (overrides state dir to state/<workspace>/)
    #[arg(short = 'w', long)]
    pub workspace: Option<String>,
}

#[derive(clap::Args, Debug)]
pub struct HistoryArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Show history for specific machine
    #[arg(short, long)]
    pub machine: Option<String>,

    /// Show last N applies (default: 10)
    #[arg(short = 'n', long, default_value = "10")]
    pub limit: usize,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// FJ-284: Show only events from the last duration (e.g., 24h, 7d, 30m)
    #[arg(long)]
    pub since: Option<String>,

    /// FJ-357: Show change history for a specific resource
    #[arg(long)]
    pub resource: Option<String>,
}

#[derive(clap::Args, Debug)]
pub struct DestroyArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Target specific machine
    #[arg(short, long)]
    pub machine: Option<String>,

    /// Skip confirmation prompt
    #[arg(long)]
    pub yes: bool,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,
}

#[derive(clap::Args, Debug)]
pub struct ImportArgs {
    /// Machine address (IP, hostname, or 'localhost')
    #[arg(short, long)]
    pub addr: String,

    /// SSH user
    #[arg(short, long, default_value = "root")]
    pub user: String,

    /// Machine name (used as key in machines section)
    #[arg(short, long)]
    pub name: Option<String>,

    /// Output file
    #[arg(short, long, default_value = "forjar.yaml")]
    pub output: PathBuf,

    /// What to scan
    #[arg(long, value_delimiter = ',', default_value = "packages,files,services")]
    pub scan: Vec<String>,
}

#[derive(clap::Args, Debug)]
pub struct ShowArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Show specific resource only
    #[arg(short, long)]
    pub resource: Option<String>,

    /// Output as JSON instead of YAML
    #[arg(long)]
    pub json: bool,
}

#[derive(clap::Args, Debug)]
pub struct CheckArgs {
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
    #[arg(long)]
    pub tag: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(clap::Args, Debug)]
pub struct DiffArgs {
    /// First state directory (older)
    pub from: PathBuf,

    /// Second state directory (newer)
    pub to: PathBuf,

    /// Filter to specific machine
    #[arg(short, long)]
    pub machine: Option<String>,

    /// FJ-291: Filter to specific resource
    #[arg(short, long)]
    pub resource: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(clap::Args, Debug)]
pub struct FmtArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Check formatting without writing (exit non-zero if unformatted)
    #[arg(long)]
    pub check: bool,
}

#[derive(clap::Args, Debug)]
pub struct LintArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// FJ-221: Enable built-in policy rules (no_root_owner, require_tags, etc.)
    #[arg(long)]
    pub strict: bool,

    /// FJ-332: Auto-fix common lint issues (normalize quotes, sort keys)
    #[arg(long)]
    pub fix: bool,

    /// FJ-374: Custom lint rules from YAML file
    #[arg(long)]
    pub rules: Option<PathBuf>,
}

#[derive(clap::Args, Debug)]
pub struct RollbackArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Git revision to rollback to (default: HEAD~1)
    #[arg(short = 'n', long, default_value = "1")]
    pub revision: u32,

    /// Target specific machine
    #[arg(short, long)]
    pub machine: Option<String>,

    /// Show what would change without applying
    #[arg(long)]
    pub dry_run: bool,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,
}

#[derive(clap::Args, Debug)]
pub struct AnomalyArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Target specific machine
    #[arg(short, long)]
    pub machine: Option<String>,

    /// Minimum events to consider (ignore resources with fewer)
    #[arg(long, default_value = "3")]
    pub min_events: usize,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(clap::Args, Debug)]
pub struct TraceArgs {
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

#[derive(clap::Args, Debug)]
pub struct MigrateArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Write migrated config to file (default: stdout)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(clap::Args, Debug)]
pub struct McpArgs {
    /// Export tool schemas as JSON instead of starting server
    #[arg(long)]
    pub schema: bool,
}

#[derive(clap::Args, Debug)]
pub struct BenchArgs {
    /// Number of iterations per benchmark (default: 1000)
    #[arg(long, default_value = "1000")]
    pub iterations: usize,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(clap::Args, Debug)]
pub struct OutputArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Specific output key to show (omit for all)
    pub key: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(clap::Args, Debug)]
pub struct PolicyArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(clap::Args, Debug)]
pub struct ScoreArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Recipe status (qualified, blocked, pending)
    #[arg(long, default_value = "qualified")]
    pub status: String,

    /// Idempotency class (strong, weak, eventual)
    #[arg(long, default_value = "strong")]
    pub idempotency: String,

    /// Performance budget in milliseconds (0 = no budget)
    #[arg(long, default_value_t = 0)]
    pub budget_ms: u64,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// FJ-1383: Merge two config files into one.
#[derive(clap::Args, Debug)]
pub struct ConfigMergeArgs {
    /// First config file
    pub file_a: std::path::PathBuf,

    /// Second config file
    pub file_b: std::path::PathBuf,

    /// Output file (default: stdout)
    #[arg(short, long)]
    pub output: Option<std::path::PathBuf>,

    /// Allow resource ID collisions (right takes precedence)
    #[arg(long)]
    pub allow_collisions: bool,
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
