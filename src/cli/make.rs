//! FJ-2724 (PMAT-199): `forjar make [GOALS...]`.
//!
//! Builds each goal and its transitive `depends_on` prerequisites, and nothing
//! else — what `make foo` means.
//!
//! # Why this is not just `apply -r`
//!
//! `-r` is exact-match with no closure. `apply -r link` runs `link` and
//! silently skips the compile step it depends on, linking whatever objects
//! happen to be on disk. That is `make -o`, not `make`. `--subset`/`--exclude`
//! have the opposite hazard: an arbitrary pattern can cut a resource out from
//! under a dependent. A `depends_on` closure is downward-closed by
//! construction, so it can never execute against an unconverged prerequisite.
//!
//! The command is a thin front end: it computes nothing itself, it hands the
//! goals to `cmd_apply`, which prunes the config to their closure and runs the
//! ordinary plan/apply pipeline. There is one convergence engine, not two.

use super::commands::MakeArgs;

pub(crate) fn cmd_make(args: &MakeArgs, verbose: bool) -> Result<(), String> {
    #[allow(clippy::too_many_arguments)]
    super::apply::cmd_apply(
        &args.file,
        &args.state_dir,
        args.machine.as_deref(),
        None, // --resource: make selects by goal closure, never exact-match
        None, // --tag
        None, // --group
        args.always_make,
        args.dry_run,
        args.no_tripwire,
        &args.param,
        false, // auto_commit
        None,  // timeout_secs
        args.json,
        verbose,
        None,  // env_file
        None,  // workspace
        false, // report
        false, // force_unlock
        None,  // output_mode
        false, // progress
        false, // timing
        0,     // retry
        args.yes,
        args.jobs.is_some(), // parallel when -j is given
        None,                // resource_timeout
        false,               // rollback_on_failure
        args.jobs,
        None,  // notify
        None,  // subset
        false, // confirm_destructive
        None,  // exclude
        false, // sequential
        None,  // telemetry_endpoint
        false, // refresh
        None,  // force_tag
        &args.goals,
    )
}
