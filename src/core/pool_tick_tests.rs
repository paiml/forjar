use super::pool_interruption::{CapacityMode, InterruptionError, RerunDecision};
use super::pool_jit::{JitConfigError, JitRunnerRequest, MAX_TOKEN_TTL_SECS};
use super::pool_orphans::{
    cleanup_orphan_runners, Host, Runner, RunnerListing, SweepError, REGISTRATION_GRACE_SECS,
};
use super::pool_plan::{PlanError, QueueEntry, SeatState};
use super::pool_reconcile::{HARD_TTL_SECS, SCALE_DOWN_QUIET_SECS, SCALE_UP_AGE_SECS};
use super::pool_tick::*;
use std::collections::BTreeSet;

const NOW: u64 = 1_000_000;

fn valid_jit() -> JitRunnerRequest {
    JitRunnerRequest {
        labels: vec!["forjar-pool".into(), "x64".into()],
        ephemeral: true,
        token_ttl_secs: MAX_TOKEN_TTL_SECS,
        token_source: "RUNNER_JIT_TOKEN".into(),
    }
}

fn q(labels: &str, queued: u32, oldest_age_secs: u64) -> QueueEntry {
    QueueEntry {
        labels: labels.into(),
        queued,
        oldest_age_secs,
    }
}

fn host_state(
    id: &str,
    labels: &str,
    seat_states: Vec<SeatState>,
    idle_secs: u64,
    age_secs: u64,
) -> HostState {
    HostState {
        host: Host {
            id: id.into(),
            idle_secs,
            age_secs,
        },
        labels: labels.into(),
        seat_states,
        spot_notice_secs_left: None,
    }
}

fn runner(name: &str, host_id: &str, busy: bool) -> Runner {
    Runner {
        name: name.into(),
        host_id: host_id.into(),
        busy,
    }
}

fn run_tick(
    queue: &[QueueEntry],
    hosts: &[HostState],
    runners: &RunnerListing,
    jit: &JitRunnerRequest,
    guards: TickGuards,
) -> Result<TickDecision, TickError> {
    reconcile_tick(&TickInput {
        now_secs: NOW,
        queue,
        hosts,
        runners,
        launches_24h: &[],
        interruptions_24h: &[],
        interrupted_jobs: &[],
        jit,
        guards,
    })
}

fn assert_structural_invariants(d: &TickDecision) {
    let drain_set: BTreeSet<&str> = d.drain_hosts.iter().map(String::as_str).collect();
    let term_set: BTreeSet<&str> = d.terminate_hosts.iter().map(String::as_str).collect();
    let dereg_set: BTreeSet<&str> = d.deregister_runners.iter().map(String::as_str).collect();
    assert_eq!(drain_set.len(), d.drain_hosts.len(), "duplicate drain");
    assert_eq!(term_set.len(), d.terminate_hosts.len(), "duplicate term");
    assert_eq!(dereg_set.len(), d.deregister_runners.len(), "dup dereg");
    assert!(
        drain_set.is_disjoint(&term_set),
        "drain and terminate must be disjoint"
    );
    for order in &d.launch_hosts {
        assert!(order.count > 0, "zero-count launch order");
        assert_eq!(order.mode, d.capacity_mode, "launch mode mismatch");
    }
}

#[test]
fn case1_scale_up_on_aged_queued_job_over_5_min() {
    let jit = valid_jit();
    let hosts = [host_state("h1", "x64", vec![SeatState::Idle], 301, 400)];
    let runners = RunnerListing::Complete(vec![runner("r1", "h1", false)]);
    let d_over = run_tick(
        &[q("x64", 1, SCALE_UP_AGE_SECS + 1)],
        &hosts,
        &runners,
        &jit,
        TickGuards::default(),
    )
    .expect("over 5 min tick");
    assert_structural_invariants(&d_over);
    assert_eq!(
        d_over.launch_hosts,
        vec![LaunchOrder {
            labels: "x64".into(),
            count: 1,
            mode: CapacityMode::Spot,
        }]
    );

    let d_eq = run_tick(
        &[q("x64", 1, SCALE_UP_AGE_SECS)],
        &hosts,
        &runners,
        &jit,
        TickGuards::default(),
    )
    .expect("at 300s boundary tick");
    assert_structural_invariants(&d_eq);
    assert!(d_eq.launch_hosts.is_empty());
}

