//! #1266 slice 1: parse-time validation of an `ami_bake` declaration.
//!
//! Declaration rules only, no AWS. A closed template (`{{ .. }}`) in a field is
//! unknown until resolve time, so it is accepted and skipped for shape checks.

use super::*;

fn templated(v: &str) -> bool {
    v.split_once("{{")
        .is_some_and(|(_, rest)| rest.contains("}}"))
}

/// A stock AMI id: `ami-` then 8 to 17 lowercase hex digits.
fn is_ami_id(v: &str) -> bool {
    v.strip_prefix("ami-").is_some_and(|h| {
        (8..=17).contains(&h.len())
            && h.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

/// `sha256:` then 64 lowercase hex digits.
fn is_fleet_digest(v: &str) -> bool {
    v.strip_prefix("sha256:").is_some_and(|h| {
        h.len() == 64
            && h.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

/// File names that must never be baked into an AMI.
fn is_secret_file(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase();
    name == ".env"
        || name.starts_with(".env.")
        || name.contains("credentials")
        || name.ends_with(".pem")
        || name.ends_with(".key")
        || name.starts_with("id_rsa")
        || name.starts_with("id_ed25519")
}

pub(super) fn validate_ami_bake(id: &str, resource: &Resource, errors: &mut Vec<ValidationError>) {
    let spec = &resource.ami;
    let mut err = |msg: String| {
        errors.push(ValidationError {
            message: format!("resource '{id}' (ami_bake): {msg}"),
        });
    };
    match spec.base.as_deref() {
        None => err("has no ami_base (the stock base AMI id)".into()),
        Some(b) if !templated(b) && !is_ami_id(b) => {
            err(format!("ami_base '{b}' is not an AMI id (ami- + 8-17 hex)"));
        }
        Some(_) => {}
    }
    match spec.image_file.as_deref() {
        None => err("has no ami_image_file (the fleet image to load)".into()),
        Some(f) if !templated(f) && is_secret_file(f) => {
            err(format!(
                "ami_image_file '{f}' looks like a secret; no secrets are baked"
            ));
        }
        Some(_) => {}
    }
    match spec.region.as_deref() {
        None => err("has no ami_region".into()),
        Some(r) if !templated(r) && !crate::resources::s3_bucket::is_region(r) => {
            err(format!("ami_region '{r}' is not a valid AWS region"));
        }
        Some(_) => {}
    }
    match spec.fleet_digest.as_deref() {
        None => err("has no ami_fleet_digest".into()),
        Some(d) if !templated(d) && !is_fleet_digest(d) => {
            err(format!("ami_fleet_digest '{d}' is not sha256: + 64 hex"));
        }
        Some(_) => {}
    }
    match spec.keep_last {
        None => err("has no ami_keep_last (declared old-AMI cleanup)".into()),
        Some(0) => err("ami_keep_last must be at least 1 (cleanup never removes every AMI)".into()),
        Some(_) => {}
    }
}
