//! Cleanup then sweep (#1139): apply the orphaned-runner cleanup plan to the
//! in-memory registration list, then run the orphan sweep on what is left.
//! Pure: no network, no process. Nothing here weakens the sweep's refusals.

use super::pool_orphans::{sweep, Host, Runner, RunnerListing, Sweep, SweepError};
use super::pool_runner_cleanup::{
    plan_cleanup, CleanupError, CleanupPlan, LiveHosts, PoolIdentity, Registration,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComposeError {
    Cleanup(CleanupError),
    Sweep(SweepError),
}

/// The cleanup plan and the sweep computed after it was applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Composed {
    pub plan: CleanupPlan,
    pub sweep: Sweep,
}

/// Plan the cleanup against the live `hosts`, drop the planned registrations,
/// and sweep the remainder. A busy or online runner on an unlisted host is
/// never planned, so the sweep still refuses with `UnknownRunnerHost`.
pub fn sweep_after_cleanup(
    runners: &[Registration],
    hosts: &[Host],
    id: &PoolIdentity,
) -> Result<Composed, ComposeError> {
    let live = LiveHosts::Complete(hosts.iter().map(|h| h.id.clone()).collect());
    let plan = plan_cleanup(runners, &live, id).map_err(ComposeError::Cleanup)?;
    let kept = runners
        .iter()
        .filter(|r| !plan.deregister.contains(&r.name))
        .map(|r| Runner {
            name: r.name.clone(),
            host_id: r.host_id.clone(),
            busy: r.busy,
        })
        .collect();
    let sweep = sweep(&RunnerListing::Complete(kept), hosts).map_err(ComposeError::Sweep)?;
    Ok(Composed { plan, sweep })
}

#[cfg(test)]
#[path = "pool_cleanup_sweep_tests.rs"]
mod tests;
