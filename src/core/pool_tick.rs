//! Pool reconcile tick (#894): pure composition of JIT validation,
//! spot interruption / 24h fallback, orphan sweep, and per-label dry-run
//! planning into one tick decision. No I/O, no clock read. Any sub-error
//! fails closed with zero actions.

use super::{
    pool_interruption::{
        capacity_mode, decide_rerun, on_notice, CapacityMode, InterruptionError, RerunDecision,
    },
    pool_jit::{validate as validate_jit, JitConfigError, JitRunnerConfig, JitRunnerRequest},
    pool_orphans::{cleanup_orphan_runners, sweep, Host, RunnerListing, SweepError},
    pool_plan::{plan, Action, Caps, PlanError, QueueEntry, Seat, SeatState, Snapshot},
    pool_reconcile::{HARD_TTL_SECS, SCALE_DOWN_QUIET_SECS},
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Observed per-host state at one reconcile tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostState {
    pub host: Host,
    pub labels: String,
    pub seat_states: Vec<SeatState>,
    /// `Some(secs_left)` when cloud IMDS 2-min spot interruption notice is active.
    pub spot_notice_secs_left: Option<u64>,
}

/// A job interrupted by spot preemption, with prior rerun count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterruptedJob {
    pub job_id: String,
    pub prior_reruns: u32,
}

/// Pool sizing and retirement guards for one reconcile tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickGuards {
    pub max_hosts: u32,
    pub max_seats_per_labels: u32,
    pub max_launch_per_tick: u32,
    pub hard_ttl_secs: u64,
    pub idle_scale_down_secs: u64,
}

impl Default for TickGuards {
    fn default() -> Self {
        Self {
            max_hosts: 4,
            max_seats_per_labels: 4,
            max_launch_per_tick: 4,
            hard_ttl_secs: HARD_TTL_SECS,
            idle_scale_down_secs: SCALE_DOWN_QUIET_SECS,
        }
    }
}

/// Complete pure input for one reconcile tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickInput<'a> {
    pub now_secs: u64,
    pub queue: &'a [QueueEntry],
    pub hosts: &'a [HostState],
    pub runners: &'a RunnerListing,
    pub launches_24h: &'a [u64],
    pub interruptions_24h: &'a [u64],
    pub interrupted_jobs: &'a [InterruptedJob],
    pub jit: &'a JitRunnerRequest,
    pub guards: TickGuards,
}

/// Host launch directive for one label set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchOrder {
    pub labels: String,
    pub count: u32,
    pub mode: CapacityMode,
}

/// Aggregated pure output for one reconcile tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickDecision {
    pub capacity_mode: CapacityMode,
    pub jit_config: JitRunnerConfig,
    pub launch_hosts: Vec<LaunchOrder>,
    /// Interrupted or TTL-expired hosts with busy seats: stop accepting, deregister.
    pub drain_hosts: Vec<String>,
    /// Hosts to terminate immediately (quiet+empty queue, idle TTL, or orphan).
    pub terminate_hosts: Vec<String>,
    /// Idle runners on unlisted hosts + runners on interrupted/draining hosts.
    pub deregister_runners: Vec<String>,
    pub rerun_jobs: Vec<(String, RerunDecision)>,
}

/// Typed fail-closed error from any sub-module in the reconcile tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TickError {
    Jit(JitConfigError),
    Interruption(InterruptionError),
    Sweep(SweepError),
    Plan(PlanError),
    InvalidHostSeats(String),
}

impl fmt::Display for TickError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Jit(e) => write!(f, "jit config error: {e}"),
            Self::Interruption(e) => write!(f, "interruption error: {e:?}"),
            Self::Sweep(e) => write!(f, "orphan sweep error: {e:?}"),
            Self::Plan(e) => write!(f, "plan error: {e}"),
            Self::InvalidHostSeats(id) => write!(f, "host '{id}' has no seats"),
        }
    }
}

