//! ALB-027: Task resource handler.
//!
//! Runs an arbitrary command, tracks exit code, hashes output artifacts
//! for idempotency, supports completion_check and timeout.
//!
//! Split into a module directory (Refs #390): closing the nested-shell hole
//! took `task.rs` past the repo's 500-line file-health limit. The split is
//! mechanical — every function kept its body, its name and its visibility, and
//! `pub use` below preserves the flat `resources::task::*` surface the tests
//! and `core::executor` already import.

mod apply;
mod check;
mod helpers;
mod query;

pub use apply::apply_script;
pub use check::check_script;
/// Only the in-crate test module reaches for this one; `check` calls it
/// directly. Re-exporting it unconditionally would be an unused import in a
/// non-test build, and this crate builds with `-D warnings`.
#[cfg(test)]
pub(crate) use helpers::extract_absolute_binary;
pub(crate) use helpers::heredoc_delimiter;
pub use helpers::NOT_CONVERGED_MARKER;
pub use query::{gather_script, scatter_script, state_query_script};
