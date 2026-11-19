use super::*;

const C: Caps = Caps {
    max_seats: 4,
    max_seats_per_labels: 3,
    max_launch_per_tick: 10,
    max_age_secs: 43_200,
    idle_retire_secs: 600,
};

fn seat(id: &str, labels: &str, state: SeatState, age: u64, idle: u64) -> Seat {
    Seat {
        id: id.into(),
        labels: labels.into(),
        state,
        age_secs: age,
        idle_secs: idle,
    }
}
fn q(labels: &str, queued: u32, oldest: u64) -> QueueEntry {
    QueueEntry {
        labels: labels.into(),
        queued,
        oldest_age_secs: oldest,
    }
}
fn launched(a: &[Action]) -> u32 {
    a.iter()
        .map(|x| {
            if let Action::Launch { count, .. } = x {
                *count
            } else {
                0
            }
        })
        .sum()
}

#[test]
fn empty_queue_no_seats_is_nothing() {
    let p = plan(
        &Snapshot {
            queue: vec![],
            seats: vec![],
        },
        &C,
    )
    .unwrap();
    assert!(matches!(p.as_slice(), [Action::Nothing { .. }]));
}

#[test]
fn burst_above_cap_is_clamped_to_label_cap() {
    let s = Snapshot {
        queue: vec![q("a", 50, 10)],
        seats: vec![],
    };
    assert_eq!(launched(&plan(&s, &C).unwrap()), 3);
}

#[test]
fn burst_clamped_by_global_cap_and_tick_budget() {
    let s = Snapshot {
        queue: vec![q("a", 9, 1), q("b", 9, 1)],
        seats: vec![],
    };
    assert_eq!(launched(&plan(&s, &C).unwrap()), 4);
    let c = Caps {
        max_launch_per_tick: 2,
        ..C
    };
    assert_eq!(launched(&plan(&s, &c).unwrap()), 2);
}

#[test]
fn seats_past_max_age_retire_when_idle_drain_when_busy() {
    let s = Snapshot {
        queue: vec![],
        seats: vec![
            seat("i", "a", SeatState::Idle, 43_200, 0),
            seat("b", "a", SeatState::Busy, 50_000, 0),
        ],
    };
    let p = plan(&s, &C).unwrap();
    assert!(matches!(&p[0], Action::Retire { seat, .. } if seat == "i"));
    assert!(matches!(&p[1], Action::Drain { seat, .. } if seat == "b"));
}

#[test]
fn idle_seat_with_empty_queue_retires_after_timeout() {
    let s = Snapshot {
        queue: vec![],
        seats: vec![seat("i", "a", SeatState::Idle, 100, 600)],
    };
    assert!(matches!(
        plan(&s, &C).unwrap().as_slice(),
        [Action::Retire { .. }]
    ));
    let s = Snapshot {
        queue: vec![],
        seats: vec![seat("i", "a", SeatState::Idle, 100, 599)],
    };
    assert!(matches!(
        plan(&s, &C).unwrap().as_slice(),
        [Action::Nothing { .. }]
    ));
}

#[test]
fn mixed_labels_serve_only_starved_set() {
    let s = Snapshot {
        queue: vec![q("a", 1, 1), q("b", 2, 1)],
        seats: vec![seat("i", "a", SeatState::Idle, 100, 0)],
    };
    let p = plan(&s, &C).unwrap();
    assert!(matches!(p.as_slice(), [Action::Launch { labels, count: 2, .. }] if labels == "b"));
}

#[test]
fn aged_job_with_only_busy_seats_launches_one() {
    let s = Snapshot {
        queue: vec![q("a", 1, 301)],
        seats: vec![seat("b", "a", SeatState::Busy, 10, 0)],
    };
    assert_eq!(launched(&plan(&s, &C).unwrap()), 1);
}