impl std::error::Error for TickError {}

fn push_unique(items: &mut Vec<String>, value: String) {
    if !items.iter().any(|existing| existing == &value) {
        items.push(value);
    }
}

fn collect_host_runners(listing: &RunnerListing, host_id: &str, out: &mut Vec<String>) {
    if let RunnerListing::Complete(runners) = listing {
        for runner in runners {
            if runner.host_id.eq_ignore_ascii_case(host_id) {
                push_unique(out, runner.name.clone());
            }
        }
    }
}

fn build_plan_snapshot(
    input: &TickInput<'_>,
    interrupted: &BTreeSet<&str>,
) -> (Snapshot, Caps, BTreeMap<String, usize>) {
    let mut seats = Vec::new();
    let mut seat_to_host = BTreeMap::new();
    for (host_idx, h) in input.hosts.iter().enumerate() {
        let is_interrupted = interrupted.contains(h.host.id.as_str());
        for (seat_idx, &seat_state) in h.seat_states.iter().enumerate() {
            let seat_id = format!("{}#{seat_idx}", h.host.id);
            seat_to_host.insert(seat_id.clone(), host_idx);
            // Interrupted hosts stop accepting jobs, so they must not count as idle capacity.
            let effective_state = if is_interrupted {
                SeatState::Busy
            } else {
                seat_state
            };
            seats.push(Seat {
                id: seat_id,
                labels: h.labels.clone(),
                state: effective_state,
                age_secs: h.host.age_secs,
                idle_secs: h.host.idle_secs,
            });
        }
    }
    let total_seats = u32::try_from(seats.len()).unwrap_or(u32::MAX);
    let live_hosts = u32::try_from(input.hosts.len()).unwrap_or(u32::MAX);
    let host_room = input.guards.max_hosts.saturating_sub(live_hosts);
    let caps = Caps {
        max_seats: total_seats.saturating_add(host_room),
        max_seats_per_labels: input.guards.max_seats_per_labels,
        max_launch_per_tick: input.guards.max_launch_per_tick,
        max_age_secs: input.guards.hard_ttl_secs,
        idle_retire_secs: input.guards.idle_scale_down_secs,
    };
    let snapshot = Snapshot {
        queue: input.queue.to_vec(),
        seats,
    };
    (snapshot, caps, seat_to_host)
}

fn aggregate_plan_actions(
    actions: &[Action],
    seat_to_host: &BTreeMap<String, usize>,
    host_count: usize,
    mode: CapacityMode,
) -> (Vec<LaunchOrder>, Vec<usize>, Vec<bool>) {
    let mut launch_hosts = Vec::new();
    let mut retired_counts = vec![0usize; host_count];
    let mut draining_flags = vec![false; host_count];
    for action in actions {
        match action {
            Action::Launch { labels, count, .. } => {
                launch_hosts.push(LaunchOrder {
                    labels: labels.clone(),
                    count: *count,
                    mode,
                });
            }
            Action::Retire { seat, .. } => {
                if let Some(&idx) = seat_to_host.get(seat.as_str()) {
                    if let Some(count) = retired_counts.get_mut(idx) {
                        *count += 1;
                    }
                }
            }
            Action::Drain { seat, .. } => {
                if let Some(&idx) = seat_to_host.get(seat.as_str()) {
                    if let Some(flag) = draining_flags.get_mut(idx) {
                        *flag = true;
                    }
                }
            }
            Action::Nothing { .. } => {}
        }
    }
    (launch_hosts, retired_counts, draining_flags)
}

fn host_has_busy_seat(hosts: &[HostState], host_id: &str) -> bool {
    hosts
        .iter()
        .find(|h| h.host.id == host_id)
        .is_some_and(|h| h.seat_states.contains(&SeatState::Busy))
}

