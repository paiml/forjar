//! PMAT-160 (#467): `apply --check` selects before it checks.
//!
//! The `--check` arm of `apply_mode_exits` handed four of the operator's
//! arguments straight to `cmd_check` and returned — before `cmd_apply_scoped`,
//! and therefore before any selection ran at all. Measured on 1.25.2 with
//! `alpha -> bravo` converged and unrelated `charlie` red:
//!
//! ```text
//!   apply --check --subset alpha    FAIL charlie (local) — exit 1     rc=1
//!   apply --check -g web            FAIL charlie (local) — exit 1     rc=1
//!   apply --check --skip charlie    FAIL charlie (local) — exit 1     rc=1
//! ```
//!
//! Every scoping flag was inert: `--subset`, `--exclude`, `--skip`,
//! `--only-machine`, `--exclude-machine` and `--resource-filter` were not even
//! read on this path, and `-r`/`-g` were passed but as a plain id filter with
//! no closure. A scoped check that fails on a resource outside its own scope
//! reports the operator's stack as broken over something they excluded — and in
//! CI the exit code is the only thing read.
//!
//! So this branch does what the apply does, in the same order and with the same
//! `Selectors`: parse, resolve once, then check what came back. Lives in its own
//! file because `dispatch_apply_b.rs` is at the 500-line ceiling.

use super::apply_selection::{resolve_selection, Selectors};
use super::commands::ApplyArgs;
use std::path::Path;

/// The `ApplyScope` half of the operator's selectors (GH-211).
///
/// One reader for the four flags, shared with `apply_execute`, so the check and
/// the apply cannot drift apart about which flags scope a run.
pub(super) fn scope_of(args: &ApplyArgs) -> super::apply_scope::ApplyScope<'_> {
    super::apply_scope::ApplyScope {
        skip: args.skip.as_deref(),
        only_machine: args.only_machine.as_deref(),
        exclude_machine: args.exclude_machine.as_deref(),
        resource_filter: args.resource_filter.as_deref(),
    }
}

/// Every resource-set selector the operator typed, as one value.
///
/// `-m` is deliberately absent (it picks the executor, not the resource set);
/// `-t` is carried so the resolver can existence-check it, and applied again
/// per resource by the check loop, exactly as the planner applies it.
pub(super) fn selectors_of<'a>(
    args: &'a ApplyArgs,
    scope: &super::apply_scope::ApplyScope<'a>,
) -> Selectors<'a> {
    Selectors {
        resource: args.resource.as_deref(),
        group: args.group.as_deref(),
        subset: args.subset.as_deref(),
        exclude: args.exclude.as_deref(),
        tag: args.tag.as_deref(),
        ..Default::default()
    }
    .with_scope(scope)
}

/// `apply --check`: parse, resolve the selection, check the selected config.
///
/// Exit code is unchanged in meaning and narrower in scope: 0 when every
/// SELECTED check passes, even if an unselected resource is red.
pub(super) fn cmd_apply_check(
    args: &ApplyArgs,
    state_dir: &Path,
    verbose: bool,
) -> Result<(), String> {
    let mut config = super::helpers::parse_and_validate(&args.file)?;
    let scope = scope_of(args);
    resolve_selection(&mut config, &selectors_of(args, &scope), verbose)?;
    super::check::cmd_check_selected(
        &config,
        args.machine.as_deref(),
        args.tag.as_deref(),
        state_dir,
        args.json,
        verbose,
    )
}
