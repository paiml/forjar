//! Parse-time validation of a `cloud_instance` declaration.

use super::*;

/// Validate a `cloud_instance` resource: a field holding a template is
/// deferred, otherwise `spec_of` must accept the declaration.
pub(super) fn validate_cloud_instance(
    id: &str,
    resource: &Resource,
    errors: &mut Vec<ValidationError>,
) {
    let templated = [
        &resource.name,
        &resource.image,
        &resource.cloud.region,
        &resource.cloud.instance_type,
        &resource.cloud.max_price,
    ]
    .iter()
    .any(|f| f.as_deref().is_some_and(|v| v.contains("{{")))
        || resource.tags.iter().any(|t| t.contains("{{"));
    if let (false, Err(e)) = (
        templated,
        crate::resources::cloud_instance::spec_of(resource),
    ) {
        errors.push(ValidationError {
            message: format!("resource '{id}' (cloud_instance): {e}"),
        });
    }
}
