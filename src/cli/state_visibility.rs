//! Making the resolved state directory visible (GH-273).
//!
//! `--state-dir` defaults to the literal `state`, which is CWD-RELATIVE. The
//! same command naming the same config by the same `-f` path therefore answers
//! differently depending on where you stand — and the difference is silent,
//! because `state::load_lock` returns None for "no such lock" with no comment.
//!
//! Measured in paiml/infra: `plan -f machines/intel/forjar.yaml` reported
//! `100 unchanged` from the repo root and `101 to add` from a git worktree of
//! the same commit, against a host provably converged and serving 16 runners.
//!
//! The default is deliberately NOT changed. Making it config-relative would
//! silently relocate that repo's state (config at `machines/<m>/forjar.yaml`,
//! state at the repo root) and break every existing caller. The defect is that
//! the choice is invisible, so the fix is to make it visible.

use std::path::Path;

/// Declared machines that have no lock in the resolved state directory.
///
/// Exact by construction: a machine appears iff it was declared and no lock was
/// loaded for it. Callers use emptiness to decide whether to warn, so a false
/// positive here is a warning that always fires — and a warning that always
/// fires is one people learn to ignore.
pub fn machines_missing_state<'a, I, J>(declared: I, with_locks: J) -> Vec<&'a str>
where
    I: IntoIterator<Item = &'a str>,
    J: IntoIterator<Item = &'a str>,
{
    let have: std::collections::BTreeSet<&str> = with_locks.into_iter().collect();
    declared.into_iter().filter(|m| !have.contains(m)).collect()
}

/// One line naming where state was read from, and which machines had none.
///
/// Printed unconditionally so a reader seeing "N to add" can always tell a
/// drifted host from a wrong working directory. The absolute path is resolved
/// because the whole failure mode is a relative path meaning two things.
pub fn describe(state_dir: &Path, missing: &[&str]) -> String {
    let resolved = std::fs::canonicalize(state_dir)
        .unwrap_or_else(|_| {
            std::env::current_dir()
                .map(|c| c.join(state_dir))
                .unwrap_or_else(|_| state_dir.to_path_buf())
        })
        .display()
        .to_string();
    if missing.is_empty() {
        format!("state: {resolved}")
    } else {
        format!(
            "state: {resolved} (no prior state for: {})",
            missing.join(", ")
        )
    }
}

/// Print where state was read from, and which declared machines had none.
///
/// `--state-dir` defaults to a CWD-relative `state`, so the same command
/// against the same config answers differently depending on the directory it
/// runs from — silently, because a missing lock is indistinguishable from a
/// converged one in the summary.
pub fn report<L>(
    state_dir: &Path,
    config: &crate::core::types::ForjarConfig,
    locks: &std::collections::HashMap<String, L>,
) {
    let declared: Vec<&str> = config.machines.keys().map(String::as_str).collect();
    let have: Vec<&str> = locks.keys().map(String::as_str).collect();
    let missing = machines_missing_state(declared, have);
    eprintln!("{}", describe(state_dir, &missing));
}

/// Set-difference specification for [`machines_missing_state`], as a bitmask.
///
/// Bit i means "machine i". This is the allocation-free form Kani can address
/// directly (KANI-SV-001): proving over the `&str` version would mean reasoning
/// through a BTreeSet of heap strings, and CBMC models every path through
/// those — this repo has already had a harness run 117 minutes and be killed
/// on exactly that shape.
///
/// A spec is only worth proving if the implementation is bound to it, so
/// `spec_agrees_with_implementation` in the tests below checks the two agree
/// over every subset pair up to 6 machines. The proof establishes the logic;
/// the differential test establishes that the code implements THAT logic.
// Referenced by KANI-SV-001 and by the differential test that binds the
// implementation to it; neither is a normal build, hence the allow.
#[cfg_attr(not(any(test, kani)), allow(dead_code))]
pub const fn missing_mask(declared: u8, with_locks: u8) -> u8 {
    declared & !with_locks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(mask: u8, n: u8) -> Vec<String> {
        (0..n)
            .filter(|i| mask & (1 << i) != 0)
            .map(|i| format!("m{i}"))
            .collect()
    }

    /// Binds the implementation to the proved spec. Without this, KANI-SV-001
    /// would prove a bitmask nobody calls.
    #[test]
    fn spec_agrees_with_implementation() {
        const N: u8 = 6;
        for declared in 0u16..(1 << N) {
            for have in 0u16..(1 << N) {
                let (d, h) = (declared as u8, have as u8);
                // only lock-holders that were actually declared are meaningful
                let h = h & d;
                let decl = names(d, N);
                let hav = names(h, N);
                let got = machines_missing_state(
                    decl.iter().map(String::as_str),
                    hav.iter().map(String::as_str),
                );
                let want = names(missing_mask(d, h), N);
                assert_eq!(
                    got, want,
                    "implementation diverged from the proved spec at declared={d:#b} have={h:#b}"
                );
            }
        }
    }

    #[test]
    fn describe_names_missing_machines() {
        let out = describe(std::path::Path::new("."), &["intel"]);
        assert!(out.contains("no prior state for: intel"), "{out}");
    }

    #[test]
    fn describe_is_quiet_when_nothing_is_missing() {
        let out = describe(std::path::Path::new("."), &[]);
        assert!(!out.contains("no prior state"), "{out}");
    }
}