#[test]
fn zero_caps_never_launch() {
    let s = Snapshot {
        queue: vec![q("a", 5, 999)],
        seats: vec![],
    };
    for c in [
        Caps { max_seats: 0, ..C },
        Caps {
            max_seats_per_labels: 0,
            ..C
        },
        Caps {
            max_launch_per_tick: 0,
            ..C
        },
    ] {
        let p = plan(&s, &c).unwrap();
        assert!(matches!(p.as_slice(), [Action::Nothing { .. }]));
    }
}

#[test]
fn bad_input_is_typed_error() {
    let dup = Snapshot {
        queue: vec![],
        seats: vec![
            seat("x", "a", SeatState::Idle, 1, 1),
            seat("x", "a", SeatState::Idle, 1, 1),
        ],
    };
    assert_eq!(plan(&dup, &C), Err(PlanError::DuplicateSeat("x".into())));
    let e = Snapshot {
        queue: vec![q("", 1, 1)],
        seats: vec![],
    };
    assert_eq!(plan(&e, &C), Err(PlanError::EmptyLabels));
    let d = Snapshot {
        queue: vec![q("a", 1, 1), q("a", 2, 1)],
        seats: vec![],
    };
    assert_eq!(
        plan(&d, &C),
        Err(PlanError::DuplicateQueueEntry("a".into()))
    );
}

#[test]
fn property_plan_never_exceeds_caps() {
    let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = |m: u64| {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x % m
    };
    for _ in 0..2000 {
        let c = Caps {
            max_seats: next(6) as u32,
            max_seats_per_labels: next(5) as u32,
            max_launch_per_tick: next(6) as u32,
            ..C
        };
        let names = ["a", "b", "c"];
        let queue: Vec<_> = names
            .iter()
            .map(|n| q(n, next(20) as u32, next(900)))
            .collect();
        let seats: Vec<_> = (0..next(7))
            .map(|i| {
                let st = if next(2) == 0 {
                    SeatState::Idle
                } else {
                    SeatState::Busy
                };
                seat(
                    &format!("s{i}"),
                    names[next(3) as usize],
                    st,
                    next(60_000),
                    next(1200),
                )
            })
            .collect();
        let s = Snapshot { queue, seats };
        let p = plan(&s, &c).unwrap();
        let retired = p
            .iter()
            .filter(|a| matches!(a, Action::Retire { .. }))
            .count() as u32;
        let live = s.seats.len() as u32 - retired;
        let added = launched(&p);
        assert!(added <= c.max_launch_per_tick);
        // Launches never take the pool past max_seats (seats already over cap stay as-is).
        assert!(added == 0 || live + added <= c.max_seats);
        for n in names {
            let per: u32 = p
                .iter()
                .map(|a| match a {
                    Action::Launch { labels, count, .. } if labels == n => *count,
                    _ => 0,
                })
                .sum();
            let have = s.seats.iter().filter(|x| x.labels == n).count() as u32;
            assert!(per == 0 || have + per <= c.max_seats_per_labels + retired);
        }
    }
}

// ---- review fixes (#1116) ----

use crate::core::pool_reconcile::{decide, PoolAction, PoolLimits, PoolSnapshot, HARD_TTL_SECS};

fn launches(s: &Snapshot, c: &Caps) -> bool {
    launched(&plan(s, c).unwrap()) > 0
}

fn seats_of(idle: u32, busy: u32) -> Vec<Seat> {
    let i = (0..idle).map(|i| seat(&format!("i{i}"), "a", SeatState::Idle, 10, 0));
    let b = (0..busy).map(|i| seat(&format!("b{i}"), "a", SeatState::Busy, 10, 0));
    i.chain(b).collect()
}