#[test]
fn case2_scale_up_when_queue_exceeds_idle_seats_clamped_by_max_hosts() {
    let jit = valid_jit();
    let queue = [q("x64", 3, 10)];
    let hosts = [host_state("h1", "x64", vec![SeatState::Idle], 10, 100)];
    let runners = RunnerListing::Complete(vec![runner("r1", "h1", false)]);

    let d4 = run_tick(
        &queue,
        &hosts,
        &runners,
        &jit,
        TickGuards {
            max_hosts: 4,
            ..TickGuards::default()
        },
    )
    .expect("max_hosts 4");
    assert_structural_invariants(&d4);
    assert_eq!(
        d4.launch_hosts,
        vec![LaunchOrder {
            labels: "x64".into(),
            count: 2,
            mode: CapacityMode::Spot,
        }]
    );

    let d2 = run_tick(
        &queue,
        &hosts,
        &runners,
        &jit,
        TickGuards {
            max_hosts: 2,
            ..TickGuards::default()
        },
    )
    .expect("max_hosts 2");
    assert_structural_invariants(&d2);
    assert_eq!(
        d2.launch_hosts,
        vec![LaunchOrder {
            labels: "x64".into(),
            count: 1,
            mode: CapacityMode::Spot,
        }]
    );
}

#[test]
fn case3_scale_to_zero_after_10_min_quiet_and_empty_queue() {
    let jit = valid_jit();
    let runners =
        RunnerListing::Complete(vec![runner("r1", "h1", false), runner("r2", "h1", false)]);

    for idle in [0, 150, 300, SCALE_DOWN_QUIET_SECS - 1] {
        let hosts = [host_state(
            "h1",
            "x64",
            vec![SeatState::Idle, SeatState::Idle],
            idle,
            1000 + idle,
        )];
        let d = run_tick(&[], &hosts, &runners, &jit, TickGuards::default())
            .expect("quiet progression");
        assert_structural_invariants(&d);
        assert!(d.terminate_hosts.is_empty(), "held at idle={idle}");
    }

    let quiet_hosts = [host_state(
        "h1",
        "x64",
        vec![SeatState::Idle, SeatState::Idle],
        SCALE_DOWN_QUIET_SECS,
        1600,
    )];
    let d_term =
        run_tick(&[], &quiet_hosts, &runners, &jit, TickGuards::default()).expect("scale to zero");
    assert_structural_invariants(&d_term);
    assert_eq!(d_term.terminate_hosts, vec!["h1"]);

    let busy_hosts = [host_state(
        "h1",
        "x64",
        vec![SeatState::Idle, SeatState::Busy],
        SCALE_DOWN_QUIET_SECS,
        1600,
    )];
    let empty_runners = RunnerListing::Complete(vec![]);
    let d_busy = run_tick(
        &[],
        &busy_hosts,
        &empty_runners,
        &jit,
        TickGuards::default(),
    )
    .expect("busy seat held");
    assert_structural_invariants(&d_busy);
    assert!(d_busy.terminate_hosts.is_empty());
}