/// Compose one full pool reconcile tick across JIT validation, 24h fallback
/// Hosts under a spot notice: stop accepting, deregister their runners.
fn collect_notices<'a>(input: &'a TickInput<'_>) -> (BTreeSet<&'a str>, Vec<String>, Vec<String>) {
    let mut interrupted = BTreeSet::new();
    let mut drain = Vec::new();
    let mut dereg = Vec::new();
    for h in input.hosts {
        let Some(secs_left) = h.spot_notice_secs_left else {
            continue;
        };
        let notice = on_notice(secs_left);
        if notice.stop_accepting {
            interrupted.insert(h.host.id.as_str());
            push_unique(&mut drain, h.host.id.clone());
        }
        if notice.deregister {
            collect_host_runners(input.runners, &h.host.id, &mut dereg);
        }
    }
    (interrupted, drain, dereg)
}

/// Turn the plan's per-host counts into drain/terminate lists.
fn classify_hosts(
    input: &TickInput<'_>,
    interrupted: &BTreeSet<&str>,
    plan_flags: (&[usize], &[bool]),
    out: (&mut Vec<String>, &mut Vec<String>),
) -> Vec<String> {
    let (retired_counts, draining_flags) = plan_flags;
    let (drain_hosts, deregister_runners) = out;
    let mut terminate = Vec::new();
    for (idx, h) in input.hosts.iter().enumerate() {
        if interrupted.contains(h.host.id.as_str()) {
            continue;
        }
        if draining_flags.get(idx).copied().unwrap_or(false) {
            push_unique(drain_hosts, h.host.id.clone());
            collect_host_runners(input.runners, &h.host.id, deregister_runners);
        } else if retired_counts.get(idx).copied().unwrap_or(0) == h.seat_states.len() {
            push_unique(&mut terminate, h.host.id.clone());
        }
    }
    terminate
}

/// capacity mode, spot interruption notices, orphan sweep, and per-label plan.
pub fn reconcile_tick(input: &TickInput<'_>) -> Result<TickDecision, TickError> {
    let jit_config = validate_jit(input.jit).map_err(TickError::Jit)?;
    let mode = capacity_mode(input.launches_24h, input.interruptions_24h, input.now_secs)
        .map_err(TickError::Interruption)?;

    let mut hosts = Vec::with_capacity(input.hosts.len());
    for h in input.hosts {
        if h.seat_states.is_empty() {
            return Err(TickError::InvalidHostSeats(h.host.id.clone()));
        }
        hosts.push(h.host.clone());
    }

    let sweep_res = sweep(input.runners, &hosts).map_err(TickError::Sweep)?;
    let cleanup_res = cleanup_orphan_runners(input.runners, &hosts).map_err(TickError::Sweep)?;

    let (interrupted_hosts, mut drain_hosts, mut deregister_runners) = collect_notices(input);

    let rerun_jobs = input
        .interrupted_jobs
        .iter()
        .map(|job| (job.job_id.clone(), decide_rerun(job.prior_reruns)))
        .collect();

    let (snapshot, caps, seat_to_host) = build_plan_snapshot(input, &interrupted_hosts);
    let actions = plan(&snapshot, &caps).map_err(TickError::Plan)?;
    let (launch_hosts, retired_counts, draining_flags) =
        aggregate_plan_actions(&actions, &seat_to_host, input.hosts.len(), mode);

    let mut terminate_hosts = classify_hosts(
        input,
        &interrupted_hosts,
        (&retired_counts, &draining_flags),
        (&mut drain_hosts, &mut deregister_runners),
    );

    for id in sweep_res.terminate_hosts {
        if !drain_hosts.contains(&id) && !host_has_busy_seat(input.hosts, &id) {
            push_unique(&mut terminate_hosts, id);
        }
    }
    for runner in cleanup_res.deregister_runners {
        push_unique(&mut deregister_runners, runner);
    }

    Ok(TickDecision {
        capacity_mode: mode,
        jit_config,
        launch_hosts,
        drain_hosts,
        terminate_hosts,
        deregister_runners,
        rerun_jobs,
    })
}
