//! forjar#385: the drift run that has no lock to compare against.
//!
//! Reached only from `drift_state::machine_state_dirs` returning `Ok(None)` —
//! the state directory is ABSENT, not unreadable. The scan then walks the
//! CONFIG instead of the lock, executes every `type: task` completion_check the
//! selected machines declare, and censuses everything that needs a baseline as
//! `SkipReason::NoLock`.
//!
//! The verdict is smaller than a locked run's and says so on every line it
//! prints. What it is not is absent: paiml/infra's nightly lane exited 1 on the
//! missing directory and measured NOTHING, over a fleet whose guards are
//! exactly the resources this path can still ask about.

use super::drift::{print_dry_run_report, print_machine_header, report_machine_findings};
use super::drift::{DriftScan, ScanOptions};
use super::drift_report::census_json;
use crate::core::types;
use crate::tripwire::drift;
use std::path::Path;

/// Run every assertion the config declares, over the machines it names.
pub(super) fn scan_lockless(
    state_dir: &Path,
    machine_filter: Option<&str>,
    config: Option<&types::ForjarConfig>,
    scan_opts: ScanOptions,
) -> Result<DriftScan, String> {
    let cfg = require_config(state_dir, config)?;
    let names = select_machines(cfg, machine_filter)?;
    let resolved = resolve(cfg);
    let mut scan = DriftScan::default();
    for name in names {
        // `machines_checked` counts INSIDE the loop, not `names.len()` before
        // it. A machine that was selected and then not scanned is exactly the
        // over-count that lets a caller read coverage it did not get, and this
        // number is what paiml/infra's lane divides by.
        let Some(machine) = cfg.machines.get(&name) else {
            continue;
        };
        print_machine_header(&name, SCOPE_NOTE, scan_opts);
        let report = drift::detect_drift_lockless(&name, machine, &resolved, scan_opts.detect);
        let (count, census) = report_machine_findings(&name, report, &mut scan.findings, scan_opts);
        scan.machines_checked += 1;
        scan.total_drift += count;
        scan.censuses.push(census_json(&name, &census));
    }
    Ok(scan)
}

/// What `drift --dry-run` previews when there is no lock.
///
/// The preview and the run share one predicate (`lockless_dry_run_ids`), so a
/// preview cannot name work the run will not do — the failure that makes a
/// preview worse than none at all.
pub(super) fn dry_run_lockless(
    state_dir: &Path,
    machine_filter: Option<&str>,
    config: Option<&types::ForjarConfig>,
    json: bool,
    opts: drift::DriftOptions,
) -> Result<(), String> {
    let cfg = require_config(state_dir, config)?;
    let names = select_machines(cfg, machine_filter)?;
    let resolved = resolve(cfg);
    let mut checks: Vec<serde_json::Value> = Vec::new();
    let mut total = 0usize;
    for name in names {
        let ids = drift::lockless_dry_run_ids(&name, &resolved, opts);
        if !json {
            println!(
                "Machine: {} ({}, {} checkable)",
                name,
                SCOPE_NOTE,
                ids.len()
            );
        }
        for id in &ids {
            if json {
                checks.push(serde_json::json!({
                    "machine": name,
                    "resource": id,
                    "status": "no lock",
                    "hash": serde_json::Value::Null,
                }));
            } else {
                println!("  would check: {id} (completion_check; no lock)");
            }
        }
        total += ids.len();
    }
    print_dry_run_report(json, total, &checks)
}

/// How this run describes its own scope, everywhere it prints one.
const SCOPE_NOTE: &str = "no lock — assertions only";

/// Template-resolve before anything executes.
///
/// PMAT-197: an unresolved `{{params.*}}` inside a `completion_check` would run
/// a DIFFERENT script than the one `apply` runs. That is a wrong answer, which
/// is worse than the missing one this whole module exists to replace.
fn resolve(cfg: &types::ForjarConfig) -> indexmap::IndexMap<String, types::Resource> {
    crate::core::resolver::resolve_all(&cfg.resources, &cfg.params, &cfg.machines, &cfg.secrets)
}

/// NO LOCK AND NO CONFIG IS NOT A CLEAN BILL OF HEALTH.
///
/// A missing state dir is survivable because the config still carries the
/// assertions. With no config either there is nothing to assert and nothing to
/// compare, so the only honest exit is a refusal — printing `No drift detected.`
/// over zero information is the defect one level up from the one being fixed.
fn require_config<'a>(
    state_dir: &Path,
    config: Option<&'a types::ForjarConfig>,
) -> Result<&'a types::ForjarConfig, String> {
    config.ok_or_else(|| {
        format!(
            "no lock and no config: {} does not exist and no config was loaded, \
             so NOTHING was measured. An absent state dir alone is survivable — \
             forjar can still execute the completion_check of every `type: task` \
             a config declares — but with neither there is no assertion to run \
             and no baseline to compare. Point -f at a config, or --state-dir at \
             the directory an apply wrote.",
            state_dir.display()
        )
    })
}

/// The machines this run will ask about, from the CONFIG rather than the lock.
///
/// An unknown `-m` is refused for the same reason the locked path refuses it: a
/// machine you named and could not check is not a pass, and a typo in a cron'd
/// `--tripwire` must not quietly stop checking anything.
fn select_machines(
    cfg: &types::ForjarConfig,
    machine_filter: Option<&str>,
) -> Result<Vec<String>, String> {
    let Some(filter) = machine_filter else {
        return Ok(cfg.machines.keys().cloned().collect());
    };
    if !cfg.machines.contains_key(filter) {
        let known: Vec<&str> = cfg.machines.keys().map(String::as_str).collect();
        return Err(format!(
            "unknown machine '{filter}' — there is no state directory and the \
             config does not declare it, so NOTHING was checked. Known: {}",
            if known.is_empty() {
                "(none)".to_string()
            } else {
                known.join(", ")
            }
        ));
    }
    Ok(vec![filter.to_string()])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(yaml: &str) -> types::ForjarConfig {
        serde_yaml_ng::from_str(yaml).expect("config")
    }

    fn two_machines() -> types::ForjarConfig {
        cfg("version: '1.0'\nname: t\nmachines:\n  a:\n    hostname: a\n    addr: 127.0.0.1\n  b:\n    hostname: b\n    addr: 127.0.0.1\nresources: {}\n")
    }

    #[test]
    fn no_filter_selects_every_declared_machine() {
        let c = two_machines();
        assert_eq!(select_machines(&c, None).unwrap(), vec!["a", "b"]);
    }

    #[test]
    fn a_filter_selects_exactly_one() {
        let c = two_machines();
        assert_eq!(select_machines(&c, Some("b")).unwrap(), vec!["b"]);
    }

    /// The false green a typo used to buy, refused on this path too.
    #[test]
    fn an_unknown_machine_is_refused_and_names_what_exists() {
        let c = two_machines();
        let err = select_machines(&c, Some("bb")).unwrap_err();
        assert!(err.contains("unknown machine 'bb'"), "{err}");
        assert!(err.contains("Known: a, b"), "{err}");
    }

    /// Nothing to compare AND nothing to assert is a refusal, not a verdict.
    #[test]
    fn no_config_is_a_refusal() {
        let err = require_config(Path::new("/nope/state"), None).unwrap_err();
        assert!(err.contains("NOTHING was measured"), "{err}");
        assert!(err.contains("/nope/state"), "{err}");
    }
}
