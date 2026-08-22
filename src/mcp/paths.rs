//! GH-208: state-directory resolution for MCP tools.
//!
//! Every MCP tool that reads state took its `state_dir` argument as
//!
//! ```ignore
//! let state_dir = PathBuf::from(input.state_dir.as_deref().unwrap_or("state"));
//! ```
//!
//! which defaults to the LITERAL RELATIVE path `state`, resolved against the
//! process's current working directory. The CLI never hits this because its cwd
//! *is* the project directory by construction. An MCP server's cwd is chosen by
//! the client, not the project — so a project addressed by an absolute `path`
//! was having its state looked for somewhere else entirely.
//!
//! The consequence was not a visible error. `state::load_lock` simply found
//! nothing, and every caller used `if let Ok(Some(lock))`, which discards both
//! `Err` and `Ok(None)`. So "I could not find your state" silently became
//! "you have no state", which in turn reads as:
//!   * `forjar_drift`  -> `{"drifted": false, "findings": []}` on a machine that
//!     had been demonstrably tampered with, while the CLI reported drift_count 1;
//!   * `forjar_plan`   -> every converged resource reported as CREATE.
//!
//! Both with `isError: false`.
//!
//! That is the project's worst-rated failure mode — a green that certifies the
//! wrong thing — on the tool the tripwire depends on.
//!
//! The parity tests in `tests_parity.rs` could not catch it: they always pass an
//! explicit ABSOLUTE `state_dir`, which is precisely the case that works.

use std::path::{Path, PathBuf};

/// Resolve the state directory for a tool invocation.
///
/// `config_path` is the `path` argument the tool was called with (the
/// forjar.yaml). `state_dir` is the caller's optional override.
///
/// Rules, in order:
/// 1. an absolute `state_dir` is honoured verbatim — callers who say exactly
///    where state lives are always obeyed;
/// 2. a relative `state_dir` is resolved against the CONFIG's directory, not the
///    process cwd, because it names a location within the project;
/// 3. no `state_dir` defaults to `<config dir>/state`, matching what the CLI
///    reaches when run from the project directory.
///
/// A config path with no parent (a bare `forjar.yaml`) yields `state`, which is
/// the historical behaviour and correct when cwd already is the project.
pub fn resolve_state_dir(config_path: &Path, state_dir: Option<&str>) -> PathBuf {
    let base = config_path.parent().unwrap_or_else(|| Path::new(""));
    match state_dir {
        Some(s) => {
            let p = Path::new(s);
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                base.join(p)
            }
        }
        None => base.join("state"),
    }
}

/// Same as [`resolve_state_dir`] but for tools whose config `path` is optional.
///
/// GH-208: `forjar_status`, `forjar_trace` and `forjar_anomaly` shipped with no
/// `path` field at all, so they had no way to express WHICH project they were
/// being asked about — they always read `./state` from the server's cwd. `path`
/// is now accepted (optional, so existing callers keep working); when it is
/// absent the historical cwd-relative behaviour is preserved.
pub fn resolve_state_dir_opt(config_path: Option<&str>, state_dir: Option<&str>) -> PathBuf {
    match config_path {
        Some(cp) => resolve_state_dir(Path::new(cp), state_dir),
        None => PathBuf::from(state_dir.unwrap_or("state")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_beside_the_config_not_the_cwd() {
        assert_eq!(
            resolve_state_dir(Path::new("/tmp/proj/forjar.yaml"), None),
            PathBuf::from("/tmp/proj/state"),
            "the regression: an absolute config path must not have its state \
             looked for in the server's cwd (GH-208)"
        );
    }

    #[test]
    fn absolute_override_is_honoured_verbatim() {
        assert_eq!(
            resolve_state_dir(Path::new("/tmp/proj/forjar.yaml"), Some("/var/lib/st")),
            PathBuf::from("/var/lib/st")
        );
    }

    #[test]
    fn relative_override_resolves_against_the_config_dir() {
        assert_eq!(
            resolve_state_dir(Path::new("/tmp/proj/forjar.yaml"), Some("alt-state")),
            PathBuf::from("/tmp/proj/alt-state"),
            "a relative override names a place inside the project"
        );
    }

    #[test]
    fn bare_config_name_keeps_historical_behaviour() {
        assert_eq!(
            resolve_state_dir(Path::new("forjar.yaml"), None),
            PathBuf::from("state")
        );
    }

    #[test]
    fn nested_relative_override() {
        assert_eq!(
            resolve_state_dir(Path::new("/a/b/forjar.yaml"), Some("x/y")),
            PathBuf::from("/a/b/x/y")
        );
    }
}