/// `plan()` must launch iff `decide()` says scale up, for one label set.
#[test]
fn plan_agrees_with_decide_on_scale_up() {
    let big = Caps {
        max_seats: 50,
        max_seats_per_labels: 50,
        max_launch_per_tick: 50,
        ..C
    };
    assert_eq!(C.max_age_secs, HARD_TTL_SECS);
    for idle in 0..3u32 {
        for busy in 0..2u32 {
            for queued in 0..4u32 {
                // queue ages around SCALE_UP_AGE_SECS x host ages around the hard TTL
                let ages = [0u64, 299, 300, 301, 900].into_iter();
                let hosts = [0, 10, HARD_TTL_SECS - 1, HARD_TTL_SECS, HARD_TTL_SECS + 1];
                for (age, host_age) in ages.flat_map(|a| hosts.map(|h| (a, h))) {
                    let mut seats = seats_of(idle, busy);
                    seats.iter_mut().for_each(|x| x.age_secs = host_age);
                    let oldest = if queued == 0 { 0 } else { age };
                    let s = Snapshot {
                        queue: vec![q("a", queued, oldest)],
                        seats,
                    };
                    let d = decide(
                        &PoolSnapshot {
                            hosts: idle + busy,
                            idle_seats: idle,
                            busy_seats: busy,
                            queued_eligible: queued,
                            oldest_queued_secs: (queued > 0).then_some(oldest),
                            quiet_secs: 0,
                            oldest_host_age_secs: host_age,
                        },
                        &PoolLimits { max_hosts: 99 },
                    );
                    assert_eq!(
                        launches(&s, &big),
                        d == PoolAction::ScaleUp,
                        "idle={idle} busy={busy} queued={queued} age={age} host_age={host_age}"
                    );
                }
            }
        }
    }
}

#[test]
fn aged_job_launches_even_with_an_idle_seat() {
    let s = Snapshot {
        queue: vec![q("a", 1, 301)],
        seats: vec![seat("i", "a", SeatState::Idle, 10, 0)],
    };
    assert_eq!(launched(&plan(&s, &C).unwrap()), 1);
}

#[test]
fn scale_up_age_boundary_matches_decide() {
    let mk = |age| Snapshot {
        queue: vec![q("a", 1, age)],
        seats: vec![seat("i", "a", SeatState::Idle, 10, 0)],
    };
    assert!(!launches(&mk(300), &C), "300s is not aged");
    assert!(launches(&mk(301), &C), "301s is aged");
}

#[test]
fn busy_seat_is_never_retired() {
    for queue in [vec![], vec![q("a", 1, 1)]] {
        for (age, idle) in [(100, 9_999), (43_200, 9_999), (90_000, 0), (90_000, 9_999)] {
            let s = Snapshot {
                queue: queue.clone(),
                seats: vec![seat("b", "a", SeatState::Busy, age, idle)],
            };
            let p = plan(&s, &C).unwrap();
            assert!(
                !p.iter().any(|a| matches!(a, Action::Retire { .. })),
                "busy seat retired: age={age} idle={idle} plan={p:?}"
            );
        }
    }
    let s = Snapshot {
        queue: vec![],
        seats: vec![seat("b", "a", SeatState::Busy, 50_000, 9_999)],
    };
    assert!(matches!(
        plan(&s, &C).unwrap().as_slice(),
        [Action::Drain { .. }]
    ));
}

#[test]
fn idle_seat_is_kept_while_its_label_has_queued_jobs() {
    let s = Snapshot {
        queue: vec![q("a", 1, 1)],
        seats: vec![seat("i", "a", SeatState::Idle, 100, 9_999)],
    };
    let p = plan(&s, &C).unwrap();
    assert!(!p.iter().any(|a| matches!(a, Action::Retire { .. })));
}

