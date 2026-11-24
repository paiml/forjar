//! Parse-time validation of a `cloud_instance` declaration.
//!
//! A field holding a closed template (`{{ .. }}`) is unknown until resolve
//! time, so it stands in only for itself: a valid stand-in takes its place and
//! every literal field is still checked. An unclosed `{{` is a literal. Tags are
//! never resolved, so a tag with braces is rejected.

use super::*;

/// True for a value holding a `{{` that a later `}}` closes.
fn is_template(v: &Option<String>) -> bool {
    v.as_deref()
        .and_then(|v| v.split_once("{{"))
        .is_some_and(|(_, rest)| rest.contains("}}"))
}

/// Replace each templated field with a value that passes `spec_of`.
fn with_stand_ins(resource: &Resource) -> Resource {
    let mut r = resource.clone();
    if is_template(&r.name) {
        r.name = Some("x".into());
    }
    if is_template(&r.image) {
        r.image = Some(format!("x@sha256:{}", "0".repeat(64)));
    }
    if is_template(&r.cloud.region) {
        // A literal zone is region plus one letter: keep that check alive.
        let from_zone = r
            .cloud
            .availability_zone
            .as_deref()
            .or_else(|| {
                r.cloud
                    .availability_zones
                    .as_deref()
                    .and_then(|zs| zs.first())
                    .map(String::as_str)
            })
            .and_then(|z| {
                z.strip_suffix(|c: char| c.is_ascii_lowercase())
                    .filter(|head| !head.is_empty())
            });
        r.cloud.region = Some(from_zone.unwrap_or("us-east-1").into());
    }
    if is_template(&r.cloud.instance_type) {
        r.cloud.instance_type = Some("m5.large".into());
    }
    if let Some(types) = &mut r.cloud.instance_types {
        for (i, t) in types.iter_mut().enumerate() {
            if t.split_once("{{")
                .is_some_and(|(_, rest)| rest.contains("}}"))
            {
                *t = if i == 0 {
                    "m5.large".into()
                } else {
                    format!("c{i}.large")
                };
            }
        }
    }
    if is_template(&r.cloud.max_price) {
        r.cloud.max_price = Some("1".into());
    }
    r
}

/// Validate a `cloud_instance` resource: `spec_of` must accept the declaration
/// with each templated field replaced by a stand-in.
pub(super) fn validate_cloud_instance(
    id: &str,
    resource: &Resource,
    errors: &mut Vec<ValidationError>,
) {
    if let Err(e) = crate::resources::cloud_instance::spec_of(&with_stand_ins(resource)) {
        errors.push(ValidationError {
            message: format!("resource '{id}' (cloud_instance): {e}"),
        });
    }
}
