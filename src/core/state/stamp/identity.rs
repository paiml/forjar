//! PMAT-161 (PMAT-175): how a stamp records the config it was applied from,
//! and how that record is compared with the config in hand.
//!
//! Split out of the parent for the 500-line ceiling, and the split is also the
//! honest boundary: everything here is about PATHS, and the parent is about
//! stacks. The one rule the whole file exists to state is that the two sides of
//! the comparison are normalised THE SAME WAY — a recorded path is whatever
//! [`stamped_config_file`] wrote, so the question "is this the same file?" is
//! answered by writing it again and comparing.
//!
//! PMAT-183: ONE comparison, and it is that one. PMAT-175 tightened the reader
//! that deletes on a match and left a looser sibling for the reader that only
//! warns ([`super::stack_conflict`], which was called without a state dir) —
//! documented as an accepted limit. It was not one: `../forjar.yaml` still
//! matched every `machines/<m>/forjar.yaml`, so the wrong-stack guard was
//! silent on the same name applied from a different file, which is the case it
//! exists for. Every reader supplies the state dir now, so the tail comparison
//! is gone rather than narrowed.

use super::StackStamp;
use std::path::{Path, PathBuf};

/// Canonicalise a path, falling back to the path as given.
///
/// The config may have moved since the apply that recorded it; comparing two
/// uncanonicalised paths still beats claiming they differ.
fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// The path relative to `base`, or `None` when the two share no prefix.
fn relative_to(base: &Path, target: &Path) -> Option<PathBuf> {
    let base: Vec<_> = base.components().collect();
    let target: Vec<_> = target.components().collect();
    let shared = base
        .iter()
        .zip(target.iter())
        .take_while(|(a, b)| a == b)
        .count();
    // One shared component is the filesystem root: no common tree.
    if shared <= 1 {
        return None;
    }
    let mut relative = PathBuf::new();
    for _ in shared..base.len() {
        relative.push("..");
    }
    for component in &target[shared..] {
        relative.push(component);
    }
    Some(relative)
}

/// How a config path is stored in a stamp: relative to the state dir when both
/// sit in one tree, else the absolute canonical path.
#[must_use]
pub fn stamped_config_file(state_dir: &Path, file: Option<&Path>) -> Option<String> {
    let config = canonical(file?);
    let base = canonical(state_dir);
    let stored = relative_to(&base, &config).unwrap_or(config);
    Some(stored.display().to_string())
}

/// Does a recorded stamp path name the config now being applied?
///
/// PMAT-175: EXACT, in the one frame both sides are written in. The recorded
/// string is whatever [`stamped_config_file`] produced for this state dir, so
/// the question "is this the same file?" is answered by producing it again for
/// the config in hand and comparing the two strings. Nothing is skipped and
/// nothing is matched by suffix.
///
/// It was a suffix match — the `..` components were dropped and the current
/// path was asked whether it ENDS WITH the rest — and that is not a weaker
/// version of the same test, it is a different one. `../forjar.yaml` matched
/// EVERY `machines/<m>/forjar.yaml` in the paiml/infra layout, so the first
/// machine manifest applied into the root stack's dir was read as that stack
/// renamed and [`rename::retire_renamed`] DELETED the root stack's stamp,
/// machines and output keys included (executed reproducer, review lane).
///
/// PMAT-183: and there is no second comparison left to fall back to.
/// `state_dir` used to be an option, `None` meaning "the caller does not know
/// which dir the lock came from" — which was true of exactly one reader,
/// [`super::stack_conflict`], and answered with a match by the tail of the
/// path. So the guard whose job is "one of `-f`/`--state-dir` points at the
/// wrong stack" kept the very defect the rename path had been cured of, and
/// kept it SILENTLY, because a loose match there exempts a warning rather than
/// deleting anything. Both of that guard's callers hold the dir already, so it
/// is a `&Path` here and a future reader cannot ask the question without
/// saying which dir it is about.
///
/// A stamp with NO recorded file — the 1.0 migration's one-apply window — is
/// answered before this is reached: [`records_config_file`] and
/// [`super::stack_written_from_other_file`] both return early on `file: None`,
/// so "origin unknown" is still not evidence of anything.
pub(super) fn same_config_file(recorded: &str, state_dir: &Path, config_file: &Path) -> bool {
    stamped_config_file(state_dir, Some(config_file)).is_some_and(|now| now == recorded)
}

/// Does this stamp record the config file now being applied?
///
/// A stamp with NO recorded file never matches: "origin unknown" is not
/// evidence of anything, and the one caller that acts destructively on a match
/// ([`rename::retire_renamed`]) would otherwise retire a 1.0 migration's stamp.
pub(super) fn records_config_file(
    stamp: &StackStamp,
    state_dir: &Path,
    config_file: Option<&Path>,
) -> bool {
    let Some(current) = config_file else {
        return false;
    };
    stamp
        .file
        .as_deref()
        .is_some_and(|recorded| same_config_file(recorded, state_dir, current))
}
