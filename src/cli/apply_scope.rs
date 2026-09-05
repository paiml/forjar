//! GH-211: the four apply scope selectors that parsed and did nothing.
//!
//! `--subset`, `-r` and `-m` narrowed an apply correctly. Their four documented
//! siblings did not: `--skip`, `--only-machine`, `--exclude-machine` and
//! `--resource-filter` were declared on `ApplyArgs`, printed in `--help` with an
//! FJ- ticket number, accepted, and read by nothing. Measured on 1.12.3 against
//! a three-resource config:
//!
//! ```text
//!   --subset 'a-*'        rc=0  files=[a.txt]              (correct)
//!   --resource-filter 'a-*'  rc=0  files=[a.txt b.txt c.txt]  (!!)
//!   --skip a-file         rc=0  files=[a.txt b.txt c.txt]  (!!)
//!   --exclude-machine local  rc=0  files=[a.txt b.txt c.txt]  (!!)
//!   --only-machine ghost  rc=0  files=[a.txt b.txt c.txt]  (!!)
//! ```
//!
//! Excluding the only machine still applied everything to it. An operator who
//! asked for LESS got EVERYTHING plus a success summary — the worst possible
//! direction for a scoping flag to fail in, and the reason these four are
//! implemented here rather than refused: the selector machinery they needed
//! already existed one call away, and a scoping flag that widens the blast
//! radius is not a missing feature, it is a hazard.
//!
//! # Machine scope narrows the resource, not just the selection
//!
//! A resource may target several machines (`machine: [a, b]`). Retaining such a
//! resource under `--only-machine a` and then applying it unchanged would still
//! touch `b`. So the machine selectors rewrite each surviving resource's target
//! list, which is what makes the frame obligation hold literally:
//! `frame(--exclude-machine m) ∩ resources_on(m) = ∅`.
//!
//! # PMAT-160: the four selectors are data here, and nothing else
//!
//! This file used to carry a second implementation of all four — `scope_skip`,
//! `scope_resource_filter`, `scope_only_machine`, `scope_exclude_machine` — run
//! from `cmd_apply_scoped` before `--subset`/`--exclude` and before the goal
//! closure. That ordering is what #468 reported: a prune before validation
//! turns a correctly declared `depends_on` into "depends on unknown". The
//! selectors now travel to `apply_selection::resolve_selection`, which applies
//! them AFTER the graph is validated and the positive set closed, with edge
//! contraction (`apply_selection::narrow`) instead of a bare delete. What is
//! left here is the struct that carries them.

use super::apply::cmd_apply_scoped;
use std::path::Path;

/// The four scope selectors that reach `cmd_apply` alongside `--subset`/`-r`/`-m`.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct ApplyScope<'a> {
    /// `--skip <RESOURCE>` (FJ-396): drop one named resource.
    pub skip: Option<&'a str>,
    /// `--only-machine <MACHINE>` (FJ-736): restrict the apply to one machine.
    pub only_machine: Option<&'a str>,
    /// `--exclude-machine <MACHINE>` (FJ-726): apply to everything but one machine.
    pub exclude_machine: Option<&'a str>,
    /// `--resource-filter <GLOB>` (FJ-666): keep only resources matching a glob.
    pub resource_filter: Option<&'a str>,
}

// GH-208: `cmd_apply` is the default-scope entry point. It lived in apply.rs as
// an 81-line pass-through that duplicated cmd_apply_scoped's entire 20-argument
// signature, which tripped the TDG quality gate (apply.rs B -> B-). It belongs
// beside the ApplyScope it defaults.
/// GH-211: the pre-scope entry point, kept so every existing caller — `make`,
/// drift auto-remediation and the suite — compiles unchanged. The four scope
/// selectors default to "no extra scoping", which is exactly what those callers
/// mean; only `apply` has flags for them.
#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_apply(
    file: &Path,
    state_dir: &Path,
    machine_filter: Option<&str>,
    resource_filter: Option<&str>,
    tag_filter: Option<&str>,
    group_filter: Option<&str>,
    force: bool,
    dry_run: bool,
    no_tripwire: bool,
    param_overrides: &[String],
    auto_commit: bool,
    timeout_secs: Option<u64>,
    json: bool,
    verbose: bool,
    env_file: Option<&Path>,
    workspace: Option<&str>,
    report: bool,
    force_unlock: bool,
    output_mode: Option<&str>,
    progress: bool,
    timing: bool,
    retry: u32,
    yes: bool,
    parallel: bool,
    resource_timeout: Option<u64>,
    rollback_on_failure: bool,
    max_parallel: Option<usize>,
    notify: Option<&str>,
    subset: Option<&str>,
    confirm_destructive: bool,
    exclude: Option<&str>,
    sequential: bool,
    telemetry_endpoint: Option<&str>,
    refresh: bool,
    force_tag: Option<&str>,
    // FJ-2724: `make`-style goals. Empty means "the whole config".
    goals: &[String],
) -> Result<(), String> {
    cmd_apply_scoped(
        file,
        state_dir,
        machine_filter,
        resource_filter,
        tag_filter,
        group_filter,
        force,
        dry_run,
        no_tripwire,
        param_overrides,
        auto_commit,
        timeout_secs,
        json,
        verbose,
        env_file,
        workspace,
        report,
        force_unlock,
        output_mode,
        progress,
        timing,
        retry,
        yes,
        parallel,
        resource_timeout,
        rollback_on_failure,
        max_parallel,
        notify,
        subset,
        confirm_destructive,
        exclude,
        sequential,
        telemetry_endpoint,
        refresh,
        force_tag,
        goals,
        &ApplyScope::default(),
    )
}
