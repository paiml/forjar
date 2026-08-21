//! FJ-038: parse-time validation for `nas_archive`.
//!
//! Everything here is a refusal the operator sees at `forjar validate` time,
//! before any machinery is installed and long before anything is deleted. That
//! ordering is the point: the predecessor script's policy lived in a string
//! literal that reached `--execute` within one timer cadence of being edited.

use super::*;

pub(super) fn validate_nas_archive(
    id: &str,
    resource: &Resource,
    errors: &mut Vec<ValidationError>,
) {
    if let Err(e) = crate::resources::nas_archive::archive_of(resource) {
        errors.push(ValidationError {
            message: format!("resource '{id}' (nas_archive): {e}"),
        });
    }
}
