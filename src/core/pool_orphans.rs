//! Pool orphan sweep (#894, slice 2): a pure function from the registered
//! runners and the live hosts to the cleanups. No I/O; the caller applies it.
//!
//! The output terminates instances. Input it cannot trust is refused with a
//! [`SweepError`] and nothing is terminated, except a runner whose host id
//! equals no listed host: it is returned in `Sweep::unknown_runners`, and only
//! a host whose id equals that host id ignoring ASCII case is kept.
//! It returns hosts to terminate only; runner deregistration is not part of it.

use super::pool_reconcile::SCALE_DOWN_QUIET_SECS;

/// A host younger than this has not necessarily registered its runner yet,
/// so it is never terminated as an orphan.
pub const REGISTRATION_GRACE_SECS: u64 = 900;

/// A runner registered with the forge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Runner {
    pub name: String,
    pub host_id: String,
    pub busy: bool,
}

/// The forge's runner listing, with whether it is the whole listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunnerListing {
    /// Every page was read.
    Complete(Vec<Runner>),
    /// The listing failed, was cut short, or was not fully paginated.
    Partial,
}

/// A live cloud host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Host {
    pub id: String,
    /// Seconds since the host last had a busy seat.
    pub idle_secs: u64,
    /// Seconds since the host was launched.
    pub age_secs: u64,
}

/// What one sweep removes. The list keeps input order, without duplicates.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Sweep {
    /// Ids of hosts to terminate (idle past the rule, no registered runner,
    /// past the registration grace).
    pub terminate_hosts: Vec<String>,
    /// `(runner name, host id)` of runners whose host id equals no listed host.
    pub unknown_runners: Vec<(String, String)>,
}

/// Why a sweep refused its input. Nothing is terminated on any of these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SweepError {
    /// The runner listing is not known to be complete.
    IncompleteListing,
    /// A host has an empty id.
    EmptyHostId,
    /// A runner has an empty host id.
    EmptyRunnerHostId { runner: String },
    /// Two host ids are equal ignoring ASCII case.
    DuplicateHostId(String),
    /// Two runner names are equal ignoring ASCII case (like host ids).
    DuplicateRunnerName(String),
    /// A runner name is empty or only whitespace.
    EmptyRunnerName,
    /// A host reports more idle seconds than its age: inconsistent input.
    IdleExceedsAge(String),
    /// A host id has a character outside printable ASCII (no whitespace,
    /// control or non-ASCII characters).
    MalformedId(String),
    /// A runner's host id has a character outside printable ASCII.
    MalformedRunnerHostId { runner: String, host: String },
    /// A runner on a host no listing names is busy: the cleanup refuses, since
    /// deregistering around live work is not safe. Nothing is deregistered.
    BusyRunnerOnUnlistedHost { runner: String, host: String },
}

/// What one orphaned-runner cleanup pass removes. It terminates nothing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Cleanup {
    /// Names of idle runners whose host id equals no listed host, to deregister.
    pub deregister_runners: Vec<String>,
}

fn first_duplicate(items: impl Iterator<Item = String>) -> Option<String> {
    let mut seen = std::collections::HashSet::new();
    items.into_iter().find(|s| !seen.insert(s.clone()))
}

fn well_formed(id: &str) -> bool {
    id.chars().all(|c| c.is_ascii_graphic())
}

fn check_well_formed(runners: &[Runner], hosts: &[Host]) -> Result<(), SweepError> {
    if let Some(h) = hosts.iter().find(|h| !well_formed(&h.id)) {
        return Err(SweepError::MalformedId(h.id.clone()));
    }
    match runners.iter().find(|r| !well_formed(&r.host_id)) {
        Some(r) => Err(SweepError::MalformedRunnerHostId {
            runner: r.name.clone(),
            host: r.host_id.clone(),
        }),
        None => Ok(()),
    }
}

