use super::*;

fn id() -> PoolIdentity {
    PoolIdentity {
        prefix: "fj".into(),
        label: "forjar-pool".into(),
    }
}
fn reg(name: &str, host: &str, status: RunnerStatus, busy: bool) -> Registration {
    Registration {
        name: name.into(),
        host_id: host.into(),
        status,
        busy,
        labels: vec!["forjar-pool".into()],
    }
}
fn orphan(name: &str) -> Registration {
    reg(name, "gone", RunnerStatus::Offline, false)
}
fn live() -> LiveHosts {
    LiveHosts::Complete(vec!["h1".into()])
}
fn names(p: &Result<CleanupPlan, CleanupError>) -> Vec<String> {
    p.as_ref().unwrap().deregister.clone()
}

#[test]
fn offline_idle_runner_on_unlisted_host_is_planned() {
    assert_eq!(
        names(&plan_cleanup(&[orphan("fj-1")], &live(), &id())),
        vec!["fj-1"]
    );
}

#[test]
fn runner_on_a_live_host_is_kept() {
    let r = reg("fj-1", "h1", RunnerStatus::Offline, false);
    assert!(names(&plan_cleanup(&[r], &live(), &id())).is_empty());
}

#[test]
fn busy_runner_is_kept() {
    let r = reg("fj-1", "gone", RunnerStatus::Offline, true);
    assert!(names(&plan_cleanup(&[r], &live(), &id())).is_empty());
}

#[test]
fn online_runner_is_kept() {
    let r = reg("fj-1", "gone", RunnerStatus::Online, false);
    assert!(names(&plan_cleanup(&[r], &live(), &id())).is_empty());
}

#[test]
fn unreadable_host_list_refuses() {
    let p = plan_cleanup(&[orphan("fj-1")], &LiveHosts::Unreadable, &id());
    assert_eq!(p, Err(CleanupError::HostListUnreadable));
}

#[test]
fn empty_host_list_refuses() {
    let p = plan_cleanup(&[orphan("fj-1")], &LiveHosts::Complete(vec![]), &id());
    assert_eq!(p, Err(CleanupError::EmptyHostList));
}

#[test]
fn malformed_live_host_id_refuses() {
    let h = LiveHosts::Complete(vec!["h 1".into()]);
    assert_eq!(
        plan_cleanup(&[orphan("fj-1")], &h, &id()),
        Err(CleanupError::MalformedHostId("h 1".into()))
    );
}

#[test]
fn bad_identity_refuses() {
    let bad = PoolIdentity {
        prefix: String::new(),
        label: "l".into(),
    };
    assert_eq!(
        plan_cleanup(&[], &live(), &bad),
        Err(CleanupError::BadIdentity)
    );
}

#[test]
fn name_must_match_exact_prefix_rule() {
    for n in ["fj", "fj-", "fjx-1", "other-fj-1", "FJ-1", "fj1"] {
        assert!(
            names(&plan_cleanup(&[orphan(n)], &live(), &id())).is_empty(),
            "{n}"
        );
    }
}

#[test]
fn runner_without_pool_label_is_kept() {
    let mut r = orphan("fj-1");
    r.labels = vec!["other".into()];
    assert!(names(&plan_cleanup(&[r], &live(), &id())).is_empty());
}

#[test]
fn malformed_runner_name_or_host_is_never_planned() {
    let a = orphan("fj-1 x");
    let b = reg("fj-2", "", RunnerStatus::Offline, false);
    let c = reg("fj-3", "g\u{e9}ne", RunnerStatus::Offline, false);
    assert!(names(&plan_cleanup(&[a, b, c], &live(), &id())).is_empty());
}

#[test]
fn cap_limits_one_sweep_and_counts_deferred() {
    let rs: Vec<_> = (0..MAX_DEREGISTER_PER_SWEEP + 3)
        .map(|i| orphan(&format!("fj-{i}")))
        .collect();
    let p = plan_cleanup(&rs, &live(), &id()).unwrap();
    assert_eq!(p.deregister.len(), MAX_DEREGISTER_PER_SWEEP);
    assert_eq!(p.deregister[0], "fj-0");
    assert_eq!(p.deferred, 3);
}

#[test]
fn property_busy_or_online_never_planned() {
    for bits in 0u8..64 {
        let status = if bits & 1 == 0 {
            RunnerStatus::Offline
        } else {
            RunnerStatus::Online
        };
        let busy = bits & 2 != 0;
        let host = if bits & 4 != 0 { "h1" } else { "gone" };
        let mut r = reg("fj-1", host, status, busy);
        if bits & 8 != 0 {
            r.labels.clear();
        }
        let planned = !names(&plan_cleanup(&[r], &live(), &id())).is_empty();
        let eligible = bits & 0b1111 == 0b0000;
        assert_eq!(planned, eligible, "bits {bits:06b}");
        if status == RunnerStatus::Online || busy {
            assert!(!planned);
        }
    }
}

#[test]
fn nested_prefix_pool_runner_is_not_planned() {
    let mut r = orphan("fj-gpu-1");
    r.labels = vec!["forjar-pool".into()];
    assert!(names(&plan_cleanup(&[r], &live(), &id())).is_empty());
}

#[test]
fn host_id_case_difference_counts_as_live() {
    let r = reg("fj-1", "H1", RunnerStatus::Offline, false);
    assert!(names(&plan_cleanup(&[r], &live(), &id())).is_empty());
}

#[test]
fn duplicate_names_are_counted_once() {
    let p = plan_cleanup(&[orphan("fj-1"), orphan("fj-1")], &live(), &id()).unwrap();
    assert_eq!(p.deregister, vec!["fj-1"]);
}

#[test]
fn duplicate_name_with_a_busy_twin_is_not_planned() {
    let busy = reg("fj-1", "gone", RunnerStatus::Offline, true);
    assert!(names(&plan_cleanup(&[orphan("fj-1"), busy], &live(), &id())).is_empty());
}
