//! forjar#1262: `ec2_spot_seat` — a spot GitHub-runner seat box managed ONLY by forjar.
//!
//! # Why
//!
//! The spot fleet was run by standalone bash (`launch-seats.sh`, `tier2-warm.sh`,
//! `pool-refill-org.sh`, the EC2 user-data). It hardcoded a stale image digest and
//! threw away the seat `docker run` output, so the pool exited 0 with no seats.
//! This resource replaces that path: one declaration, one pin file, every failure
//! a non-zero exit plus a log line.
//!
//! # YAML
//!
//! ```yaml
//! seat-a:
//!   type: ec2_spot_seat
//!   machine: controller
//!   spot_phase: launch          # launch | image-load | seat-start | register | teardown
//!   spot_pin_file: machines/fleet-image/fleet-image.tsv
//!   spot_pin_id_key: id
//!   spot_pin_s3_key_key: s3_key
//!   spot_image_bucket: paiml-fleet-image
//!   spot_instance_types: [c6a.4xlarge, m6a.4xlarge]
//!   spot_region: us-east-1
//!   spot_az: us-east-1a
//!   spot_subnet: subnet-0abc1234
//!   spot_ttl_min: 120
//!   spot_seats: 4
//!   spot_owner: forjar-spot
//!   spot_max_boxes: 2
//!   spot_max_usd_per_hour: 1.0
//!   spot_price_usd_per_hour: 0.5
//!   spot_market: spot           # on-demand is a validation error
//!   spot_runner_org: paiml
//!   spot_runner_group: tier2
//!   spot_runner_labels: [self-hosted, Linux, X64, tier2]
//!   spot_jit_source: gh-api
//! ```
//!
//! # Contract
//!
//! * Spot only; hard caps (`spot_max_boxes`, `spot_max_usd_per_hour`) fail validation
//!   when this one box alone breaks them, and the launch script re-counts live boxes
//!   by `Owner` tag before `run-instances` (exit 3 when a cap would be exceeded).
//! * The image id, tar key and tar sha256 come from the pin file only; nothing here
//!   spells a digest. `image-load` exits non-zero with a log line when the pinned
//!   image is not present after `docker load`.
//! * `seat-start` writes one log per seat and exits non-zero (with the log tail) for
//!   a seat that never reaches `running`.
//! * `teardown` ships evidence off-box first and only then cancels spot requests and
//!   terminates; a failed ship stops the teardown (exit 1) and terminates nothing.
//! * The orchestration in [`lifecycle`] runs against the [`api`] traits, so tests use
//!   fakes and never touch AWS or GitHub; [`script`] renders the same steps as shell.

pub mod api;
pub mod aws_cli;
pub mod lifecycle;
mod pin;
mod script;
mod script_ctl;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_aws_cli;
#[cfg(test)]
mod tests_lifecycle;
#[cfg(test)]
mod tests_script;

use crate::core::error::ForjarError;
use crate::core::types::Resource;
pub use pin::{parse_pin, Pin};
pub use script::{apply_script, check_script, state_query_script};

/// Phases of the resource, one per action of the issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Launch,
    ImageLoad,
    SeatStart,
    Register,
    Teardown,
}

/// A validated `ec2_spot_seat` declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct SpotSeat {
    pub phase: Phase,
    pub pin_file: String,
    pub pin_id_key: String,
    pub pin_s3_key_key: String,
    pub pin_sha256_key: String,
    pub image_bucket: Option<String>,
    pub instance_types: Vec<String>,
    pub region: String,
    pub az: String,
    pub subnet: String,
    pub ami: Option<String>,
    pub ttl_min: u32,
    pub seats: u32,
    pub owner: String,
    pub max_boxes: u32,
    pub max_usd_per_hour: f64,
    pub price_usd_per_hour: f64,
    pub runner_org: String,
    pub runner_group: String,
    pub runner_labels: Vec<String>,
    pub jit_source: String,
    pub evidence_dest: Option<String>,
}

fn word(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

fn req<'a>(v: &'a Option<String>, name: &str) -> Result<&'a str, ForjarError> {
    v.as_deref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ForjarError::validation(format!("ec2_spot_seat requires `{name}`")))
}

fn phase_of(r: &Resource) -> Result<Phase, ForjarError> {
    let absent = r.state.as_deref() == Some("absent");
    match (r.spot.phase.as_deref(), absent) {
        (None, true) | (Some("teardown"), _) => Ok(Phase::Teardown),
        (None | Some("launch"), false) => Ok(Phase::Launch),
        (Some("image-load"), false) => Ok(Phase::ImageLoad),
        (Some("seat-start"), false) => Ok(Phase::SeatStart),
        (Some("register"), false) => Ok(Phase::Register),
        (Some(p), true) => Err(ForjarError::validation(format!(
            "ec2_spot_seat: `state: absent` means teardown, not `spot_phase: {p}`"
        ))),
        (Some(p), false) => Err(ForjarError::validation(format!(
            "ec2_spot_seat: `spot_phase: {p}` is not launch, image-load, seat-start, register or teardown"
        ))),
    }
}

fn check_market(market: Option<&str>) -> Result<(), ForjarError> {
    match market {
        None | Some("spot") => Ok(()),
        Some(m) => Err(ForjarError::validation(format!(
            "ec2_spot_seat: `spot_market: {m}` is refused; only spot is allowed"
        ))),
    }
}

