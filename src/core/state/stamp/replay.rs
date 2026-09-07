//! PMAT-161 (S2, PMAT-172): who a REPLAYED apply stamps the state dir as.
//!
//! `undo` re-converges the host by applying the TARGET generation's RECORDED
//! config through the ordinary `cmd_apply`. That config is a document from the
//! past: its `name:` is whatever the stack was called then, and it is staged in
//! a hidden temp sibling of the operator's `-f` that is deleted seconds later.
//! Neither is this stack's identity. The apply underneath an `undo` is an
//! implementation detail of the command the operator ran, so it stamps as the
//! stack that ran it, with the `-f` that ran it.
//!
//! WHAT IT COST TO GET THIS WRONG. `rollback_to_generation` restores the target
//! generation's `forjar.lock.yaml` too, so the replay landed on a lock stamped
//! with the OLD name and wrote that name straight back: a stack renamed under
//! PMAT-171's one-lineage retirement was un-renamed by its own first undo. The
//! replay also withheld the `-f` (correctly — the staged path is a temp file),
//! and a stamp with no recorded file is not positive evidence of a rename, so
//! `retire_renamed` could not clean up afterwards either. `undo` then refused
//! the next restore, or demanded a re-stamping apply, for a stack that had only
//! ever been itself.
//!
//! The guard is a thread-local scope, like
//! `apply_snapshot::PauseGenerationRecording`, rather than another argument
//! threaded through `cmd_apply`, and it is read at the one choke point every
//! stamp goes through: [`super::apply_stamp`].

use std::cell::RefCell;
use std::path::{Path, PathBuf};

/// The identity an apply must stamp with while a replay is in flight.
#[derive(Debug, Clone)]
struct Invoking {
    /// The CURRENT config's `name:` — the stack the operator invoked.
    name: String,
    /// The operator's own `-f`, never the staged replay temp file.
    file: Option<PathBuf>,
}

thread_local! {
    /// `Some` only inside [`StampOverride`]'s scope; `None` for every ordinary
    /// apply, which is why no other caller changes behaviour.
    static INVOKING: RefCell<Option<Invoking>> = const { RefCell::new(None) };
}

/// Scope guard: the apply underneath it stamps the state dir as `name`, from
/// `file`, whatever the config it is applying happens to say.
///
/// Restores the previous value on drop — the error paths and a nested scope
/// included — so a replay cannot leak an identity into the apply after it.
pub struct StampOverride {
    previous: Option<Invoking>,
}

impl StampOverride {
    /// Stamp as the stack that invoked the command, not as the replayed config.
    #[must_use]
    pub fn invoking(name: &str, file: Option<&Path>) -> Self {
        let invoking = Invoking {
            name: name.to_string(),
            file: file.map(Path::to_path_buf),
        };
        let previous = INVOKING.with(|c| c.borrow_mut().replace(invoking));
        Self { previous }
    }
}

impl Drop for StampOverride {
    fn drop(&mut self) {
        let previous = self.previous.take();
        INVOKING.with(|c| *c.borrow_mut() = previous);
    }
}

/// The stack name this apply must be stamped under: the invoking stack while a
/// replay is in flight, else the config's own name.
#[must_use]
pub fn stamping_name(config_name: &str) -> String {
    INVOKING
        .with(|c| c.borrow().as_ref().map(|i| i.name.clone()))
        .unwrap_or_else(|| config_name.to_string())
}

/// The `-f` this apply must record: the invoking command's while a replay is in
/// flight, else the file it was handed.
///
/// The staged replay config is a temp sibling deleted on return, so recording
/// it would pin the stack to a path that no longer exists — the forjar#469
/// false positive re-entering through the one door the fix does not cover.
#[must_use]
pub fn stamping_file(config_file: Option<&Path>) -> Option<PathBuf> {
    match INVOKING.with(|c| c.borrow().clone()) {
        Some(invoking) => invoking.file,
        None => config_file.map(Path::to_path_buf),
    }
}

