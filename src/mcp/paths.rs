//! GH-208 / #367: state-directory resolution for MCP tools.
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
//!
//! # The second door (paiml/forjar#367)
//!
//! Fixing the base left the WORKSPACE. `forjar workspace select prod` moves
//! where the CLI reads and writes — `cli::workspace::resolve_state_dir` joins
//! the active workspace onto its state dir — and this module did not, so the
//! same failure mode reopened the moment anyone ran `forjar workspace new`.
//! Measured on 1.24.0, one project, `.forjar/workspace = prod`, state applied
//! under `state/prod/`:
//!
//! ```text
//!   $ forjar plan -f forjar.yaml                            1 unchanged
//!   $ forjar verb call plan   '{"path":".../forjar.yaml"}'  to_create: 1
//!   $ wc -l state/prod/local/events.jsonl                   4
//!   $ forjar verb call audit  '{"path":".../forjar.yaml"}'  event_count: 0
//!   $ forjar verb call status '{"path":".../forjar.yaml"}'  machines: []
//! ```
//!
//! `to_create: 1`, `event_count: 0` and `machines: []` are exactly what an empty
//! project reports, so not one of the three carries a tell.
//!
//! [`resolve_state_dir`] therefore joins the selection — but ONLY on the default
//! branch. An explicit `state_dir` stays verbatim, because the documented
//! workaround for #367 was to hand the `workspace` verb's `workspace_state_dir`
//! back as the next verb's `state_dir`; joining onto that would resolve
//! `state/prod/prod` and break every caller who followed it. The CLI resolver
//! DOES join onto an explicit `--state-dir` — `forjar plan --state-dir state/prod`
//! inside the `prod` workspace resolves `state/prod/prod` — which is why this
//! module does not simply delegate to it.
//!
//! [`resolve_state_base`] is the unjoined half, and it is not a leftover: the
//! `workspace` verb has to ENUMERATE `state/` and performs its own
//! `join(active)`. Route that one through the joined resolver and its listing
//! becomes the MACHINE directories under the active workspace, reported as if
//! they were workspaces.

use std::path::{Path, PathBuf};

