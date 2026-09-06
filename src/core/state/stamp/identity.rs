//! PMAT-161 (PMAT-175): how a stamp records the config it was applied from,
//! and how that record is compared with the config in hand.
//!
//! Split out of the parent for the 500-line ceiling, and the split is also the
//! honest boundary: everything here is about PATHS, and the parent is about
//! stacks. The one rule the whole file exists to state is that the two sides of
//! the comparison are normalised THE SAME WAY — a recorded path is whatever
//! [`stamped_config_file`] wrote, so the question "is this the same file?" is
//! answered by writing it again and comparing.

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
/// `state_dir` is `None` for the callers that ask about a lock without saying
/// which dir it came from ([`stack_conflict`] and its two callers). A relative
/// recorded path cannot be resolved without that base, so those keep the old
/// tail comparison — see [`unattributed_match`] for what that costs and why it
/// is not tightened here.
pub(super) fn same_config_file(
    recorded: &str,
    state_dir: Option<&Path>,
    config_file: &Path,
) -> bool {
    let Some(state_dir) = state_dir else {
        return unattributed_match(recorded, config_file);
    };
    stamped_config_file(state_dir, Some(config_file)).is_some_and(|now| now == recorded)
}

/// The comparison available to a caller that does not know the state dir the
/// recorded path is relative to: the absolute form exactly, and a relative form
/// by the part it names.
///
/// FAIL-OPEN, deliberately and narrowly. Its two readers ([`machine_owner`] and
/// [`stack_written_from_other_file`], through [`stack_conflict`]) use a match as
/// an EXEMPTION from a warning, so being loose here misses a warning rather than
/// deleting a stamp; being strict instead would refuse every ordinary apply,
/// whose recorded path is relative and cannot be resolved from here. The
/// destructive reader, [`rename::retire_renamed`], always has the state dir and
/// therefore never lands in this branch.
fn unattributed_match(recorded: &str, config_file: &Path) -> bool {
    let recorded = Path::new(recorded);
    let current = canonical(config_file);
    if recorded == current {
        return true;
    }
    let named: PathBuf = recorded
        .components()
        .skip_while(|c| matches!(c, std::path::Component::ParentDir))
        .collect();
    !named.as_os_str().is_empty() && recorded.is_relative() && current.ends_with(&named)
}

/// Does this stamp record the config file now being applied?
///
/// A stamp with NO recorded file never matches: "origin unknown" is not
/// evidence of anything, and the one caller that acts destructively on a match
/// ([`rename::retire_renamed`]) would otherwise retire a 1.0 migration's stamp.
pub(super) fn records_config_file(
    stamp: &StackStamp,
    state_dir: Option<&Path>,
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
