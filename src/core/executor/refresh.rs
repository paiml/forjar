//! FJ-3010: `--refresh` — re-run check scripts against the HOST and
//! re-apply only what fails.
//!
//! Split out of `mod.rs` to keep it under the 500-line limit.

use super::super::codegen;
use super::super::resolver;
use super::super::types::*;
use super::ApplyConfig;
use crate::transport;
use std::collections::{HashMap, HashSet};

/// FJ-3010: Build locks with entries removed for resources whose LIVE check
/// script fails, so the planner re-plans exactly those and leaves the rest.
///
/// This is what `--refresh` always advertised — "re-run check scripts, only
/// re-apply what fails" — and never did. `ApplyConfig::refresh` was declared,
/// commented, threaded through every call site as `false`, and read by nothing.
/// The flag was accepted and ignored.
///
/// What that cost, measured on paiml/infra's intel 2026-08-19: a shared
/// `~/.cargo/bin` was emptied by a CI job's cache-prune step, and
/// `forjar apply -t stack-tools` reported 5 of 5 resources converged on a host
/// with no rustup, no cargo and no rustc. `--refresh` returned in 0.1s without
/// contacting the machine. Each resource's own check script, run by hand
/// against that host, correctly printed `missing:` and exited 1 — the checks
/// were right, nothing asked them.
///
/// Deliberately NOT the default. A plain apply stays lock-based and fast; only
/// `--refresh` pays for a host round-trip per resource. Making every apply
/// verify is a bigger decision than fixing a flag that lied.
///
/// Exit 2 is NOT a failure (FJ-2720: "not applicable on this host"), and a
/// transport error is not either — neither observes a diverged resource, and
/// re-applying on "I could not look" would rebuild the world whenever a host is
/// briefly unreachable.
/// Does `--refresh` owe this resource a check?
///
/// Honours the apply's own scoping — `--refresh -t stack-tools` must not pay
/// for a host round-trip on all 100 resources. A phony resource names an action
/// with no artifact (FJ-2725), so it has nothing to observe and must never be
/// forced every run.
fn refresh_in_scope(cfg: &ApplyConfig, id: &str, resource: &Resource) -> bool {
    if cfg.resource_filter.is_some_and(|f| id != f) {
        return false;
    }
    if cfg
        .tag_filter
        .is_some_and(|t| !resource.tags.iter().any(|x| x == t))
    {
        return false;
    }
    !resource.phony
}

/// Run this resource's check script on its host(s); true if any reports diverged.
///
/// Exit 2 is NOT a failure (FJ-2720: "not applicable on this host"), and neither
/// is a transport error or an unresolvable machine — none of those observed a
/// diverged resource, and re-applying on "I could not look" would rebuild the
/// world whenever a host is briefly unreachable.
fn refresh_check_fails(cfg: &ApplyConfig, resource: &Resource) -> bool {
    let Ok(resolved) =
        resolver::resolve_resource_templates(resource, &cfg.config.params, &cfg.config.machines)
    else {
        return false;
    };
    let Ok(script) = codegen::check_script(&resolved) else {
        return false;
    };
    resource.machine.iter().any(|machine_name| {
        if cfg.machine_filter.is_some_and(|f| machine_name != f) {
            return false;
        }
        // An undeclared machine cannot be checked, and guessing localhost would
        // run the check against the CONTROLLER — reporting the wrong host's
        // state with full confidence.
        cfg.config
            .machines
            .get(machine_name)
            .and_then(|m| transport::exec_script(m, &script).ok())
            .is_some_and(|out| !out.success() && out.exit_code != 2)
    })
}

pub(crate) fn refresh_locks(
    cfg: &ApplyConfig,
    locks: &HashMap<String, StateLock>,
) -> HashMap<String, StateLock> {
    let stale: HashSet<String> = cfg
        .config
        .resources
        .iter()
        .filter(|(id, r)| refresh_in_scope(cfg, id, r))
        .filter(|(_, r)| refresh_check_fails(cfg, r))
        .map(|(id, _)| id.clone())
        .collect();

    let mut result = HashMap::with_capacity(locks.len());
    for (machine, lock) in locks {
        let mut new_lock = lock.clone();
        new_lock.resources.retain(|rid, _| !stale.contains(rid));
        result.insert(machine.clone(), new_lock);
    }
    result
}
