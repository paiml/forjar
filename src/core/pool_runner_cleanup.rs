//! Pool orphaned-runner cleanup (#1139): a pure function from the registered
//! runners and the live hosts to the registrations to deregister. No I/O; the
//! caller applies the plan. It terminates nothing, and it fails closed.

/// At most this many registrations are deregistered in one sweep.
pub const MAX_DEREGISTER_PER_SWEEP: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunnerStatus {
    Online,
    Offline,
}

/// A runner registered with the forge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registration {
    pub name: String,
    pub host_id: String,
    pub status: RunnerStatus,
    pub busy: bool,
    pub labels: Vec<String>,
}

/// The live host ids, and whether the list could be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveHosts {
    Complete(Vec<String>),
    Unreadable,
}

/// The pool's identity: a runner belongs to it only if its name is
/// `<prefix>-<rest>` (non-empty rest) AND it carries `label`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolIdentity {
    pub prefix: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CleanupPlan {
    /// Names of runner registrations to deregister, in input order.
    pub deregister: Vec<String>,
    /// Orphans left for a later sweep because of the cap.
    pub deferred: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CleanupError {
    HostListUnreadable,
    /// An empty host list cannot prove any host is gone.
    EmptyHostList,
    MalformedHostId(String),
    /// The pool identity itself is empty or malformed.
    BadIdentity,
}

fn well_formed(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_graphic())
}

/// `<prefix>-<rest>` where `rest` is one alphanumeric token: a longer pool
/// prefix that extends this one (`fj` vs `fj-gpu-1`) never matches.
fn in_pool(r: &Registration, id: &PoolIdentity) -> bool {
    let rest = r
        .name
        .strip_prefix(id.prefix.as_str())
        .and_then(|t| t.strip_prefix('-'));
    rest.is_some_and(|t| !t.is_empty() && t.chars().all(|c| c.is_ascii_alphanumeric()))
        && r.labels.contains(&id.label)
}

fn orphaned(r: &Registration, id: &PoolIdentity, live: &[String]) -> bool {
    r.status == RunnerStatus::Offline
        && !r.busy
        && well_formed(&r.name)
        && well_formed(&r.host_id)
        && in_pool(r, id)
        && !live.iter().any(|h| h.eq_ignore_ascii_case(&r.host_id))
}

/// Plan the deregistration of orphaned runners: registered to this pool,
/// offline, not busy, and whose host is not in the complete live host list.
pub fn plan_cleanup(
    runners: &[Registration],
    hosts: &LiveHosts,
    id: &PoolIdentity,
) -> Result<CleanupPlan, CleanupError> {
    if !well_formed(&id.prefix) || !well_formed(&id.label) {
        return Err(CleanupError::BadIdentity);
    }
    let LiveHosts::Complete(live) = hosts else {
        return Err(CleanupError::HostListUnreadable);
    };
    if live.is_empty() {
        return Err(CleanupError::EmptyHostList);
    }
    if let Some(h) = live.iter().find(|h| !well_formed(h)) {
        return Err(CleanupError::MalformedHostId(h.clone()));
    }
    let mut all: Vec<String> = Vec::new();
    for r in runners.iter().filter(|r| orphaned(r, id, live)) {
        let twin_kept = runners
            .iter()
            .any(|o| o.name == r.name && !orphaned(o, id, live));
        if !twin_kept && !all.contains(&r.name) {
            all.push(r.name.clone());
        }
    }
    let deferred = all.len().saturating_sub(MAX_DEREGISTER_PER_SWEEP);
    let deregister = all.into_iter().take(MAX_DEREGISTER_PER_SWEEP).collect();
    Ok(CleanupPlan {
        deregister,
        deferred,
    })
}

#[cfg(test)]
#[path = "pool_runner_cleanup_tests.rs"]
mod tests;