#[test]
fn case4_hard_12h_ttl_with_idle_vs_busy_seats() {
    let jit = valid_jit();
    let queue = [q("x64", 2, 10)];
    let runners = RunnerListing::Complete(vec![runner("r1", "h1", false)]);
    let idle_ttl = [host_state(
        "h1",
        "x64",
        vec![SeatState::Idle],
        0,
        HARD_TTL_SECS,
    )];
    let d_idle =
        run_tick(&queue, &idle_ttl, &runners, &jit, TickGuards::default()).expect("idle ttl");
    assert_structural_invariants(&d_idle);
    assert_eq!(d_idle.terminate_hosts, vec!["h1"]);
    assert!(d_idle.drain_hosts.is_empty());
    assert!(d_idle.launch_hosts.is_empty());

    let busy_ttl = [host_state(
        "h1",
        "x64",
        vec![SeatState::Busy],
        0,
        HARD_TTL_SECS,
    )];
    let busy_runners = RunnerListing::Complete(vec![runner("r1", "h1", true)]);
    let d_busy = run_tick(
        &queue,
        &busy_ttl,
        &busy_runners,
        &jit,
        TickGuards::default(),
    )
    .expect("busy ttl");
    assert_structural_invariants(&d_busy);
    assert_eq!(d_busy.drain_hosts, vec!["h1"]);
    assert!(d_busy.terminate_hosts.is_empty());
    assert_eq!(d_busy.deregister_runners, vec!["r1"]);
}

#[test]
fn case5_ephemeral_jit_and_no_disk_token_enforced() {
    let runners = RunnerListing::Complete(vec![]);
    let cases = [
        (
            JitRunnerRequest {
                token_ttl_secs: MAX_TOKEN_TTL_SECS + 1,
                ..valid_jit()
            },
            TickError::Jit(JitConfigError::TtlTooLong { secs: 3601 }),
        ),
        (
            JitRunnerRequest {
                ephemeral: false,
                ..valid_jit()
            },
            TickError::Jit(JitConfigError::NotEphemeral),
        ),
        (
            JitRunnerRequest {
                token_source: "/run/secrets/runner.tok".into(),
                ..valid_jit()
            },
            TickError::Jit(JitConfigError::TokenSourceIsPath),
        ),
    ];
    for (bad, expected) in cases {
        let err =
            run_tick(&[], &[], &runners, &bad, TickGuards::default()).expect_err("bad jit refused");
        assert_eq!(err, expected);
        assert!(!format!("{err:?}").contains("/run/secrets"));
    }
}

#[test]
fn case6_spot_2min_notice_drains_deregisters_reruns_and_excludes_from_launches() {
    let jit = valid_jit();
    let mut h = host_state("h-spot", "x64", vec![SeatState::Idle], 5, 200);
    h.spot_notice_secs_left = Some(120);
    let hosts = [h];
    let runners = RunnerListing::Complete(vec![runner("r-spot", "h-spot", true)]);
    let jobs = [
        InterruptedJob {
            job_id: "j1".into(),
            prior_reruns: 0,
        },
        InterruptedJob {
            job_id: "j2".into(),
            prior_reruns: 1,
        },
    ];
    let queue = [q("x64", 1, 10)];
    let d = reconcile_tick(&TickInput {
        now_secs: NOW,
        queue: &queue,
        hosts: &hosts,
        runners: &runners,
        launches_24h: &[],
        interruptions_24h: &[],
        interrupted_jobs: &jobs,
        jit: &jit,
        guards: TickGuards::default(),
    })
    .expect("spot notice tick");
    assert_structural_invariants(&d);
    assert_eq!(d.drain_hosts, vec!["h-spot"]);
    assert!(d.terminate_hosts.is_empty());
    assert_eq!(d.deregister_runners, vec!["r-spot"]);
    assert_eq!(
        d.rerun_jobs,
        vec![
            ("j1".into(), RerunDecision::Rerun),
            ("j2".into(), RerunDecision::GiveUp),
        ]
    );
    assert_eq!(
        d.launch_hosts,
        vec![LaunchOrder {
            labels: "x64".into(),
            count: 1,
            mode: CapacityMode::Spot,
        }]
    );
}

