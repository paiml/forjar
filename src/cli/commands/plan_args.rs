//! CLI Args structs for plan-related commands.

use std::path::PathBuf;

/// CLI arguments for the `plan` command.
#[derive(clap::Args, Debug)]
pub struct PlanArgs {
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

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Output plan as JSON
    #[arg(long)]
    pub json: bool,

    /// Write generated scripts to directory for auditing
    #[arg(long)]
    pub output_dir: Option<PathBuf>,

    /// FJ-211: Load param overrides from external YAML file
    #[arg(long)]
    pub env_file: Option<PathBuf>,

    /// FJ-210: Use workspace (overrides state dir to state/<workspace>/)
    #[arg(short = 'w', long)]
    pub workspace: Option<String>,

    /// FJ-255: Suppress content diff in plan output
    #[arg(long)]
    pub no_diff: bool,

    /// FJ-285: Plan single resource and its transitive dependencies
    #[arg(long)]
    pub target: Option<String>,

    /// FJ-312: Show estimated change cost per resource type
    #[arg(long)]
    pub cost: bool,

    /// FJ-333: Hypothetical param override — show plan as if param had this value
    #[arg(long = "what-if", value_name = "KEY=VALUE")]
    pub what_if: Vec<String>,

    /// FJ-1250: Write plan to file for later execution with `apply --plan-file`
    #[arg(long)]
    pub out: Option<PathBuf>,

    /// FJ-1379: Show per-resource explanation of why each change is needed
    #[arg(long)]
    pub why: bool,
}

/// CLI arguments for the `plan --compact` command.
#[derive(clap::Args, Debug)]
pub struct PlanCompactArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

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

/// CLI arguments for `forjar make [GOALS...]`.
///
/// Deliberately small and make-flavoured. `ApplyArgs` has ~40 flags; reusing it
/// wholesale would produce a `forjar make --help` nobody could read. The flags
/// here are the ones that have a `make` counterpart, named the same way.
#[derive(clap::Args, Debug)]
pub struct MakeArgs {
    /// Goals to build, with their prerequisites. No goals = the whole config.
    pub goals: Vec<String>,

    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Print what would run without running it (make -n)
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Unconditionally rebuild every goal (make -B)
    #[arg(short = 'B', long = "always-make")]
    pub always_make: bool,

    /// Maximum resources to converge in parallel (make -j)
    #[arg(short = 'j', long)]
    pub jobs: Option<usize>,

    /// Override a parameter (KEY=VALUE)
    #[arg(short, long)]
    pub param: Vec<String>,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Target specific machine
    #[arg(short, long)]
    pub machine: Option<String>,

    /// Skip provenance tracing
    #[arg(long)]
    pub no_tripwire: bool,

    /// Skip the confirmation prompt (CI mode)
    #[arg(long)]
    pub yes: bool,

    /// Output results as JSON
    #[arg(long)]
    pub json: bool,
}
