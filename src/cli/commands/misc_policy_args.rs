//! CLI Args structs for the `policy` command family.
//!
//! `policy`, `policy-coverage` and `policy-install` are one surface — evaluate
//! the compliance packs, report what they cover, install a pack — split off
//! from the `misc_args` grab-bag the same way the analysis commands were.

use std::path::PathBuf;

/// CLI arguments for the `policy` command.
#[derive(clap::Args, Debug)]
pub struct PolicyArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// FJ-3207: Output as SARIF 2.1.0 (for GitHub Code Scanning / CI)
    #[arg(long)]
    pub sarif: bool,
}

/// FJ-3208: CLI arguments for the `policy-coverage` command.
#[derive(clap::Args, Debug)]
pub struct PolicyCoverageArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// FJ-3206: CLI arguments for the `policy-install` command.
#[derive(clap::Args, Debug)]
pub struct PolicyInstallArgs {
    /// Pack name (e.g., cis-ubuntu-22, nist-800-53, soc2, hipaa)
    pub pack: String,
    /// Output directory for installed pack
    #[arg(long, default_value = "policies")]
    pub output_dir: PathBuf,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}