#[cfg(test)]
mod tests {
    use super::super::{apply_stamp, stamped_config_file};
    use super::*;
    use crate::core::state::new_global_lock;

    fn config_file(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(format!("{name}.yaml"));
        std::fs::write(&path, format!("version: '1.0'\nname: {name}\n")).unwrap();
        path
    }

    fn results() -> Vec<(String, usize, usize, usize)> {
        vec![("mini".to_string(), 1_usize, 1_usize, 0_usize)]
    }

    /// Unset — the ordinary apply — is the config's own identity, unchanged.
    #[test]
    fn without_the_guard_the_config_names_itself() {
        let dir = tempfile::tempdir().unwrap();
        let file = config_file(dir.path(), "alpha");
        assert_eq!(stamping_name("alpha"), "alpha");
        assert_eq!(stamping_file(Some(&file)), Some(file.clone()));
        assert_eq!(stamping_file(None), None);

        let mut lock = new_global_lock("alpha");
        apply_stamp(&mut lock, dir.path(), ("alpha", Some(&file)), &results());
        assert_eq!(lock.stacks.keys().collect::<Vec<_>>(), ["alpha"]);
        assert_eq!(lock.name, "alpha");
    }

    /// PMAT-172: inside the guard, `apply_stamp` writes the INVOKING stack's
    /// entry — and records the invoking `-f`, not the replayed config's.
    #[test]
    fn the_guard_stamps_the_invoking_stack_and_keeps_its_file() {
        let dir = tempfile::tempdir().unwrap();
        let real = config_file(dir.path(), "alpha");
        let staged = config_file(dir.path(), ".forjar-undo-gen0-1");
        let state = dir.path().join("state");
        // The lock the rollback restored: the generation's, stamped with the
        // name the stack carried THEN.
        let mut lock = new_global_lock("alpha-old");
        apply_stamp(&mut lock, &state, ("alpha-old", Some(&real)), &results());

        let _guard = StampOverride::invoking("alpha", Some(&real));
        assert_eq!(stamping_name("alpha-old"), "alpha");
        assert_eq!(stamping_file(Some(&staged)), Some(real.clone()));
        // The replayed config still says `alpha-old`; the stamp must not.
        apply_stamp(&mut lock, &state, ("alpha-old", Some(&staged)), &results());

        assert_eq!(
            lock.stacks.keys().collect::<Vec<_>>(),
            ["alpha"],
            "the replay must retire the historical name rather than stand beside \
             it — two stamps is what `multi_stack_restore_refusal` counts"
        );
        assert_eq!(
            lock.name, "alpha",
            "and the top-level name is the invoking one"
        );
        assert_eq!(
            lock.stamp_for("alpha").unwrap().file,
            stamped_config_file(&state, Some(&real)),
            "the recorded `-f` is the operator's, never the staged temp config"
        );
        assert_eq!(lock.stamp_for("alpha").unwrap().machines, ["mini"]);
    }

    /// The scope is exactly the guard's lifetime: dropping it restores the
    /// previous identity, so the next ordinary apply is untouched.
    #[test]
    fn the_guard_is_released_on_drop_and_nests() {
        let dir = tempfile::tempdir().unwrap();
        let outer = config_file(dir.path(), "outer");
        let inner = config_file(dir.path(), "inner");
        {
            let _o = StampOverride::invoking("outer", Some(&outer));
            {
                let _i = StampOverride::invoking("inner", Some(&inner));
                assert_eq!(stamping_name("x"), "inner");
                assert_eq!(stamping_file(None), Some(inner.clone()));
            }
            assert_eq!(stamping_name("x"), "outer");
            assert_eq!(stamping_file(None), Some(outer.clone()));
        }
        assert_eq!(stamping_name("x"), "x");
        assert_eq!(stamping_file(None), None);
    }
}
