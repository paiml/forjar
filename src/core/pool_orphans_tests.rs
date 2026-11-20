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
        age_secs: 99_999,
    }
}
fn ha(id: &str, idle: u64, age: u64) -> Host {
    Host {
        id: id.into(),
        idle_secs: idle,
        age_secs: age,
    }
}
fn complete(v: Vec<Runner>) -> RunnerListing {
    RunnerListing::Complete(v)
}

#[test]
fn host_with_an_idle_runner_is_kept() {
    let s = sweep(&complete(vec![r("r", "h1", false)]), &[h("h1", 99_999)]).unwrap();
    assert!(s.terminate_hosts.is_empty());
}

#[test]
fn idle_rule_boundary_is_inclusive() {
    let hosts = vec![
        h("a", SCALE_DOWN_QUIET_SECS - 1),
        h("b", SCALE_DOWN_QUIET_SECS),
    ];
    let s = sweep(&complete(vec![]), &hosts).unwrap();
    assert_eq!(s.terminate_hosts, vec!["b"]);
}

#[test]
fn empty_inputs_sweep_nothing() {
    assert_eq!(sweep(&complete(vec![]), &[]), Ok(Sweep::default()));
}

#[test]
fn partial_listing_is_refused() {
    assert_eq!(
        sweep(&RunnerListing::Partial, &[h("a", 99_999)]),
        Err(SweepError::IncompleteListing)
    );
}

#[test]
fn empty_host_id_is_refused() {
    assert_eq!(
        sweep(&complete(vec![]), &[h("", 99_999)]),
        Err(SweepError::EmptyHostId)
    );
}

#[test]
fn empty_runner_host_id_is_refused() {
    assert_eq!(
        sweep(&complete(vec![r("x", "", false)]), &[h("a", 0)]),
        Err(SweepError::EmptyRunnerHostId { runner: "x".into() })
    );
}

#[test]
fn duplicate_host_ids_are_refused() {
    assert_eq!(
        sweep(&complete(vec![]), &[h("a", 99_999), h("a", 99_999)]),
        Err(SweepError::DuplicateHostId("a".into()))
    );
}

#[test]
fn duplicate_runner_names_are_refused() {
    let l = complete(vec![r("n", "a", false), r("n", "b", false)]);
    assert_eq!(
        sweep(&l, &[h("a", 0), h("b", 0)]),
        Err(SweepError::DuplicateRunnerName("n".into()))
    );
}

#[test]
fn just_launched_host_is_not_terminated_on_an_empty_listing() {
    let hosts = [ha(
        "new",
        REGISTRATION_GRACE_SECS - 1,
        REGISTRATION_GRACE_SECS - 1,
    )];
    assert_eq!(sweep(&complete(vec![]), &hosts), Ok(Sweep::default()));
}

#[test]
fn grace_boundary_is_inclusive() {
    let hosts = [ha("old", REGISTRATION_GRACE_SECS, REGISTRATION_GRACE_SECS)];
    let s = sweep(&complete(vec![]), &hosts).unwrap();
    assert_eq!(s.terminate_hosts, vec!["old"]);
}

#[test]
fn exact_match_is_not_a_near_match() {
    let l = complete(vec![r("x", "i-abc", true)]);
    assert_eq!(
        sweep(&l, &[h("i-abc", 99_999), h("i-abcd", 0)]),
        Ok(Sweep::default())
    );
}

#[test]
fn idle_longer_than_age_is_refused() {
    let hosts = [ha("a", 1000, 999)];
    assert_eq!(
        sweep(&complete(vec![]), &hosts),
        Err(SweepError::IdleExceedsAge("a".into()))
    );
    assert!(sweep(&complete(vec![]), &[ha("a", 1000, 1000)]).is_ok());
}

#[test]
fn host_with_busy_runner_is_never_terminated_even_when_old_and_idle() {
    let l = complete(vec![r("b", "a", true)]);
    let s = sweep(&l, &[ha("a", 99_999, 99_999)]).unwrap();
    assert!(s.terminate_hosts.is_empty());
}

#[test]
fn unknown_runner_host_is_reported_and_orphans_are_still_terminated() {
    let l = complete(vec![r("ghost", "gone", false), r("b", "I-0ABC", true)]);
    let s = sweep(&l, &[h("o1", 99_999), h("i-0abc", 99_999), h("o2", 99_999)]).unwrap();
    assert_eq!(s.terminate_hosts, ["o1", "o2"]);
    assert_eq!(s.unknown_runners.len(), 2);
}

#[test]
fn host_ids_equal_ignoring_case_are_duplicates() {
    let l = complete(vec![r("b", "i-0abc", true)]);
    let hosts = [ha("i-0abc", 1, 99_999), ha("I-0ABC", 99_999, 99_999)];
    assert!(matches!(
        sweep(&l, &hosts),
        Err(SweepError::DuplicateHostId(_))
    ));
}

#[test]
fn ids_with_whitespace_or_control_characters_are_refused() {
    for bad in [
        "i-0abc\n",
        " i-0abc",
        "i-0abc ",
        "i-0 abc",
        "i-0\tabc",
        "i-0\u{0}abc",
    ] {
        let hosts = [h(bad, 99_999)];
        assert_eq!(
            sweep(&complete(vec![]), &hosts),
            Err(SweepError::MalformedId(bad.into())),
            "host {bad:?}"
        );
        let l = complete(vec![r("x", bad, true)]);
        assert_eq!(
            sweep(&l, &[h("i-0abc", 0)]),
            Err(SweepError::MalformedRunnerHostId {
                runner: "x".into(),
                host: bad.into()
            }),
            "runner host {bad:?}"
        );
    }
}

