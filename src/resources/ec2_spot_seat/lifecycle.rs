//! Orchestration of an `ec2_spot_seat` against the [`api`](super::api) traits.

use super::api::{Ec2Api, EvidenceSink, LaunchError, LaunchRequest, RunnerApi};
use super::SpotSeat;
use crate::core::error::ForjarError;

/// Prefix an error with what was being done, keeping its declared class.
fn while_doing(what: &str) -> impl Fn(ForjarError) -> ForjarError + '_ {
    move |e| ForjarError::new(e.class(), format!("{what}: {e}"))
}

/// Result of the plan-time cap check.
#[derive(Debug, Clone, PartialEq)]
pub struct LaunchPlan {
    pub live_boxes: u32,
    pub projected_boxes: u32,
    pub projected_usd_per_hour: f64,
}

/// Count live boxes by owner tag and refuse when this one would break a cap.
///
/// # Errors
///
/// `Err` when the read fails (not measured, never "zero boxes") or a cap would be exceeded.
pub fn plan_launch(spec: &SpotSeat, api: &dyn Ec2Api) -> Result<LaunchPlan, ForjarError> {
    let live = api
        .live_boxes(&spec.owner)
        .map_err(while_doing("live-box read failed (not measured)"))?;
    let boxes = u32::try_from(live.len()).unwrap_or(u32::MAX) + 1;
    let usd = live.iter().map(|b| b.price_usd_per_hour).sum::<f64>() + spec.price_usd_per_hour;
    if boxes > spec.max_boxes {
        return Err(ForjarError::validation(format!(
            "cap: {boxes} boxes for owner {} exceeds spot_max_boxes {}",
            spec.owner, spec.max_boxes
        )));
    }
    if usd > spec.max_usd_per_hour + 1e-9 {
        return Err(ForjarError::validation(format!(
            "cap: {usd:.4} USD/h for owner {} exceeds spot_max_usd_per_hour {}",
            spec.owner, spec.max_usd_per_hour
        )));
    }
    Ok(LaunchPlan {
        live_boxes: boxes - 1,
        projected_boxes: boxes,
        projected_usd_per_hour: usd,
    })
}

/// Launch one spot box, trying each instance type in order; capacity errors move on.
///
/// # Errors
///
/// `Err` for a cap breach, a non-capacity AWS error, or every type out of capacity.
pub fn launch(spec: &SpotSeat, api: &dyn Ec2Api, token: &str) -> Result<String, ForjarError> {
    plan_launch(spec, api)?;
    let mut last = String::from("no instance types");
    for t in &spec.instance_types {
        let req = LaunchRequest {
            instance_type: t.clone(),
            az: spec.az.clone(),
            subnet: spec.subnet.clone(),
            ami: spec.ami.clone(),
            owner: spec.owner.clone(),
            ttl_min: spec.ttl_min,
            price_usd_per_hour: spec.price_usd_per_hour,
            client_token: format!("{token}-{t}"),
        };
        match api.run_spot(&req) {
            Ok(id) => return Ok(id),
            Err(LaunchError::Capacity(m)) => last = format!("{t}: {m}"),
            Err(LaunchError::Other(m)) => {
                return Err(ForjarError::other(format!("launch {t} failed: {m}")))
            }
        }
    }
    Err(ForjarError::other(format!(
        "no capacity for any type ({last})"
    )))
}

/// What a teardown did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Teardown {
    pub shipped: Vec<String>,
    pub cancelled: Vec<String>,
    pub terminated: Vec<String>,
}

/// Ship evidence off-box, cancel open spot requests, terminate, and read back 0.
///
/// # Errors
///
/// `Err` (and nothing terminated) when evidence cannot be shipped; `Err` when any
/// later step fails or boxes remain after the terminate.
pub fn teardown(
    spec: &SpotSeat,
    api: &dyn Ec2Api,
    sink: &dyn EvidenceSink,
) -> Result<Teardown, ForjarError> {
    let live = api
        .live_boxes(&spec.owner)
        .map_err(while_doing("live-box read failed (not measured)"))?;
    let dest = spec
        .evidence_dest
        .as_deref()
        .ok_or_else(|| ForjarError::validation("teardown needs `spot_evidence_dest`"))?;
    let mut out = Teardown {
        shipped: vec![],
        cancelled: vec![],
        terminated: vec![],
    };
    for b in &live {
        sink.ship(&b.id, dest).map_err(|e| {
            let id = &b.id;
            ForjarError::new(
                e.class(),
                format!("evidence ship failed for {id}: {e}; nothing terminated"),
            )
        })?;
        out.shipped.push(b.id.clone());
    }
    let reqs = api
        .open_spot_requests(&spec.owner)
        .map_err(while_doing("spot-request read failed"))?;
    if !reqs.is_empty() {
        api.cancel_spot_requests(&reqs)
            .map_err(while_doing("cancel failed"))?;
    }
    out.cancelled = reqs;
    let ids: Vec<String> = live.iter().map(|b| b.id.clone()).collect();
    if !ids.is_empty() {
        api.terminate(&ids)
            .map_err(while_doing("terminate failed"))?;
    }
    out.terminated = ids;
    let left = api
        .live_boxes(&spec.owner)
        .map_err(while_doing("read-back failed"))?;
    if !left.is_empty() {
        return Err(ForjarError::other(format!(
            "state shows {} boxes after teardown, want 0",
            left.len()
        )));
    }
    Ok(out)
}

/// Seat runner name for slot `i` (1-based).
#[must_use]
pub fn seat_name(spec: &SpotSeat, i: u32) -> String {
    format!("seat-{}-{i:02}", spec.owner)
}

/// Remove offline, idle registrations whose name starts with this owner's seat prefix.
///
/// # Errors
///
/// `Err` when the list or a delete fails.
pub fn deregister_stale(spec: &SpotSeat, gh: &dyn RunnerApi) -> Result<Vec<String>, ForjarError> {
    let prefix = format!("seat-{}-", spec.owner);
    let mut gone = vec![];
    for r in gh.list_runners(&spec.runner_org)? {
        if r.name.starts_with(&prefix) && !r.online && !r.busy {
            gh.delete_runner(&spec.runner_org, r.id)?;
            gone.push(r.name);
        }
    }
    Ok(gone)
}

/// Clear stale registrations, then mint one JIT config per missing seat.
///
/// Returns `(seat name, jit config)` pairs; a seat already online keeps its registration.
///
/// # Errors
///
/// `Err` when any GitHub call fails.
pub fn register(spec: &SpotSeat, gh: &dyn RunnerApi) -> Result<Vec<(String, String)>, ForjarError> {
    deregister_stale(spec, gh)?;
    let online: Vec<String> = gh
        .list_runners(&spec.runner_org)?
        .into_iter()
        .filter(|r| r.online)
        .map(|r| r.name)
        .collect();
    let mut out = vec![];
    for i in 1..=spec.seats {
        let n = seat_name(spec, i);
        if online.contains(&n) {
            continue;
        }
        let jit = gh.mint_jit(
            &spec.runner_org,
            &spec.runner_group,
            &n,
            &spec.runner_labels,
        )?;
        out.push((n, jit));
    }
    Ok(out)
}