#[test]
fn case7_24h_interruption_rate_over_5_percent_falls_back_to_ondemand() {
    let jit = valid_jit();
    let queue = [q("x64", 1, 10)];
    let runners = RunnerListing::Complete(vec![]);
    let launches = vec![NOW - 10; 100];
    let six_hits = vec![NOW - 5; 6];
    let five_hits = vec![NOW - 5; 5];

    let d_od = reconcile_tick(&TickInput {
        now_secs: NOW,
        queue: &queue,
        hosts: &[],
        runners: &runners,
        launches_24h: &launches,
        interruptions_24h: &six_hits,
        interrupted_jobs: &[],
        jit: &jit,
        guards: TickGuards::default(),
    })
    .expect("6% fallback");
    assert_structural_invariants(&d_od);
    assert_eq!(d_od.capacity_mode, CapacityMode::OnDemand);
    assert_eq!(d_od.launch_hosts[0].mode, CapacityMode::OnDemand);

    let d_spot = reconcile_tick(&TickInput {
        now_secs: NOW,
        queue: &queue,
        hosts: &[],
        runners: &runners,
        launches_24h: &launches,
        interruptions_24h: &five_hits,
        interrupted_jobs: &[],
        jit: &jit,
        guards: TickGuards::default(),
    })
    .expect("5% stays spot");
    assert_structural_invariants(&d_spot);
    assert_eq!(d_spot.capacity_mode, CapacityMode::Spot);
    assert_eq!(d_spot.launch_hosts[0].mode, CapacityMode::Spot);

    let future = [NOW + 1];
    let err = reconcile_tick(&TickInput {
        now_secs: NOW,
        queue: &queue,
        hosts: &[],
        runners: &runners,
        launches_24h: &launches,
        interruptions_24h: &future,
        interrupted_jobs: &[],
        jit: &jit,
        guards: TickGuards::default(),
    })
    .expect_err("future timestamp fails closed");
    assert_eq!(
        err,
        TickError::Interruption(InterruptionError::FutureTimestamp {
            ts: NOW + 1,
            now_secs: NOW,
        })
    );
}

#[test]
fn case8_orphan_sweep_and_fail_closed_on_partial_or_invalid_inputs() {
    let jit = valid_jit();
    let hosts = [host_state(
        "orphan-h",
        "x64",
        vec![SeatState::Idle],
        SCALE_DOWN_QUIET_SECS,
        REGISTRATION_GRACE_SECS,
    )];
    let runners = RunnerListing::Complete(vec![runner("ghost-r", "dead-h", false)]);
    let d = run_tick(&[], &hosts, &runners, &jit, TickGuards::default()).expect("orphan cleanup");
    assert_structural_invariants(&d);
    assert_eq!(d.terminate_hosts, vec!["orphan-h"]);
    assert_eq!(d.deregister_runners, vec!["ghost-r"]);

    let post_cleanup = cleanup_orphan_runners(&RunnerListing::Complete(vec![]), &[])
        .expect("0 orphaned runners left");
    assert!(post_cleanup.deregister_runners.is_empty());

    let err_partial = run_tick(
        &[],
        &hosts,
        &RunnerListing::Partial,
        &jit,
        TickGuards::default(),
    )
    .expect_err("partial listing fails closed");
    assert_eq!(err_partial, TickError::Sweep(SweepError::IncompleteListing));

    let empty_runners = RunnerListing::Complete(vec![]);
    let err_seats = run_tick(
        &[],
        &[host_state("empty-seats", "x64", vec![], 0, 10)],
        &empty_runners,
        &jit,
        TickGuards::default(),
    )
    .expect_err("empty seats fail closed");
    assert_eq!(err_seats, TickError::InvalidHostSeats("empty-seats".into()));

    let err_plan = run_tick(
        &[q("", 1, 10)],
        &[],
        &empty_runners,
        &jit,
        TickGuards::default(),
    )
    .expect_err("empty label fails closed");
    assert_eq!(err_plan, TickError::Plan(PlanError::EmptyLabels));
}
