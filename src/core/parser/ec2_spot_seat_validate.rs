//! Parse-time validation of an `ec2_spot_seat` declaration (market, caps, fields).

use super::*;

pub(super) fn validate_ec2_spot_seat(
    id: &str,
    resource: &Resource,
    errors: &mut Vec<ValidationError>,
) {
    if let Err(e) = crate::resources::ec2_spot_seat::spec_of(resource) {
        errors.push(ValidationError {
            message: format!("resource '{id}' (ec2_spot_seat): {e}"),
        });
    }
}