#[test]
fn one_sweep_leaves_no_orphans_on_a_synthetic_set() {
    let runners = vec![
        r("r1", "h1", false),
        r("r4", "h2", true),
        r("r5", "h5", false),
    ];
    let hosts = vec![
        h("h1", 0),
        h("h2", 9999),
        h("h3", 600),
        h("h4", 700),
        h("h5", 0),
    ];
    let s = sweep(&complete(runners.clone()), &hosts).unwrap();
    assert_eq!(s.terminate_hosts, vec!["h3", "h4"]);
    let hosts: Vec<Host> = hosts
        .into_iter()
        .filter(|x| !s.terminate_hosts.contains(&x.id))
        .collect();
    assert_eq!(sweep(&complete(runners), &hosts), Ok(Sweep::default()));
}

#[test]
fn busy_runner_keeps_its_host() {
    let l = complete(vec![r("busy", "h1", true)]);
    assert_eq!(sweep(&l, &[h("h1", 99_999)]), Ok(Sweep::default()));
}

#[test]
fn exact_match_with_a_longer_sibling_id_is_fine() {
    let l = complete(vec![r("x", "i-abc", true)]);
    let s = sweep(&l, &[h("i-abc", 99_999), h("i-abcd", 99_999)]).unwrap();
    assert_eq!(s.terminate_hosts, vec!["i-abcd"]);
}

#[test]
fn busy_and_idle_runners_on_a_host_id_with_inner_whitespace_are_refused() {
    let hosts = [ha("i-0abc", 99_999, 99_999)];
    for busy in [true, false] {
        let l = complete(vec![r("b", "i-0 abc", busy)]);
        assert_eq!(
            sweep(&l, &hosts),
            Err(SweepError::MalformedRunnerHostId {
                runner: "b".into(),
                host: "i-0 abc".into()
            })
        );
    }
}

#[test]
fn non_ascii_ids_are_refused() {
    for bad in ["i-0\u{200b}abc", "i-\u{456}-0abc"] {
        let hosts = [h(bad, 99_999)];
        assert_eq!(
            sweep(&complete(vec![]), &hosts),
            Err(SweepError::MalformedId(bad.into())),
            "host {bad:?}"
        );
        let l = complete(vec![r("x", bad, true)]);
        assert_eq!(
            sweep(&l, &[h("i-0abc", 0)]),
            Err(SweepError::MalformedRunnerHostId {
                runner: "x".into(),
                host: bad.into()
            }),
            "runner host {bad:?}"
        );
    }
}

#[test]
fn runner_names_differing_only_in_case_are_duplicates() {
    let l = complete(vec![r("A", "a", false), r("a", "b", false)]);
    let e = sweep(&l, &[h("a", 0), h("b", 0)]);
    assert_eq!(e, Err(SweepError::DuplicateRunnerName("a".into())));
}

#[test]
fn blank_runner_names_are_refused() {
    for name in ["", "  "] {
        let l = complete(vec![r(name, "a", false)]);
        let e = sweep(&l, &[h("a", 0)]);
        assert_eq!(e, Err(SweepError::EmptyRunnerName), "{name:?}");
    }
}

#[test]
fn cleanup_deregisters_idle_runners_on_unlisted_hosts_only() {
    let l = complete(vec![
        r("ghost", "gone", false),
        r("kept", "I-0ABC", false),
        r("ghost2", "gone2", false),
    ]);
    let c = cleanup_orphan_runners(&l, &[h("i-0abc", 99_999)]).unwrap();
    assert_eq!(c.deregister_runners, ["ghost", "ghost2"]);
}

#[test]
fn cleanup_terminates_nothing_and_leaves_a_sweep_that_proceeds() {
    let hosts = [h("o1", 99_999), h("live", 99_999)];
    let before = vec![r("ghost", "gone", false), r("live-r", "live", false)];
    let c = cleanup_orphan_runners(&complete(before.clone()), &hosts).unwrap();
    assert_eq!(c.deregister_runners, ["ghost"]);
    let after: Vec<Runner> = before
        .into_iter()
        .filter(|x| !c.deregister_runners.contains(&x.name))
        .collect();
    let s = sweep(&complete(after), &hosts).unwrap();
    assert!(s.unknown_runners.is_empty());
    assert_eq!(s.terminate_hosts, ["o1"]);
}

#[test]
fn cleanup_refuses_when_an_unlisted_host_runner_is_busy() {
    let l = complete(vec![r("ghost", "gone", false), r("work", "gone2", true)]);
    assert_eq!(
        cleanup_orphan_runners(&l, &[h("o1", 99_999)]),
        Err(SweepError::BusyRunnerOnUnlistedHost {
            runner: "work".into(),
            host: "gone2".into()
        })
    );
}

#[test]
fn cleanup_refuses_a_partial_listing_and_invalid_input() {
    assert_eq!(
        cleanup_orphan_runners(&RunnerListing::Partial, &[]),
        Err(SweepError::IncompleteListing)
    );
    let l = complete(vec![r("g", "", false)]);
    assert!(cleanup_orphan_runners(&l, &[]).is_err());
}
