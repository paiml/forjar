//! Instance key and ownership key for `cloud_instance`.

use super::{ami, profile, CloudInstance};

/// Idempotency key (`name@region@digest12`): the resource's own name, its region and the first 12 hex
/// digits of the image digest, then the `cloud_ami`, placement and storage tails when declared. Two declarations
/// get the same key only if they share a name (refused by [`validate_unique_names`]).
pub(super) fn key_of(s: &CloudInstance) -> String {
    let digest = s.image.rsplit_once("@sha256:").map_or("", |(_, d)| d);
    let head = format!("{}@{}@{}", s.name, s.region, &digest[..12]);
    head + &ami::key_tail(&s.ami)
        + &s.placement.key_suffix()
        + &s.storage.key_suffix()
        + &profile::key_tail(&s.profile)
}

/// Ownership key for teardown: name plus region, stable across image changes.
/// `@` joins the parts: neither the name nor the region allow-list can contain
/// it, so two different (name, region) pairs never give the same string.
pub(super) fn owner_of(s: &CloudInstance) -> String {
    format!("{}@{}", s.name, s.region)
}