fn check_ids(runners: &[Runner], hosts: &[Host]) -> Result<(), SweepError> {
    if hosts.iter().any(|h| h.id.is_empty()) {
        return Err(SweepError::EmptyHostId);
    }
    if let Some(r) = runners.iter().find(|r| r.host_id.is_empty()) {
        return Err(SweepError::EmptyRunnerHostId {
            runner: r.name.clone(),
        });
    }
    check_well_formed(runners, hosts)?;
    if let Some(d) = first_duplicate(hosts.iter().map(|h| h.id.to_ascii_lowercase())) {
        return Err(SweepError::DuplicateHostId(d));
    }
    if runners.iter().any(|r| r.name.trim().is_empty()) {
        return Err(SweepError::EmptyRunnerName);
    }
    match first_duplicate(runners.iter().map(|r| r.name.to_ascii_lowercase())) {
        Some(d) => Err(SweepError::DuplicateRunnerName(d)),
        None => Ok(()),
    }
}

fn validate(runners: &[Runner], hosts: &[Host]) -> Result<(), SweepError> {
    check_ids(runners, hosts)?;
    if let Some(h) = hosts.iter().find(|h| h.idle_secs > h.age_secs) {
        return Err(SweepError::IdleExceedsAge(h.id.clone()));
    }
    Ok(())
}

/// Compute one sweep: the hosts to terminate. This slice returns only hosts
/// to terminate and `unknown_runners`; it does not deregister runners and does not deliver the
/// parent's "0 orphaned runners" check. `Runner.busy` is not read: any runner
/// protects its host. With zero runners and a `Complete` listing, every host
/// idle past the rule and older than the grace is returned.
pub fn sweep(listing: &RunnerListing, hosts: &[Host]) -> Result<Sweep, SweepError> {
    let RunnerListing::Complete(runners) = listing else {
        return Err(SweepError::IncompleteListing);
    };
    validate(runners, hosts)?;
    let unknown_runners: Vec<(String, String)> = runners
        .iter()
        .filter(|r| !hosts.iter().any(|h| h.id == r.host_id))
        .map(|r| (r.name.clone(), r.host_id.clone()))
        .collect();
    let terminate_hosts = hosts
        .iter()
        .filter(|h| {
            h.idle_secs >= SCALE_DOWN_QUIET_SECS
                && h.age_secs >= REGISTRATION_GRACE_SECS
                && !runners.iter().any(|r| r.host_id == h.id)
                && !unknown_runners
                    .iter()
                    .any(|(_, u)| u.eq_ignore_ascii_case(&h.id))
        })
        .map(|h| h.id.clone())
        .collect();
    Ok(Sweep {
        terminate_hosts,
        unknown_runners,
    })
}

/// Orphaned-runner cleanup (#1139): the idle runners whose host is not listed,
/// to deregister. It terminates nothing. Host ids match ignoring ASCII case,
/// as in [`sweep`]. A busy runner on an unlisted host refuses the whole pass
/// (nothing is deregistered), and so does an incomplete or invalid listing.
/// After the caller deregisters the result, [`sweep`] has no unknown runners.
pub fn cleanup_orphan_runners(
    listing: &RunnerListing,
    hosts: &[Host],
) -> Result<Cleanup, SweepError> {
    let RunnerListing::Complete(runners) = listing else {
        return Err(SweepError::IncompleteListing);
    };
    validate(runners, hosts)?;
    let orphans: Vec<&Runner> = runners
        .iter()
        .filter(|r| !hosts.iter().any(|h| h.id.eq_ignore_ascii_case(&r.host_id)))
        .collect();
    if let Some(r) = orphans.iter().find(|r| r.busy) {
        return Err(SweepError::BusyRunnerOnUnlistedHost {
            runner: r.name.clone(),
            host: r.host_id.clone(),
        });
    }
    Ok(Cleanup {
        deregister_runners: orphans.iter().map(|r| r.name.clone()).collect(),
    })
}

#[cfg(test)]
#[path = "pool_orphans_tests.rs"]
mod tests;
