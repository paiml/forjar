//! Pool dry-run plan (#894, slice 4): a pure function from a snapshot (queue per
//! label set, seats, caps) to the actions the pool would take. No I/O, no clock
//! read, nothing is executed; the caller decides what to do with the plan.

use super::pool_reconcile::SCALE_UP_AGE_SECS;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeatState {
    Idle,
    Busy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seat {
    pub id: String,
    pub labels: String,
    pub state: SeatState,
    pub age_secs: u64,
    /// Seconds since the seat last ran a job (meaningful when idle).
    pub idle_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueEntry {
    pub labels: String,
    pub queued: u32,
    pub oldest_age_secs: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Caps {
    pub max_seats: u32,
    pub max_seats_per_labels: u32,
    pub max_launch_per_tick: u32,
    pub max_age_secs: u64,
    pub idle_retire_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub queue: Vec<QueueEntry>,
    pub seats: Vec<Seat>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Launch {
        labels: String,
        count: u32,
        reason: String,
    },
    Retire {
        seat: String,
        reason: String,
    },
    /// Past max age but busy: stop accepting, retire when the job ends.
    Drain {
        seat: String,
        reason: String,
    },
    Nothing {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    DuplicateSeat(String),
    EmptyLabels,
    DuplicateQueueEntry(String),
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateSeat(id) => write!(f, "duplicate seat id: {id}"),
            Self::EmptyLabels => write!(f, "empty label set"),
            Self::DuplicateQueueEntry(l) => write!(f, "duplicate queue entry for labels: {l}"),
        }
    }
}

impl std::error::Error for PlanError {}

fn validate(s: &Snapshot) -> Result<(), PlanError> {
    let mut ids = BTreeSet::new();
    for seat in &s.seats {
        if seat.labels.is_empty() {
            return Err(PlanError::EmptyLabels);
        }
        if !ids.insert(seat.id.as_str()) {
            return Err(PlanError::DuplicateSeat(seat.id.clone()));
        }
    }
    let mut labels = BTreeSet::new();
    for q in &s.queue {
        if q.labels.is_empty() {
            return Err(PlanError::EmptyLabels);
        }
        if !labels.insert(q.labels.as_str()) {
            return Err(PlanError::DuplicateQueueEntry(q.labels.clone()));
        }
    }
    Ok(())
}

/// Decide what to do with one seat: retire, drain, or leave it.
fn seat_action(seat: &Seat, queued: u32, c: &Caps) -> Option<Action> {
    let id = seat.id.clone();
    if seat.age_secs >= c.max_age_secs {
        let reason = format!("age {}s >= max {}s", seat.age_secs, c.max_age_secs);
        return Some(match seat.state {
            SeatState::Idle => Action::Retire { seat: id, reason },
            SeatState::Busy => Action::Drain { seat: id, reason },
        });
    }
    let idle_out =
        seat.state == SeatState::Idle && queued == 0 && seat.idle_secs >= c.idle_retire_secs;
    idle_out.then(|| Action::Retire {
        seat: id,
        reason: format!("idle {}s with empty queue", seat.idle_secs),
    })
}

/// Count idle seats of one label set that are not being retired this tick.
fn count_idle_kept(s: &Snapshot, retired: &BTreeSet<&str>, labels: &str) -> u32 {
    s.seats
        .iter()
        .filter(|x| !retired.contains(x.id.as_str()))
        .filter(|x| x.labels == labels && x.state == SeatState::Idle)
        .count() as u32
}

/// Count seats that exist now (retirements do not free room until a later tick).
fn count_existing(s: &Snapshot, labels: Option<&str>) -> u32 {
    s.seats
        .iter()
        .filter(|x| labels.is_none_or(|l| x.labels == l))
        .count() as u32
}

/// Compute the plan. Order: retirements first, then launches by label set
/// (sorted), so the result is deterministic. Launches are clamped against the
/// seats that exist now: a retirement may fail, so it frees no room until a
/// later tick. Scale-up follows `pool_reconcile::decide()`: a label set scales
/// up when queued jobs exceed its idle seats or its oldest job waited over
/// `SCALE_UP_AGE_SECS`.
pub fn plan(s: &Snapshot, c: &Caps) -> Result<Vec<Action>, PlanError> {
    validate(s)?;
    let queue: BTreeMap<&str, &QueueEntry> =
        s.queue.iter().map(|q| (q.labels.as_str(), q)).collect();
    let mut actions = Vec::new();
    let mut retired = BTreeSet::new();
    for seat in &s.seats {
        let queued = queue.get(seat.labels.as_str()).map_or(0, |q| q.queued);
        if let Some(a) = seat_action(seat, queued, c) {
            if matches!(a, Action::Retire { .. }) {
                retired.insert(seat.id.as_str());
            }
            actions.push(a);
        }
    }
    let mut total_live = count_existing(s, None);
    let mut budget = c.max_launch_per_tick;
    for (labels, q) in &queue {
        let idle = count_idle_kept(s, &retired, labels);
        let due = q.queued > idle || q.oldest_age_secs > SCALE_UP_AGE_SECS;
        if q.queued == 0 || !due {
            continue;
        }
        let want = q.queued.saturating_sub(idle).max(1);
        let room_label = c
            .max_seats_per_labels
            .saturating_sub(count_existing(s, Some(labels)));
        let room_total = c.max_seats.saturating_sub(total_live);
        let count = want.min(room_label).min(room_total).min(budget);
        if count > 0 {
            total_live += count;
            budget -= count;
            actions.push(Action::Launch {
                labels: (*labels).to_string(),
                count,
                reason: format!(
                    "{} queued, {idle} idle, oldest {}s",
                    q.queued, q.oldest_age_secs
                ),
            });
        }
    }
    if actions.is_empty() {
        actions.push(Action::Nothing {
            reason: "queue served, no seat due for retirement".into(),
        });
    }
    Ok(actions)
}

#[cfg(test)]
#[path = "pool_plan_tests.rs"]
mod tests;
