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
fn retired_seat_frees_room_for_launch() {
    let c = Caps { max_seats: 1, ..C };
    let s = Snapshot {
        queue: vec![q("b", 1, 1)],
        seats: vec![seat("o", "a", SeatState::Idle, 50_000, 0)],
    };
    let p = plan(&s, &c).unwrap();
    assert_eq!(launched(&p), 1);
    assert!(matches!(&p[0], Action::Retire { .. }));
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
