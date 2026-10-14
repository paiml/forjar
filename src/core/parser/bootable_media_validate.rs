//! Refs #713: parse-time validation for `bootable_media`.
//!
//! The declaration refusals (a `/dev/sdX` device, an unpinned ISO, a seed
//! without `user-data`) and the bashrs lint of every seed shell fragment run
//! here, so `forjar validate` and `forjar plan` refuse them before a byte is
//! written. A field still holding a `{{ template }}` cannot be judged before
//! it is rendered: that resource is judged after rendering instead, where the
//! plan line reads `REFUSED` and the apply script refuses without writing.

use super::*;

fn templated(resource: &Resource) -> bool {
    let m = &resource.media;
    [&m.device, &m.iso, &m.iso_sha256, &m.seed_state]
        .iter()
        .filter_map(|f| f.as_deref())
        .chain(m.seed.values().map(String::as_str))
        .any(|v| v.contains("{{"))
}

pub(super) fn validate_bootable_media(
    id: &str,
    resource: &Resource,
    errors: &mut Vec<ValidationError>,
) {
    if templated(resource) {
        return;
    }
    if let Err(e) = crate::resources::bootable_media::media_of(resource) {
        errors.push(ValidationError {
            message: format!("resource '{id}' (bootable_media): {e}"),
        });
    }
}
