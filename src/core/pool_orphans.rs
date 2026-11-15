//! Pool orphan sweep (#894, slice 2): a pure function from the registered
//! runners and the live hosts to the cleanups. No I/O; the caller applies it.

use super::pool_reconcile::SCALE_DOWN_QUIET_SECS;

/// A runner registered with the forge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Runner {
    pub name: String,
    pub host_id: String,
    pub busy: bool,
}

/// A live cloud host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Host {
    pub id: String,
    /// Seconds since the host last had a busy seat.
    pub idle_secs: u64,
}

/// What one sweep removes. Both lists keep input order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Sweep {
    /// Names of runners to deregister (their host is gone).
    pub deregister_runners: Vec<String>,
    /// Ids of hosts to terminate (idle past the rule, no registered runner).
    pub terminate_hosts: Vec<String>,
}

/// Compute one sweep. A busy runner is never deregistered and its host is
/// never terminated; a busy runner on a vanished host is left for the next
/// sweep, once the forge reports it idle.
pub fn sweep(runners: &[Runner], hosts: &[Host]) -> Sweep {
    let live = |id: &str| hosts.iter().any(|h| h.id == id);
    let deregister_runners = runners
        .iter()
        .filter(|r| !r.busy && !live(&r.host_id))
        .map(|r| r.name.clone())
        .collect();
    let terminate_hosts = hosts
        .iter()
        .filter(|h| h.idle_secs >= SCALE_DOWN_QUIET_SECS)
        .filter(|h| !runners.iter().any(|r| r.host_id == h.id))
        .map(|h| h.id.clone())
        .collect();
    Sweep {
        deregister_runners,
        terminate_hosts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(name: &str, host: &str, busy: bool) -> Runner {
        Runner {
            name: name.into(),
            host_id: host.into(),
            busy,
        }
    }
    fn h(id: &str, idle: u64) -> Host {
        Host {
            id: id.into(),
            idle_secs: idle,
        }
    }

    #[test]
    fn one_sweep_leaves_no_orphans_on_a_synthetic_set() {
        let runners = vec![
            r("r1", "h1", false),
            r("r2", "gone", false),
            r("r3", "gone2", false),
            r("r4", "h2", true),
        ];
        let hosts = vec![h("h1", 0), h("h2", 9999), h("h3", 600), h("h4", 700)];
        let s = sweep(&runners, &hosts);
        assert_eq!(s.deregister_runners, vec!["r2", "r3"]);
        assert_eq!(s.terminate_hosts, vec!["h3", "h4"]);
        // Apply it, then sweep again: nothing left.
        let runners: Vec<Runner> = runners
            .into_iter()
            .filter(|x| !s.deregister_runners.contains(&x.name))
            .collect();
        let hosts: Vec<Host> = hosts
            .into_iter()
            .filter(|x| !s.terminate_hosts.contains(&x.id))
            .collect();
        assert_eq!(sweep(&runners, &hosts), Sweep::default());
    }

    #[test]
    fn busy_runner_is_never_selected_nor_its_host() {
        let runners = vec![r("busy", "h1", true), r("busy-ghost", "gone", true)];
        let hosts = vec![h("h1", 99_999)];
        assert_eq!(sweep(&runners, &hosts), Sweep::default());
    }

    #[test]
    fn host_with_an_idle_runner_is_kept() {
        let s = sweep(&[r("r", "h1", false)], &[h("h1", 99_999)]);
        assert!(s.terminate_hosts.is_empty());
    }

    #[test]
    fn idle_rule_boundary_is_inclusive() {
        let hosts = vec![
            h("a", SCALE_DOWN_QUIET_SECS - 1),
            h("b", SCALE_DOWN_QUIET_SECS),
        ];
        assert_eq!(sweep(&[], &hosts).terminate_hosts, vec!["b"]);
    }

    #[test]
    fn empty_inputs_sweep_nothing() {
        assert_eq!(sweep(&[], &[]), Sweep::default());
    }
}
