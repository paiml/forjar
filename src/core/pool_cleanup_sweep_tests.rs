use super::*;
use crate::core::pool_runner_cleanup::RunnerStatus;

const OLD: u64 = 1_000_000;

fn id() -> PoolIdentity {
    PoolIdentity {
        prefix: "fj".into(),
        label: "forjar-pool".into(),
    }
}
fn host(i: &str) -> Host {
    Host {
        id: i.into(),
        idle_secs: OLD,
        age_secs: OLD,
    }
}
fn reg(name: &str, h: &str, status: RunnerStatus, busy: bool) -> Registration {
    Registration {
        name: name.into(),
        host_id: h.into(),
        status,
        busy,
        labels: vec!["forjar-pool".into()],
    }
}

#[test]
fn sweep_refuses_alone_but_proceeds_after_cleanup() {
    let hosts = [host("h1")];
    let orphan = reg("fj-1", "gone", RunnerStatus::Offline, false);
    let listing = RunnerListing::Complete(vec![Runner {
        name: "fj-1".into(),
        host_id: "gone".into(),
        busy: false,
    }]);
    assert!(matches!(
        sweep(&listing, &hosts),
        Err(SweepError::UnknownRunnerHost { .. })
    ));
    let c = sweep_after_cleanup(&[orphan], &hosts, &id()).unwrap();
    assert_eq!(c.plan.deregister, vec!["fj-1"]);
    assert_eq!(c.sweep.terminate_hosts, vec!["h1"]);
}

#[test]
fn still_refuses_when_the_runner_is_busy() {
    let r = reg("fj-1", "gone", RunnerStatus::Offline, true);
    let e = sweep_after_cleanup(&[r], &[host("h1")], &id()).unwrap_err();
    assert!(matches!(
        e,
        ComposeError::Sweep(SweepError::UnknownRunnerHost { .. })
    ));
}

#[test]
fn still_refuses_when_the_runner_is_online() {
    let r = reg("fj-1", "gone", RunnerStatus::Online, false);
    let e = sweep_after_cleanup(&[r], &[host("h1")], &id()).unwrap_err();
    assert!(matches!(
        e,
        ComposeError::Sweep(SweepError::UnknownRunnerHost { .. })
    ));
}

#[test]
fn cleanup_errors_pass_through() {
    let r = reg("fj-1", "gone", RunnerStatus::Offline, false);
    let e = sweep_after_cleanup(&[r], &[], &id()).unwrap_err();
    assert_eq!(e, ComposeError::Cleanup(CleanupError::EmptyHostList));
}
