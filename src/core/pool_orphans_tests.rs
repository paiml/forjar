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
fn one_sweep_leaves_no_orphans_on_a_synthetic_set() {
    let runners = vec![
        r("r1", "h1", false),
        r("r2", "gone", false),
        r("r3", "zzz", false),
        r("r4", "h2", true),
    ];
    let hosts = vec![h("h1", 0), h("h2", 9999), h("h3", 600), h("h4", 700)];
    let s = sweep(&complete(runners.clone()), &hosts).unwrap();
    assert_eq!(s.deregister_runners, vec!["r2", "r3"]);
    assert_eq!(s.terminate_hosts, vec!["h3", "h4"]);
    let runners: Vec<Runner> = runners
        .into_iter()
        .filter(|x| !s.deregister_runners.contains(&x.name))
        .collect();
    let hosts: Vec<Host> = hosts
        .into_iter()
        .filter(|x| !s.terminate_hosts.contains(&x.id))
        .collect();
    assert_eq!(sweep(&complete(runners), &hosts), Ok(Sweep::default()));
}

#[test]
fn busy_runner_is_never_selected_nor_its_host() {
    let l = complete(vec![r("busy", "h1", true), r("busy-ghost", "zzz", true)]);
    assert_eq!(sweep(&l, &[h("h1", 99_999)]), Ok(Sweep::default()));
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
    let l = complete(vec![r("n", "zzz", false), r("n", "yyy", false)]);
    assert_eq!(
        sweep(&l, &[h("a", 0)]),
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
fn case_only_host_id_mismatch_is_refused_not_guessed() {
    let l = complete(vec![r("x", "I-ABC", true)]);
    assert_eq!(
        sweep(&l, &[h("i-abc", 99_999)]),
        Err(SweepError::NearMatchHostId {
            runner: "x".into(),
            host: "i-abc".into()
        })
    );
}

#[test]
fn prefix_host_id_mismatch_is_refused_in_both_directions() {
    let a = complete(vec![r("x", "forjar-i-abc", true)]);
    assert!(matches!(
        sweep(&a, &[h("forjar", 99_999)]),
        Err(SweepError::NearMatchHostId { .. })
    ));
    let b = complete(vec![r("x", "i-ab", true)]);
    assert!(matches!(
        sweep(&b, &[h("i-abc", 99_999)]),
        Err(SweepError::NearMatchHostId { .. })
    ));
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
fn outputs_are_deduplicated() {
    // Reachable only via push_unique; pin it directly.
    let mut v = vec![];
    push_unique(&mut v, "a");
    push_unique(&mut v, "a");
    push_unique(&mut v, "b");
    assert_eq!(v, vec!["a", "b"]);
}

#[test]
fn suffix_and_inner_host_id_mismatch_is_refused_in_both_directions() {
    for (runner, host) in [
        ("forjar-i-abc", "i-abc"),
        ("i-abc", "forjar-i-abc"),
        ("x-i-abc-y", "i-abc"),
    ] {
        let l = complete(vec![r("x", runner, true)]);
        assert!(
            matches!(
                sweep(&l, &[h(host, 99_999)]),
                Err(SweepError::NearMatchHostId { .. })
            ),
            "{runner} vs {host}"
        );
    }
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
