//! GH-377: refuse to join one stack's config to another stack's state dir.
//!
//! THE DEFECT. `-f/--file` defaults to `forjar.yaml` in the CWD while
//! `--state-dir` is a separate argument, so the two can name different stacks
//! and nothing noticed. Measured on 1.22.0: run from a directory holding stack
//! B's config, `forjar undo --state-dir <stack A's state>` exited 0, diffed
//! stack A's generations (printing `~ demo_file (box): will be updated`, a
//! resource B does not declare), then converged stack B's resources against the
//! host and re-stamped A's `forjar.lock.yaml` as `name: stack-bravo` — erasing
//! the only evidence that the dirs had ever belonged to different stacks.
//!
//! The plan shown and the work done were about different stacks. That is what
//! makes `undo` a refusal rather than a warning: `apply` at least does exactly
//! what its two arguments say.
//!
//! THE SIGNAL. `state/forjar.lock.yaml` carries `name:`, copied verbatim from
//! the config's top-level `name` on every apply (`state::update_global_lock`).
//! `name` is required — parsing fails without it — never templated, never
//! param-substituted, and untouched by include merges. It is the only stable
//! stack identity the state dir records, and it is already there.
//!
//! FAIL-OPEN IS LOAD-BEARING, NOT LAZINESS. A state dir with no readable
//! `forjar.lock.yaml` is reachable in normal use: `forjar rollback
//! --generation 0 --yes` restores a generation that predates the first global
//! lock and leaves the state dir without one. Refusing there would brick a
//! state dir that works today, so absence, unreadability and an empty name all
//! ALLOW; only a name that is present and different refuses.

use crate::core::{state, types};
use std::path::{Path, PathBuf};

/// Absolute path when it can be resolved. `-f` defaults to the relative
/// `forjar.yaml`, and printing that bare name is what kept the defect invisible.
fn shown(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

/// Refuse when `state_dir` was last applied by a stack other than `config`.
///
/// `verb` names the command in the message ("undo", "undo --resume").
pub(super) fn check_state_dir_owner(
    verb: &str,
    config: &types::ForjarConfig,
    file: &Path,
    state_dir: &Path,
) -> Result<(), String> {
    let Ok(Some(lock)) = state::load_global_lock(state_dir) else {
        return Ok(());
    };
    if lock.name.is_empty() || lock.name == config.name {
        return Ok(());
    }
    // The name in the lock is not the only name this state dir has answered to.
    //
    // `undo` replays a generation's RECORDED config, and that replay stamps the
    // global lock with the name the stack carried BACK THEN. So undoing across
    // a rename rewrites the lock to the historical name, and the next undo
    // compared the operator's current name against it and refused — one config,
    // one state dir, one operator, no mistake. Worse, the remedy this printed
    // ("run `forjar apply` once to re-stamp") converges the host FORWARD,
    // destroying the very undo in progress.
    //
    // A renamed stack's own history is recorded in its generations; a genuinely
    // foreign stack's is not. So accept a name this state dir has applied under
    // before, and keep refusing everything else.
    if applied_under_before(state_dir, &config.name) {
        return Ok(());
    }
    Err(mismatch_error(
        verb,
        &config.name,
        &lock.name,
        file,
        state_dir,
    ))
}

/// The refusal. It names both stacks, both absolute paths, what would have
/// happened, and the two ways out — including the one that is not a mistake at
/// all, a stack that was renamed.
fn mismatch_error(
    verb: &str,
    config_name: &str,
    owner: &str,
    file: &Path,
    state_dir: &Path,
) -> String {
    format!(
        "refusing to {verb}: --state-dir belongs to a different stack.\n  \
         config:    '{config_name}' ({})\n  \
         state dir: '{owner}' ({})\n\
         The generations there record what '{owner}' applied, but {verb} would re-converge \
         the host from '{config_name}' — every resource '{config_name}' declares would be \
         applied against state that does not describe it.\n\
         Point -f at the config that owns that state, or --state-dir at the state that \
         belongs to this config. If '{config_name}' is '{owner}' renamed, run \
         `forjar apply` once to re-stamp the state dir, then {verb} again",
        shown(file).display(),
        shown(state_dir).display(),
    )
}

/// Has this state dir ever been applied under `name`?
///
/// Reads the stack name out of each generation's recorded config. Only the name
/// is needed, so the document is parsed as loose YAML rather than a full
/// `ForjarConfig` — a generation recorded by an older or newer forjar must not
/// be able to turn this into a refusal.
fn applied_under_before(state_dir: &Path, name: &str) -> bool {
    let gens = state_dir.join("generations");
    let Ok(entries) = std::fs::read_dir(&gens) else {
        return false;
    };
    entries.flatten().any(|e| {
        std::fs::read_to_string(e.path().join(super::undo_replay::APPLIED_CONFIG))
            .ok()
            .and_then(|s| serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&s).ok())
            .and_then(|v| v.get("name")?.as_str().map(str::to_string))
            .is_some_and(|n| n == name)
    })
}