/// A retire may fail, so a retired seat frees no room until a later tick.
#[test]
fn retired_seat_does_not_free_room_in_the_same_tick() {
    let c = Caps { max_seats: 1, ..C };
    let s = Snapshot {
        queue: vec![q("b", 1, 1)],
        seats: vec![seat("o", "a", SeatState::Idle, 50_000, 0)],
    };
    let p = plan(&s, &c).unwrap();
    assert_eq!(launched(&p), 0);
    assert!(matches!(&p[0], Action::Retire { .. }));
    let c = Caps {
        max_seats_per_labels: 1,
        ..C
    };
    let s = Snapshot {
        queue: vec![q("a", 2, 1)],
        seats: vec![seat("o", "a", SeatState::Idle, 50_000, 0)],
    };
    assert_eq!(launched(&plan(&s, &c).unwrap()), 0);
}

fn label_launched(p: &[Action], n: &str) -> u32 {
    p.iter()
        .map(|a| match a {
            Action::Launch { labels, count, .. } if labels == n => *count,
            _ => 0,
        })
        .sum()
}

fn assert_caps(s: &Snapshot, c: &Caps, p: &[Action]) {
    let added = launched(p);
    assert!(added <= c.max_launch_per_tick);
    assert!(
        added == 0 || s.seats.len() as u32 + added <= c.max_seats,
        "global cap"
    );
    for n in ["a", "b", "c"] {
        let per = label_launched(p, n);
        let have = s.seats.iter().filter(|x| x.labels == n).count() as u32;
        assert!(
            per == 0 || have + per <= c.max_seats_per_labels,
            "label cap"
        );
    }
}

fn assert_only_idle_retired(s: &Snapshot, p: &[Action]) {
    for a in p {
        if let Action::Retire { seat: id, .. } = a {
            let st = s.seats.iter().find(|x| &x.id == id).map(|x| x.state);
            assert_eq!(st, Some(SeatState::Idle), "retired a busy seat");
        }
    }
}

/// True when some label set has more queued jobs than idle seats (a retiring one counts) and
/// room under its per-label cap.
fn any_starved(s: &Snapshot, c: &Caps) -> bool {
    s.queue.iter().any(|e| {
        let idle_all = s
            .seats
            .iter()
            .filter(|x| x.labels == e.labels && x.state == SeatState::Idle)
            .count() as u32;
        let have = s.seats.iter().filter(|x| x.labels == e.labels).count() as u32;
        e.queued > idle_all && have < c.max_seats_per_labels
    })
}

fn random_snapshot(next: &mut impl FnMut(u64) -> u64) -> (Snapshot, Caps) {
    let names = ["a", "b", "c"];
    let c = Caps {
        max_seats: next(8) as u32,
        max_seats_per_labels: next(6) as u32,
        max_launch_per_tick: next(6) as u32,
        ..C
    };
    let queue: Vec<_> = names
        .iter()
        .map(|n| q(n, next(10) as u32, next(900)))
        .collect();
    let seats: Vec<_> = (0..next(8))
        .map(|i| {
            let st = [SeatState::Idle, SeatState::Busy][next(2) as usize];
            let l = names[next(3) as usize];
            seat(&format!("s{i}"), l, st, next(60_000), next(1200))
        })
        .collect();
    (Snapshot { queue, seats }, c)
}

/// Property: caps hold against seats that exist now, only idle seats retire,
/// and a launch happens when the queue exceeds idle seats and caps allow.
#[test]
fn property_launch_bounds_and_liveness() {
    let mut x: u64 = 0xD1B5_4A32_D192_ED03;
    let mut next = |m: u64| {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x % m
    };
    let mut launch_seen = 0;
    for _ in 0..3000 {
        let (s, c) = random_snapshot(&mut next);
        let p = plan(&s, &c).unwrap();
        assert_caps(&s, &c, &p);
        assert_only_idle_retired(&s, &p);
        let room = (s.seats.len() as u32) < c.max_seats
            && c.max_launch_per_tick > 0
            && !pool_at_ttl(&s, &c);
        if room && any_starved(&s, &c) {
            assert!(launched(&p) > 0, "starved queue with room must launch");
            launch_seen += 1;
        }
    }
    assert!(launch_seen > 100, "generator must exercise launches");
}
