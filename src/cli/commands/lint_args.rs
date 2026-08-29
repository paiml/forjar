//! `forjar lint`'s own arguments.
//!
//! Extracted from `misc_args.rs`, which the gate's `#[command(flatten)]`
//! pushed to 501 lines — one past the repo's 500-line ceiling, and
//! `misc_args.rs` is not one of the four files that ceiling exempts. It lives
//! beside `lint_gate_args.rs`, the struct it flattens.

use std::path::PathBuf;

/// CLI arguments for the `lint` command.
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

    /// FJ-374: Custom lint rules from YAML file [UNIMPLEMENTED — rejected, see GH-211]
    #[arg(long)]
    pub rules: Option<PathBuf>,

    /// FJ-2400: Show bashrs version used for script purification
    #[arg(long)]
    pub bashrs_version: bool,

    /// Quality-gate knobs: --sarif, --policy-dir, --max-cyclomatic
    #[command(flatten)]
    pub gate: super::LintGateArgs,
}
