//! Pool orphan sweep (#894, slice 2): a pure function from the registered
//! runners and the live hosts to the cleanups. No I/O; the caller applies it.
//!
//! The output terminates instances, so the sweep fails closed: any input it
//! cannot trust is refused with a [`SweepError`] and nothing is terminated.

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

/// What one sweep removes. Both lists keep input order, without duplicates.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Sweep {
    /// Names of runners to deregister (their host is gone).
    pub deregister_runners: Vec<String>,
    /// Ids of hosts to terminate (idle past the rule, no registered runner,
    /// past the registration grace).
    pub terminate_hosts: Vec<String>,
}

/// Why a sweep refused its input. Nothing is terminated on any of these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SweepError {
    /// The runner listing is not known to be complete.
    IncompleteListing,
    /// A host has an empty id.
    EmptyHostId,
    /// A runner has an empty host id.
    EmptyRunnerHostId {
        runner: String,
    },
    DuplicateHostId(String),
    DuplicateRunnerName(String),
    /// A host reports more idle seconds than its age: inconsistent input.
    IdleExceedsAge(String),
    /// A runner's host id matches a host only by case or containment
    /// (prefix, suffix): a naming mismatch, refused rather than guessed.
    NearMatchHostId {
        runner: String,
        host: String,
    },
}

fn first_duplicate<'a>(items: impl Iterator<Item = &'a str>) -> Option<String> {
    let mut seen = std::collections::HashSet::new();
    items
        .into_iter()
        .find(|s| !seen.insert(*s))
        .map(String::from)
}

fn push_unique(out: &mut Vec<String>, s: &str) {
    if !out.iter().any(|x| x == s) {
        out.push(s.to_string());
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
    if let Some(d) = first_duplicate(hosts.iter().map(|h| h.id.as_str())) {
        return Err(SweepError::DuplicateHostId(d));
    }
    match first_duplicate(runners.iter().map(|r| r.name.as_str())) {
        Some(d) => Err(SweepError::DuplicateRunnerName(d)),
        None => Ok(()),
    }
}

fn is_near(a: &str, b: &str) -> bool {
    a != b && (a.eq_ignore_ascii_case(b) || a.contains(b) || b.contains(a))
}

fn check_near(r: &Runner, hosts: &[Host]) -> Result<(), SweepError> {
    if hosts.iter().any(|h| h.id == r.host_id) {
        return Ok(());
    }
    let lower = r.host_id.to_ascii_lowercase();
    match hosts
        .iter()
        .find(|h| is_near(&h.id, &r.host_id) || is_near(&h.id.to_ascii_lowercase(), &lower))
    {
        Some(h) => Err(SweepError::NearMatchHostId {
            runner: r.name.clone(),
            host: h.id.clone(),
        }),
        None => Ok(()),
    }
}

fn validate(runners: &[Runner], hosts: &[Host]) -> Result<(), SweepError> {
    check_ids(runners, hosts)?;
    if let Some(h) = hosts.iter().find(|h| h.idle_secs > h.age_secs) {
        return Err(SweepError::IdleExceedsAge(h.id.clone()));
    }
    runners.iter().try_for_each(|r| check_near(r, hosts))
}

/// Compute one sweep. A busy runner is never deregistered and its host is
/// never terminated; a busy runner on a vanished host is left for the next
/// sweep, once the forge reports it idle (the forge can keep an offline runner
/// busy until its job timeout, so this can linger).
pub fn sweep(listing: &RunnerListing, hosts: &[Host]) -> Result<Sweep, SweepError> {
    let RunnerListing::Complete(runners) = listing else {
        return Err(SweepError::IncompleteListing);
    };
    validate(runners, hosts)?;
    let mut out = Sweep::default();
    for r in runners {
        if !r.busy && !hosts.iter().any(|h| h.id == r.host_id) {
            push_unique(&mut out.deregister_runners, &r.name);
        }
    }
    for h in hosts {
        let registered = runners.iter().any(|r| r.host_id == h.id);
        if !registered
            && h.idle_secs >= SCALE_DOWN_QUIET_SECS
            && h.age_secs >= REGISTRATION_GRACE_SECS
        {
            push_unique(&mut out.terminate_hosts, &h.id);
        }
    }
    Ok(out)
}

#[cfg(test)]
#[path = "pool_orphans_tests.rs"]
mod tests;
