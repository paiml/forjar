//! The quality-gate knobs on `forjar lint`.
//!
//! Flattened into [`super::LintArgs`] rather than declared inside it: they are
//! the gate's surface, not lint's, and the same three would be flattened into
//! any other leaf that grows one.

use std::path::PathBuf;

/// Gate flags shared by every leaf that runs `core::quality_gate`.
#[derive(clap::Args, Debug, Default, Clone)]
pub struct LintGateArgs {
    /// Emit the quality gate as SARIF 2.1.0 for CI ingestion
    #[arg(long)]
    pub sarif: bool,

    /// Directory of compliance packs to evaluate as part of the gate. A pack
    /// rule of `type: script` RUNS its shell locally, so this flag executes
    /// what the pack author wrote. CLI-only, deliberately: `forjar_lint`
    /// publishes `readOnlyHint: true`, so it takes no such parameter (#356).
    /// A directory that cannot be READ fails the gate rather than passing it.
    #[arg(long)]
    pub policy_dir: Option<PathBuf>,

    /// Flag generated shell whose cyclomatic complexity exceeds N. Omitted,
    /// the script is never parsed — the parse is the expensive part.
    #[arg(long)]
    pub max_cyclomatic: Option<usize>,
}
