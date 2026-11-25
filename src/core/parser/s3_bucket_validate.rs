//! Parse-time validation of an `s3_bucket` declaration.
//!
//! A bucket or region holding a closed template (`{{ .. }}`) is unknown until resolve
//! time, so a valid stand-in takes its place and every literal field is still checked.

use super::*;

fn templated(v: &Option<String>) -> bool {
    v.as_deref()
        .and_then(|v| v.split_once("{{"))
        .is_some_and(|(_, rest)| rest.contains("}}"))
}

pub(super) fn validate_s3_bucket(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    let mut r = resource.clone();
    if templated(&r.s3.bucket) {
        r.s3.bucket = Some("x-bucket".into());
    }
    if templated(&r.s3.region) {
        r.s3.region = Some("us-east-1".into());
    }
    if let Err(e) = crate::resources::s3_bucket::spec_of(&r) {
        errors.push(ValidationError {
            message: format!("resource '{id}' (s3_bucket): {e}"),
        });
    }
}
