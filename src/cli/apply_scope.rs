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

use super::apply::cmd_apply_scoped;
use crate::core::types;
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

impl ApplyScope<'_> {
    /// True when no scope selector was supplied — the common path.
    pub(crate) fn is_empty(&self) -> bool {
        self.skip.is_none()
            && self.only_machine.is_none()
            && self.exclude_machine.is_none()
            && self.resource_filter.is_none()
    }
}

/// Sorted, comma-joined key list for an error message that names what IS available.
fn known(keys: impl Iterator<Item = String>) -> String {
    let mut v: Vec<String> = keys.collect();
    v.sort_unstable();
    v.join(", ")
}

/// FJ-396: `--skip <RESOURCE>` removes one resource by exact id.
///
/// A `--skip` naming nothing is a typo, and following the house rule from
/// FJ-2723 a selector that matches nothing is an error rather than a silent
/// no-op — otherwise `--skip a-fil` would apply `a-file` and report success.
fn scope_skip(config: &mut types::ForjarConfig, id: &str) -> Result<(), String> {
    if config.resources.shift_remove(id).is_none() {
        return Err(format!(
            "--skip '{id}' matches no resource in this config. Known: {}",
            known(config.resources.keys().cloned())
        ));
    }
    Ok(())
}

/// FJ-666: `--resource-filter <GLOB>` keeps only the resources a glob matches.
///
/// Deliberately the same predicate as `--subset`, because the help text
/// promises the same thing. The two compose as an intersection.
fn scope_resource_filter(config: &mut types::ForjarConfig, pattern: &str) -> Result<(), String> {
    super::apply_gates::filter_subset(&mut config.resources, pattern)
        .map(|_| ())
        .map_err(|e| format!("--resource-filter: {e}"))
}

/// FJ-736: `--only-machine <MACHINE>` restricts the apply to a single machine.
fn scope_only_machine(config: &mut types::ForjarConfig, machine: &str) -> Result<(), String> {
    if !config.machines.contains_key(machine) {
        return Err(format!(
            "--only-machine '{machine}' matches no machine in this config. Known: {}",
            known(config.machines.keys().cloned())
        ));
    }
    config
        .resources
        .retain(|_, r| r.machine.iter().any(|m| m == machine));
    if config.resources.is_empty() {
        return Err(format!(
            "--only-machine '{machine}' matches no resource in this config"
        ));
    }
    // Narrow multi-machine targets so the apply cannot reach any other host.
    for r in config.resources.values_mut() {
        r.machine = types::MachineTarget::Single(machine.to_string());
    }
    Ok(())
}

/// FJ-726: `--exclude-machine <MACHINE>` applies to everything except one machine.
///
/// An empty result is legitimate here in a way it never is for a positive
/// selector: excluding the only machine SHOULD converge nothing. It is
/// announced rather than treated as an error, because "you asked for nothing
/// and got nothing" is the requested behaviour, and the shipped bug was the
/// opposite — asking for nothing and getting everything.
fn scope_exclude_machine(
    config: &mut types::ForjarConfig,
    machine: &str,
    verbose: bool,
) -> Result<(), String> {
    if !config.machines.contains_key(machine) {
        return Err(format!(
            "--exclude-machine '{machine}' matches no machine in this config. Known: {}",
            known(config.machines.keys().cloned())
        ));
    }
    for r in config.resources.values_mut() {
        let rest: Vec<String> = r
            .machine
            .iter()
            .filter(|m| *m != machine)
            .map(str::to_string)
            .collect();
        r.machine = types::MachineTarget::Multiple(rest);
    }
    config.resources.retain(|_, r| !r.machine.is_empty());
    if config.resources.is_empty() && verbose {
        eprintln!("--exclude-machine '{machine}': no resources remain to apply");
    }
    Ok(())
}

/// Apply every scope selector, in a fixed order, to the parsed config.
///
/// Runs before `--subset`/`--exclude` and before the goal closure, so the
/// existence checks above see the config the user wrote rather than a
/// previously narrowed one.
pub(crate) fn apply_scope(
    config: &mut types::ForjarConfig,
    scope: &ApplyScope,
    verbose: bool,
) -> Result<(), String> {
    if let Some(p) = scope.resource_filter {
        scope_resource_filter(config, p)?;
    }
    if let Some(id) = scope.skip {
        scope_skip(config, id)?;
    }
    if let Some(m) = scope.only_machine {
        scope_only_machine(config, m)?;
    }
    if let Some(m) = scope.exclude_machine {
        scope_exclude_machine(config, m, verbose)?;
    }
    if verbose && !scope.is_empty() {
        eprintln!(
            "Scope selectors: {} resources selected",
            config.resources.len()
        );
    }
    Ok(())
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
