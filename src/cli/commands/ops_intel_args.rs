//! CLI Args structs for operational intelligence commands (complexity, impact, drift-predict).

/// FJ-1450: Configuration complexity analysis.
#[derive(clap::Args, Debug)]
pub struct ComplexityArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: std::path::PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// FJ-1451: Dependency impact analysis.
#[derive(clap::Args, Debug)]
pub struct ImpactArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: std::path::PathBuf,

    /// Target resource to analyze impact for
    #[arg(short, long)]
    pub resource: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// FJ-1452: Configuration drift prediction.
#[derive(clap::Args, Debug)]
pub struct DriftPredictArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: std::path::PathBuf,

    /// Target specific machine
    #[arg(short, long)]
    pub machine: Option<String>,

    /// Limit number of predictions shown
    #[arg(short, long, default_value = "0")]
    pub limit: usize,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}