fn check_placement(
    region: &str,
    az: &str,
    subnet: &str,
    types: &[String],
) -> Result<(), ForjarError> {
    if !crate::resources::s3_bucket::is_region(region) {
        return Err(ForjarError::validation(format!(
            "ec2_spot_seat: `{region}` is not a valid region"
        )));
    }
    if !az.starts_with(region) || az.len() != region.len() + 1 {
        return Err(ForjarError::validation(format!(
            "ec2_spot_seat: `spot_az: {az}` is not a zone of {region}"
        )));
    }
    if !subnet.starts_with("subnet-") || !word(subnet) {
        return Err(ForjarError::validation(format!(
            "ec2_spot_seat: `spot_subnet: {subnet}` is not a subnet id"
        )));
    }
    if types.is_empty() || !types.iter().all(|t| t.contains('.') && word(t)) {
        return Err(ForjarError::validation(
            "ec2_spot_seat: `spot_instance_types` needs one or more types like c6a.4xlarge",
        ));
    }
    Ok(())
}

fn check_runner(org: &str, group: &str, labels: &[String]) -> Result<(), ForjarError> {
    if !word(org) || !word(group) {
        return Err(ForjarError::validation(
            "ec2_spot_seat: `spot_runner_org` and `spot_runner_group` must be plain words",
        ));
    }
    if labels.is_empty() || !labels.iter().all(|l| word(l)) {
        return Err(ForjarError::validation(
            "ec2_spot_seat: `spot_runner_labels` needs one or more plain labels",
        ));
    }
    Ok(())
}

fn check_jit(jit: &str) -> Result<(), ForjarError> {
    let ok = jit == "gh-api"
        || jit
            .strip_prefix("dir:")
            .is_some_and(|d| d.starts_with('/') && !d.contains(['\'', '\n', '$', '`']));
    if ok {
        Ok(())
    } else {
        Err(ForjarError::validation(
            "ec2_spot_seat: `spot_jit_source` is `gh-api` or `dir:/absolute/path`",
        ))
    }
}

/// Validate a declaration.
///
/// # Errors
///
/// Returns `Err` for a missing or malformed field, a market other than `spot`, a box
/// or price that alone breaks its own cap, or an unknown phase.
pub fn spec_of(r: &Resource) -> Result<SpotSeat, ForjarError> {
    let s = &r.spot;
    let phase = phase_of(r)?;
    check_market(s.market.as_deref())?;
    let pin_file = req(&s.pin_file, "spot_pin_file")?.to_string();
    let owner = req(&s.owner, "spot_owner")?;
    if !word(owner) {
        return Err(ForjarError::validation(format!(
            "ec2_spot_seat: `spot_owner: {owner}` is not a plain word"
        )));
    }
    let region = req(&s.region, "spot_region")?;
    let az = req(&s.az, "spot_az")?;
    let subnet = req(&s.subnet, "spot_subnet")?;
    check_placement(region, az, subnet, &s.instance_types)?;
    let ttl = s
        .ttl_min
        .ok_or_else(|| ForjarError::validation("ec2_spot_seat requires `spot_ttl_min`"))?;
    let seats = s
        .seats
        .ok_or_else(|| ForjarError::validation("ec2_spot_seat requires `spot_seats`"))?;
    if !(1..=720).contains(&ttl) || !(1..=64).contains(&seats) {
        return Err(ForjarError::validation(
            "ec2_spot_seat: `spot_ttl_min` must be 1..=720 and `spot_seats` 1..=64",
        ));
    }
    let max_boxes = s
        .max_boxes
        .ok_or_else(|| ForjarError::validation("ec2_spot_seat requires `spot_max_boxes`"))?;
    let cap = s
        .max_usd_per_hour
        .ok_or_else(|| ForjarError::validation("ec2_spot_seat requires `spot_max_usd_per_hour`"))?;
    let price = s.price_usd_per_hour.ok_or_else(|| {
        ForjarError::validation("ec2_spot_seat requires `spot_price_usd_per_hour`")
    })?;
    if max_boxes == 0 || !(cap.is_finite() && cap > 0.0) || !(price.is_finite() && price > 0.0) {
        return Err(ForjarError::validation(
            "ec2_spot_seat: caps and price must be positive",
        ));
    }
    if price > cap {
        return Err(ForjarError::validation(format!(
            "ec2_spot_seat: one box at {price} USD/h exceeds `spot_max_usd_per_hour` {cap}"
        )));
    }
    let org = req(&s.runner_org, "spot_runner_org")?;
    let group = req(&s.runner_group, "spot_runner_group")?;
    check_runner(org, group, &s.runner_labels)?;
    let jit = req(&s.jit_source, "spot_jit_source")?;
    check_jit(jit)?;
    Ok(SpotSeat {
        phase,
        pin_file,
        pin_id_key: s.pin_id_key.clone().unwrap_or_else(|| "id".into()),
        pin_s3_key_key: s.pin_s3_key_key.clone().unwrap_or_else(|| "s3_key".into()),
        pin_sha256_key: s
            .pin_sha256_key
            .clone()
            .unwrap_or_else(|| "tar_sha256".into()),
        image_bucket: s.image_bucket.clone(),
        instance_types: s.instance_types.clone(),
        region: region.into(),
        az: az.into(),
        subnet: subnet.into(),
        ami: s.ami.clone(),
        ttl_min: ttl,
        seats,
        owner: owner.into(),
        max_boxes,
        max_usd_per_hour: cap,
        price_usd_per_hour: price,
        runner_org: org.into(),
        runner_group: group.into(),
        runner_labels: s.runner_labels.clone(),
        jit_source: jit.into(),
        evidence_dest: s.evidence_dest.clone(),
    })
}
