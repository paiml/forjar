//! CLI Args structs for misc-related commands (core).

use std::path::PathBuf;

/// CLI arguments for the `init` command.
#[derive(clap::Args, Debug)]
pub struct InitArgs {
    /// Directory to initialize (default: current)
    #[arg(default_value = ".")]
    pub path: PathBuf,
}

/// CLI arguments for the `drift` command.
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

    /// Do not execute task completion_checks (cheaper, and blind to guards)
    ///
    /// forjar#380: drift executes the `completion_check` of every converged
    /// task, because for a guard resource that assertion IS the observable.
    /// This opts out for a run where the extra executions are unaffordable, or
    /// where a check is not the pure predicate it is supposed to be. What it
    /// silences is counted in the census line, never dropped in silence.
    #[arg(long)]
    pub no_task_checks: bool,
}

/// CLI arguments for the `history` command.
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

/// CLI arguments for the `destroy` command.
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

/// CLI arguments for the `import` command.
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

    /// Smart filter: only include manually installed packages (not base system)
    #[arg(long)]
    pub smart: bool,
}

/// CLI arguments for the `show` command.
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

/// CLI arguments for the `codegen` command.
///
/// FJ-038: emit the shell a resource GENERATES, resolved exactly as `apply`
/// would resolve it. Without this, a resource whose real payload is synthesised
/// shell can only be tested against fixtures its own author wrote — which is
/// how three inverted-assumption bugs shipped in 1.13.0/1.13.1. Dogfooding a
/// generated artifact requires being able to get at it.
#[derive(clap::Args, Debug)]
pub struct CodegenArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Resource id to emit
    #[arg(short, long)]
    pub resource: String,

    /// Which script to emit. `reaper` is the disk_budget reclaim pass alone,
    /// which previews unless FORJAR_BUDGET_EXECUTE=1 is set; `apply` emits the
    /// installer, which grants that opt-in and deletes (forjar#334).
    #[arg(long, default_value = "apply", value_parser = ["apply", "check", "state-query", "reaper"])]
    pub phase: String,
}

/// CLI arguments for the `dogfood` command.
#[derive(clap::Args, Debug)]
pub struct DogfoodArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// CLI arguments for the `check` command.
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

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// CLI arguments for the `diff` command.
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

/// FJ-1389: Unified stack diff — resource, machine, param comparison.
#[derive(clap::Args, Debug)]
pub struct StackDiffArgs {
    /// First config file
    pub file1: PathBuf,

    /// Second config file
    pub file2: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// CLI arguments for the `fmt` command.
#[derive(clap::Args, Debug)]
pub struct FmtArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Check formatting without writing (exit non-zero if unformatted)
    #[arg(long)]
    pub check: bool,
}

/// CLI arguments for the `rollback` command.
#[derive(clap::Args, Debug)]
pub struct RollbackArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Git revision to rollback to (default: HEAD~1)
    #[arg(short = 'n', long, default_value = "1")]
    pub revision: u32,

    /// FJ-1386: Rollback to a specific state generation (Nix-style)
    #[arg(long)]
    pub generation: Option<u32>,

    /// Target specific machine
    #[arg(short, long)]
    pub machine: Option<String>,

    /// Show what would change without applying
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm destructive operation
    #[arg(long)]
    pub yes: bool,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,
}

/// CLI arguments for the `anomaly` command.
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

/// CLI arguments for the `trace` command.
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

/// CLI arguments for the `migrate` command.
#[derive(clap::Args, Debug)]
pub struct MigrateArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Write migrated config to file (default: stdout)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

/// CLI arguments for the `mcp` command.
#[derive(clap::Args, Debug)]
pub struct McpArgs {
    /// Export tool schemas as JSON instead of starting server
    #[arg(long)]
    pub schema: bool,
}

/// CLI arguments for the `bench` command.
#[derive(clap::Args, Debug)]
pub struct BenchArgs {
    /// Number of iterations per benchmark (must be >= 1; default: 1000)
    // Dogfood #208 (bench-iterations-zero-nan-and-exit-zero): `--iterations 0`
    // divided by zero, printed "NaNµs", marked all six targets FAIL and still
    // exited 0. clap already rejects `abc` here — apply the same mechanism to
    // the value's domain.
    #[arg(long, default_value = "1000", value_parser = clap::value_parser!(u64).range(1..))]
    pub iterations: u64,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Compare against stored baseline in benchmarks/RESULTS.md (errors if the
    /// baseline is absent or unparseable; exits non-zero on regression)
    #[arg(long)]
    pub compare: bool,
}

/// CLI arguments for the `output` command.
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

/// CLI arguments for the `score` command.
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

    /// FJ-3020: State directory for runtime data (events.jsonl)
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,
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

// SecurityScan, Sbom, Cbom, Prove, Extract, PrivilegeAnalysis, Provenance, Lineage
// args moved to misc_analysis_args.rs
// Policy, PolicyCoverage, PolicyInstall args moved to misc_policy_args.rs
