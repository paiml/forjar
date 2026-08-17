//! GH-247: arguments for `forjar verify`.

use std::path::PathBuf;

/// CLI arguments for the `verify` command.
///
/// Deliberately has no `--fix`, `--restore` or `--write` flag. The issue this
/// implements asks for a check whose defining property is that it never writes
/// the declared output path, and a flag that relaxes that would make the
/// guarantee conditional on the caller remembering not to pass it.
#[derive(clap::Args, Debug)]
pub struct VerifyArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Target specific resource
    #[arg(short, long)]
    pub resource: Option<String>,

    /// Filter to resources with this tag
    #[arg(long)]
    pub tag: Option<String>,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Output as JSON (for CI gating)
    #[arg(long)]
    pub json: bool,

    /// Keep the scratch tree instead of removing it (for inspecting a mismatch)
    #[arg(long)]
    pub keep_scratch: bool,
}
