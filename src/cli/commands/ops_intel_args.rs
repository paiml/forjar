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

/// paiml/forjar#356: policy-derived corrections to a forjar.yaml.
///
/// There is no `--write` and no `--in-place`. The command prints the corrected
/// document; redirection is the write. That keeps the CLI leaf and the
/// `forjar_remediate` verb the same operation rather than two with different
/// effects.
#[derive(clap::Args, Debug)]
pub struct RemediateArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: std::path::PathBuf,

    /// Restrict to these policy ids (repeatable). Omitted means every rule.
    #[arg(long = "policy-id")]
    pub policy_id: Vec<String>,

    /// Output the full report as JSON instead of the corrected document
    #[arg(long)]
    pub json: bool,
}
