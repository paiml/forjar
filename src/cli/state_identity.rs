//! GH-377 + forjar#469: refuse to join one stack's config to another stack's
//! state dir — without refusing the layout where N stacks legitimately share
//! one.
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
//! THE SIGNAL, forjar#469. The first guard compared the lock's single `name:`
//! against the config's. That name is written by whichever stack applied LAST,
//! so in the supported many-stacks-one-state-dir layout (paiml/infra: six
//! `machines/<m>/forjar.yaml`, one `state/`) it named a different stack on
//! every second apply and `undo` refused a layout `apply` supports.
//!
//! The identity is now the map `stacks: {name -> StackStamp}`, keyed exactly as
//! the lock's machine sections are, and this guard asks the SAME question
//! `apply` warns on — `state::stack_conflict` — so the two cannot disagree
//! about what "wrong stack" means. `undo` refuses where `apply` warns, plus one
//! case only `undo` has: a name this dir has no record of at all, whose
//! generations therefore belong to somebody else.
//!
//! FAIL-OPEN IS LOAD-BEARING, NOT LAZINESS. A state dir with no readable
//! `forjar.lock.yaml` is reachable in normal use: `forjar rollback
//! --generation 0 --yes` restores a generation that predates the first global
//! lock and leaves the state dir without one. Refusing there would brick a
//! state dir that works today, so absence, unreadability and a lock with no
//! stamps at all all ALLOW.

use crate::core::{state, types};
use std::path::{Path, PathBuf};

/// Absolute path when it can be resolved. `-f` defaults to the relative
/// `forjar.yaml`, and printing that bare name is what kept the defect invisible.
fn shown(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

/// The global lock, when this dir has one that parses. Absence, unreadability
/// and an unknown schema all yield `None`, and both guards below read that as
/// ALLOW — the fail-open half documented at the top of this module.
fn recorded_lock(state_dir: &Path) -> Option<types::GlobalLock> {
    state::load_global_lock(state_dir).ok().flatten()
}

/// Refuse when `state_dir`'s stamps say this config does not own what `undo`
/// would replay.
///
/// `verb` names the command in the message ("undo", "undo --resume").
pub(super) fn check_state_dir_owner(
    verb: &str,
    config: &types::ForjarConfig,
    file: &Path,
    state_dir: &Path,
) -> Result<(), String> {
    let Some(lock) = recorded_lock(state_dir) else {
        return Ok(());
    };
    if lock.stacks.is_empty() {
        return Ok(());
    }
    let machines: Vec<String> = config.machines.keys().cloned().collect();
    // The same condition `apply` warns on, asked of the same map: the same name
    // from a different `-f`, or a machine another stack owns.
    if let Some(conflict) = state::stack_conflict(&lock, &config.name, Some(file), &machines) {
        let detail = format!("stack '{}' {conflict}", config.name);
        return Err(refusal(verb, &config.name, &lock, &detail, file, state_dir));
    }
    if lock.stamp_for(&config.name).is_some() {
        return Ok(());
    }
    // The name in the stamps is not the only name this state dir has answered
    // to.
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
    let detail = format!(
        "state dir {} has no record of stack '{}' (it was applied by: {}), so {verb} would \
         replay another stack's generations against this config's resources.",
        shown(state_dir).display(),
        config.name,
        stack_names(&lock),
    );
    Err(refusal(verb, &config.name, &lock, &detail, file, state_dir))
}

/// The stacks this dir has stamps for, quoted, in the order they were recorded.
fn stack_names(lock: &types::GlobalLock) -> String {
    quoted(&state::stack_names(lock))
}

/// `'a', 'b'` — the stack list as every message in this module prints it.
fn quoted(names: &[String]) -> String {
    names
        .iter()
        .map(|n| format!("'{n}'"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The refusal. It names both sides, both absolute paths, what would have
/// happened, and the two ways out — including the one that is not a mistake at
/// all, a stack that was renamed.
///
/// `detail` is the sentence `apply` warns with, verbatim, so an operator who
/// read the warning recognises the refusal.
fn refusal(
    verb: &str,
    config_name: &str,
    lock: &types::GlobalLock,
    detail: &str,
    file: &Path,
    state_dir: &Path,
) -> String {
    format!(
        "refusing to {verb}: --state-dir belongs to a different stack.\n  \
         config:    '{config_name}' ({})\n  \
         state dir: {} ({})\n\
         {detail}\n\
         Point -f at the config that owns that state, or --state-dir at the state that \
         belongs to this config. If '{config_name}' is a stack this dir knows under another \
         name, run `forjar apply` once to re-stamp the state dir, then {verb} again",
        shown(file).display(),
        stack_names(lock),
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
