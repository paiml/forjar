//! `lint --fix`: the auto-fixers, and what they refuse.
//!
//! Split out of `lint.rs` when combining the #356 gate work with #359's
//! comment-preserving rewrite took that file past the repo's 500-line ceiling.
//! These belong together: `AutoFix` is the shape both halves report through,
//! and `sort_resources` is the only transformation that currently produces one.

use std::path::Path;

/// What `lint --fix` actually did, and what it refused to do.
///
/// paiml/forjar#359: the previous shape was a bare `Vec<String>` of "fixes
/// applied", and the one entry it could hold was pushed UNCONDITIONALLY —
/// whenever a `resources:` mapping existed, sorted or not. So `--fix` claimed
/// "sorted resource keys alphabetically" on an already-sorted file, and
/// rewrote the file to prove it. Separating what was applied from what was
/// refused is what makes both halves reportable without one lying about the
/// other.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct AutoFix {
    /// Transformations that changed the file. Empty means the file was not
    /// written.
    pub applied: Vec<String>,
    /// Transformations forjar declined, each carrying why.
    pub refused: Vec<String>,
}

/// The result of trying to sort the `resources:` mapping in place.
enum SortOutcome {
    /// Nothing to do: no `resources:` mapping, or its keys are already sorted.
    Unchanged,
    /// The document with the mapping's entries reordered and nothing else
    /// touched.
    Sorted(String),
    /// The reorder could not be proven sound. Carries the reason.
    Refused(String),
}

/// Sort the entries of `resources:` by moving their source byte ranges.
///
/// paiml/forjar#359: this used to parse the whole document into
/// `serde_yaml_ng::Value`, rebuild the `resources` mapping in sorted order and
/// re-emit the file. `Value` does not carry comments, so every comment in the
/// user's config was deleted — silently, by a flag whose contract was to fix
/// lint findings. In an IaC config the comments are the operational reasoning:
/// why a host is pinned, why an ordering matters, which runbook depends on it.
///
/// Sorting does not need a re-serialisation. Each entry owns a contiguous run
/// of source lines, so this is a permutation of byte ranges: no byte is
/// rewritten, only moved, and a comment above an entry travels with it.
fn sort_resources(content: &str) -> SortOutcome {
    use crate::core::yaml_edit::{blocks, verify, AnchorError};

    let blocks = match blocks::key_blocks(content, &["resources"]) {
        Ok(b) => b,
        // No `resources:` mapping at all is not a refusal — there is nothing
        // to sort, which is exactly the same outcome as "already sorted".
        Err(AnchorError::NotFound) => return SortOutcome::Unchanged,
        Err(e) => return SortOutcome::Refused(e.reason().to_string()),
    };
    if blocks::is_sorted(&blocks) {
        return SortOutcome::Unchanged;
    }
    let sorted = match blocks::reorder(content, &blocks, &blocks::sorted_order(&blocks)) {
        Ok(text) => text,
        Err(e) => return SortOutcome::Refused(e.reason().to_string()),
    };
    // Fail closed. Reordering entries must change no value anywhere in the
    // document; if the re-parse disagrees, the edit is discarded rather than
    // written.
    match verify::changed_paths_of_text(content, &sorted) {
        Ok(changed) if changed.is_empty() => SortOutcome::Sorted(sorted),
        Ok(_) => {
            SortOutcome::Refused("the reorder changed a value, so it was discarded".to_string())
        }
        Err(e) => SortOutcome::Refused(e),
    }
}

/// Apply every auto-fix `lint --fix` knows, writing the file only if one of
/// them actually changed something.
pub(crate) fn lint_auto_fix(file: &Path) -> Result<AutoFix, String> {
    let content = std::fs::read_to_string(file)
        .map_err(|e| format!("cannot read {}: {}", file.display(), e))?;
    // Fail closed before touching anything: an auto-fixer must never rewrite a
    // document it cannot parse. `cmd_lint` parses the config before it gets
    // here, so this guard is for every other caller.
    serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&content)
        .map_err(|e| format!("YAML parse error: {e}"))?;
    let mut out = AutoFix::default();
    match sort_resources(&content) {
        SortOutcome::Unchanged => {}
        SortOutcome::Sorted(sorted) => {
            std::fs::write(file, &sorted)
                .map_err(|e| format!("cannot write {}: {}", file.display(), e))?;
            out.applied
                .push("sorted resource keys alphabetically".to_string());
        }
        SortOutcome::Refused(reason) => out
            .refused
            .push(format!("resource keys left unsorted: {reason}")),
    }
    Ok(out)
}
