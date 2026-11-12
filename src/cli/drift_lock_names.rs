//! forjar#585: a drift verdict names the lock file(s) it was read from.
//!
//! With more than one lock tree for one machine, "the same" command at the same
//! commit gives different counts, and nothing in the output said why.

use crate::core::types::StateLock;
use std::path::{Path, PathBuf};

fn absolute(p: PathBuf) -> PathBuf {
    std::fs::canonicalize(&p)
        .unwrap_or_else(|_| std::env::current_dir().map(|c| c.join(&p)).unwrap_or(p))
}

/// Resolved absolute lock path for every machine whose lock was loaded.
pub(super) fn resolved_lock_files(state_dir: &Path, locks: &[(String, StateLock)]) -> Vec<String> {
    locks
        .iter()
        .map(|(m, _)| {
            absolute(crate::core::state::lock_file_path(state_dir, m))
                .display()
                .to_string()
        })
        .collect()
}

/// The one line a text verdict prints.
pub(super) fn describe(files: &[String]) -> String {
    if files.is_empty() {
        "  lock: none found".to_string()
    } else {
        format!("  lock: {}", files.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_each_lock_and_says_when_none() {
        assert_eq!(describe(&[]), "  lock: none found");
        let d = describe(&["/a/m/state.lock.yaml".into(), "/b/m/state.lock.yaml".into()]);
        assert!(d.contains("/a/m/state.lock.yaml") && d.contains("/b/m/state.lock.yaml"));
    }

    #[test]
    fn two_state_dirs_give_different_paths() {
        let a = absolute(PathBuf::from("/nonexistent-a/m/state.lock.yaml"));
        let b = absolute(PathBuf::from("/nonexistent-b/m/state.lock.yaml"));
        assert_ne!(
            describe(&[a.display().to_string()]),
            describe(&[b.display().to_string()])
        );
    }
}
