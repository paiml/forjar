//! PMAT-161 (PMAT-176): the machines a stack's stamp is allowed to claim.
//!
//! `stacks[name].machines` is an OWNERSHIP record: [`super::stack_conflict`]
//! reads it to decide whether another stack is about to write a machine this
//! one owns, because generations and per-machine locks are keyed by machine
//! name alone. It was written from the apply's own results, and a SCOPED apply
//! (`apply -m mini`) produces results for one machine. So one narrow invocation
//! rewrote `machines` as `[mini]` and RELEASED every other machine of the stack
//! — to the next sibling stack that named one, silently, because releasing is
//! exactly how `stack_conflict` is told a machine is free.
//!
//! A machine is released when the CONFIG stops declaring it. That is an edit an
//! operator made and can see; `-m` is a filter on this run.
//!
//! The declared set arrives as a thread-local scope, like
//! [`super::replay::StampOverride`] and `apply_snapshot::PauseGenerationRecording`,
//! rather than as another argument through `cmd_apply` → `update_global_lock` →
//! `apply_stamp`, and it is read at the one choke point every stamp passes
//! through. Unset — every unit caller, and any future caller that has no config
//! in hand — means "no claim about what is declared", and the stamp is written
//! exactly as it was before this module existed.

use std::cell::RefCell;

thread_local! {
    /// `Some` only inside [`DeclaredMachines`]'s scope.
    static DECLARED: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

/// Scope guard: the stamp written underneath it keeps the machines this config
/// declares, and only those.
///
/// Restores the previous value on drop — error paths and nesting included — so
/// one apply's declaration cannot leak into the next.
pub struct DeclaredMachines {
    previous: Option<Vec<String>>,
}

impl DeclaredMachines {
    /// Declare the machines of the config being applied.
    #[must_use]
    pub fn of(machines: &[String]) -> Self {
        let previous = DECLARED.with(|c| c.borrow_mut().replace(machines.to_vec()));
        Self { previous }
    }
}

impl Drop for DeclaredMachines {
    fn drop(&mut self) {
        let previous = self.previous.take();
        DECLARED.with(|c| *c.borrow_mut() = previous);
    }
}

/// The machines the config being applied declares, when a caller has said.
#[must_use]
pub(super) fn declared() -> Option<Vec<String>> {
    DECLARED.with(|c| c.borrow().clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(of: &[&str]) -> Vec<String> {
        of.iter().map(|s| (*s).to_string()).collect()
    }

    /// Unset is the ordinary unit caller: no claim, and the stamp is unchanged.
    #[test]
    fn without_the_guard_nothing_is_declared() {
        assert_eq!(declared(), None);
    }

    /// The scope is exactly the guard's lifetime, and it nests.
    #[test]
    fn the_guard_is_released_on_drop_and_nests() {
        {
            let _outer = DeclaredMachines::of(&names(&["mini", "lambda-labs"]));
            assert_eq!(declared(), Some(names(&["mini", "lambda-labs"])));
            {
                let _inner = DeclaredMachines::of(&names(&["clean-room"]));
                assert_eq!(declared(), Some(names(&["clean-room"])));
            }
            assert_eq!(
                declared(),
                Some(names(&["mini", "lambda-labs"])),
                "an inner scope must not outlive itself"
            );
        }
        assert_eq!(declared(), None, "and the outer scope releases too");
    }

    /// A config that declares no machines is a CLAIM (the empty one), not the
    /// absence of a claim: the stamp releases everything, because the config
    /// really does declare nothing.
    #[test]
    fn declaring_nothing_is_a_claim_not_an_absence() {
        let _guard = DeclaredMachines::of(&[]);
        assert_eq!(declared(), Some(Vec::new()));
    }
}