/// The state BASE for a tool invocation — the directory workspaces live under.
///
/// `config_path` is the `path` argument the tool was called with (the
/// forjar.yaml). `state_dir` is the caller's optional override.
///
/// Rules, in order:
/// 1. an absolute `state_dir` is honoured verbatim — callers who say exactly
///    where state lives are always obeyed;
/// 2. a relative `state_dir` is resolved against the CONFIG's directory, not the
///    process cwd, because it names a location within the project;
/// 3. no `state_dir` defaults to `<config dir>/state`.
///
/// A config path with no parent (a bare `forjar.yaml`) yields `state`, which is
/// the historical behaviour and correct when cwd already is the project.
///
/// This is what a caller that ENUMERATES or reports on workspaces wants. A
/// caller that wants to READ STATE wants [`resolve_state_dir`].
pub fn resolve_state_base(config_path: &Path, state_dir: Option<&str>) -> PathBuf {
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

/// The active workspace recorded beside the config, if it is a name worth joining.
///
/// `current_workspace_in`, never `current_workspace`: the latter hard-codes
/// `Path::new(".")`, which is the GH-208 cwd bug this module exists to avoid.
///
/// A malformed name falls back to the unjoined base rather than erroring.
/// Validation is not decoration — `workspace delete ../../victim` once removed a
/// tree outside the project — but this path only READS, so refusing to answer
/// would turn a stray marker file into an outage on a read-only surface.
fn active_workspace(project_root: &Path) -> Option<String> {
    crate::cli::workspace::current_workspace_in(project_root)
        .filter(|ws| crate::cli::workspace::validate_workspace_name(ws).is_ok())
}

/// Resolve the state directory a tool should READ.
///
/// [`resolve_state_base`] plus the active workspace, joined ONLY when the caller
/// named no `state_dir` — see the module header for why an explicit override is
/// never joined onto.
pub fn resolve_state_dir(config_path: &Path, state_dir: Option<&str>) -> PathBuf {
    let base = resolve_state_base(config_path, state_dir);
    if state_dir.is_some() {
        return base;
    }
    let project_root = config_path.parent().unwrap_or_else(|| Path::new(""));
    match active_workspace(project_root) {
        Some(ws) => base.join(ws),
        None => base,
    }
}

/// Same as [`resolve_state_base`] but for tools whose config `path` is optional.
///
/// GH-208: `forjar_status`, `forjar_trace` and `forjar_anomaly` shipped with no
/// `path` field at all, so they had no way to express WHICH project they were
/// being asked about — they always read `./state` from the server's cwd. `path`
/// is now accepted (optional, so existing callers keep working); when it is
/// absent the historical cwd-relative behaviour is preserved.
pub fn resolve_state_base_opt(config_path: Option<&str>, state_dir: Option<&str>) -> PathBuf {
    match config_path {
        Some(cp) => resolve_state_base(Path::new(cp), state_dir),
        None => PathBuf::from(state_dir.unwrap_or("state")),
    }
}

/// Same as [`resolve_state_dir`] but for tools whose config `path` is optional.
///
/// With no `path` there is no project directory to look for `.forjar/workspace`
/// beside, and the cwd fallback is the known-bad GH-208 case; joining a
/// cwd-derived selection onto a cwd-derived base would deepen the guess rather
/// than answer it. So that branch is left exactly as it was.
pub fn resolve_state_dir_opt(config_path: Option<&str>, state_dir: Option<&str>) -> PathBuf {
    match config_path {
        Some(cp) => resolve_state_dir(Path::new(cp), state_dir),
        None => PathBuf::from(state_dir.unwrap_or("state")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every case below runs inside a tempdir rather than against literal
    /// `/tmp/proj`. Once resolution consults `.forjar/workspace` beside the
    /// config, a real `/tmp/proj/.forjar/workspace` on somebody's box — and
    /// `/tmp/proj` is exactly the kind of path that gets created by hand —
    /// would flip these assertions for a reason that has nothing to do with
    /// the code.
    fn project() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    fn select(root: &Path, name: &str) {
        std::fs::create_dir_all(root.join(".forjar")).expect("mkdir");
        std::fs::write(root.join(".forjar").join("workspace"), name).expect("write");
    }

    #[test]
    fn defaults_beside_the_config_not_the_cwd() {
        let d = project();
        assert_eq!(
            resolve_state_dir(&d.path().join("forjar.yaml"), None),
            d.path().join("state"),
            "the regression: an absolute config path must not have its state \
             looked for in the server's cwd (GH-208)"
        );
    }

    #[test]
    fn absolute_override_is_honoured_verbatim() {
        let d = project();
        assert_eq!(
            resolve_state_dir(&d.path().join("forjar.yaml"), Some("/var/lib/st")),
            PathBuf::from("/var/lib/st")
        );
    }

    #[test]
    fn relative_override_resolves_against_the_config_dir() {
        let d = project();
        assert_eq!(
            resolve_state_dir(&d.path().join("forjar.yaml"), Some("alt-state")),
            d.path().join("alt-state"),
            "a relative override names a place inside the project"
        );
    }

    #[test]
    fn bare_config_name_keeps_historical_behaviour() {
        assert_eq!(
            resolve_state_base(Path::new("forjar.yaml"), None),
            PathBuf::from("state")
        );
    }

    #[test]
    fn nested_relative_override() {
        let d = project();
        assert_eq!(
            resolve_state_dir(&d.path().join("b").join("forjar.yaml"), Some("x/y")),
            d.path().join("b").join("x").join("y")
        );
    }

    // ── #367 ────────────────────────────────────────────────────────

    #[test]
    fn the_default_follows_the_workspace_selection() {
        let d = project();
        select(d.path(), "prod");
        assert_eq!(
            resolve_state_dir(&d.path().join("forjar.yaml"), None),
            d.path().join("state").join("prod"),
            "the selection the CLI honours was ignored, so a converged \
             resource reads back as CREATE (paiml/forjar#367)"
        );
    }

    #[test]
    fn an_explicit_state_dir_is_never_joined_onto() {
        let d = project();
        select(d.path(), "prod");
        let designated = d.path().join("state").join("prod");
        assert_eq!(
            resolve_state_dir(
                &d.path().join("forjar.yaml"),
                Some(&designated.display().to_string())
            ),
            designated,
            "handing `workspace_state_dir` back as `state_dir` — the documented \
             workaround — resolved state/prod/prod"
        );
    }

    #[test]
    fn the_state_base_never_follows_the_selection() {
        let d = project();
        select(d.path(), "prod");
        assert_eq!(
            resolve_state_base(&d.path().join("forjar.yaml"), None),
            d.path().join("state"),
            "the `workspace` verb enumerates this directory; joining the \
             selection here lists machine dirs as workspaces"
        );
    }

    #[test]
    fn a_workspace_name_that_is_a_path_is_refused_not_joined() {
        let d = project();
        select(d.path(), "../../victim");
        assert_eq!(
            resolve_state_dir(&d.path().join("forjar.yaml"), None),
            d.path().join("state"),
            "a marker naming a traversal must fall back to the unjoined base, \
             never be joined (GH-208 deleted a tree outside the project this way)"
        );
    }

    #[test]
    fn no_marker_means_the_bare_state_dir() {
        let d = project();
        std::fs::create_dir_all(d.path().join(".forjar")).unwrap();
        std::fs::write(d.path().join(".forjar").join("workspace"), "  \n").unwrap();
        assert_eq!(
            resolve_state_dir(&d.path().join("forjar.yaml"), None),
            d.path().join("state"),
            "an empty marker is no selection, not a selection called \"\""
        );
    }

    #[test]
    fn the_opt_form_with_no_path_is_unchanged() {
        assert_eq!(
            resolve_state_dir_opt(None, None),
            PathBuf::from("state"),
            "with no config there is no project dir to look beside; joining a \
             cwd-derived selection onto a cwd-derived base deepens the guess"
        );
        assert_eq!(
            resolve_state_dir_opt(None, Some("/var/lib/st")),
            PathBuf::from("/var/lib/st")
        );
    }
}
