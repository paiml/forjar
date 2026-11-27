//! #1266 (SPOT-CAPACITY T2, slice 2): the `ami_bake` spot builder, as a plan.
//!
//! `plan_bake` turns one declared `AmiBakeSpec` into the ordered steps that
//! bake a fleet AMI on a spot builder: launch from the stock base, bootstrap
//! Ubuntu + Docker + runner + warm caches, load the fleet image file, stop,
//! and register the image tagged with the fleet digest. Old-AMI cleanup is
//! planned from `keep_last`.
//!
//! This slice builds the plan and runs it through a `BakeBackend` trait. No
//! backend talks to AWS here: the only implementation is the test fake. The
//! image load, the register-image call and the cleanup execution stay under
//! #1266 for a later slice.

use crate::core::types::AmiBakeSpec;

/// One step of a bake, in execution order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BakeStep {
    /// Launch one spot builder from the stock base AMI.
    LaunchSpot { base: String, region: String },
    /// Bootstrap the builder: Ubuntu packages, Docker, runner, warm caches.
    Bootstrap,
    /// Load the fleet image file onto the builder (a path, never inline content).
    LoadImage { file: String },
    /// Stop the builder so its disk is consistent before the image is taken.
    StopBuilder,
    /// Register the image, tagged with the fleet digest.
    RegisterImage {
        region: String,
        fleet_digest: String,
    },
    /// Terminate the spot builder once its image is registered.
    TerminateBuilder,
    /// Deregister all but the newest `keep_last` bakes.
    CleanupOld { keep_last: u32 },
}

/// Why a declaration cannot be planned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BakePlanError {
    MissingField(&'static str),
    BadBaseAmi(String),
    BadRegion(String),
    BadDigest(String),
    SecretImageFile(String),
    BadImageFile(String),
    KeepLastZero,
}

impl std::fmt::Display for BakePlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingField(k) => write!(f, "ami_bake: {k} is required"),
            Self::BadBaseAmi(v) => write!(f, "ami_bake: ami_base '{v}' is not an AMI id"),
            Self::BadRegion(v) => write!(f, "ami_bake: ami_region '{v}' has no region shape"),
            Self::BadDigest(v) => {
                write!(f, "ami_bake: ami_fleet_digest '{v}' is not sha256:<64 hex>")
            }
            Self::SecretImageFile(v) => {
                write!(f, "ami_bake: ami_image_file '{v}' looks like a secret")
            }
            Self::BadImageFile(v) => {
                write!(f, "ami_bake: ami_image_file '{v}' is not a plain path")
            }
            Self::KeepLastZero => write!(f, "ami_bake: ami_keep_last must be at least 1"),
        }
    }
}

/// The side effects a bake needs. Implemented for real in a later slice.
pub trait BakeBackend {
    fn launch_spot(&mut self, base: &str, region: &str) -> Result<(), String>;
    fn bootstrap(&mut self) -> Result<(), String>;
    fn load_image(&mut self, file: &str) -> Result<(), String>;
    fn stop_builder(&mut self) -> Result<(), String>;
    fn register_image(&mut self, region: &str, fleet_digest: &str) -> Result<(), String>;
    fn cleanup_old(&mut self, keep_last: u32) -> Result<(), String>;
    /// Tear down the builder when a bake fails after launch, so no spot instance is leaked.
    fn terminate_builder(&mut self) -> Result<(), String>;
}

fn is_ami_id(v: &str) -> bool {
    // AWS AMI ids are "ami-" + 8 hex (legacy) or 17 hex.
    v.strip_prefix("ami-").is_some_and(|h| {
        matches!(h.len(), 8 | 17)
            && h.bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    })
}

fn is_region(v: &str) -> bool {
    // e.g. us-east-1, eu-west-2, ap-southeast-1: lowercase groups, final digit.
    let parts: Vec<&str> = v.split('-').collect();
    parts.len() >= 3
        && parts[..parts.len() - 1]
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_lowercase()))
        && parts
            .last()
            .is_some_and(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

fn is_fleet_digest(v: &str) -> bool {
    v.strip_prefix("sha256:").is_some_and(|h| {
        h.len() == 64
            && h.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

/// A file path, not inline content: no control characters, no padding, and
/// not a directory or a degenerate name ("/", "images/", ".", "..").
fn is_bad_image_path(path: &str) -> bool {
    if path.trim() != path || path.chars().any(|c| c.is_control()) {
        return true;
    }
    if path.ends_with('/') || path.ends_with('\\') {
        return true;
    }
    let base = path.rsplit(['/', '\\']).next().unwrap_or(path);
    matches!(base, "" | "." | "..")
}

fn is_secret_file(path: &str) -> bool {
    // Windows separators and trailing slashes must not hide the basename.
    let norm = path.replace('\\', "/");
    let norm = norm.trim_end_matches('/');
    let name = norm.rsplit('/').next().unwrap_or(norm).to_ascii_lowercase();
    name == ".env"
        || name.starts_with(".env.")
        || name.contains("credentials")
        || name.ends_with(".pem")
        || name.ends_with(".key")
        || name.starts_with("id_rsa")
        || name.starts_with("id_ed25519")
}

/// Plan one bake. Refuses a declaration missing a required field or carrying
/// a value the builder must never accept, even when the parser was bypassed.
pub fn plan_bake(spec: &AmiBakeSpec) -> Result<Vec<BakeStep>, BakePlanError> {
    let base = spec
        .base
        .as_deref()
        .ok_or(BakePlanError::MissingField("ami_base"))?;
    let image_file = spec
        .image_file
        .as_deref()
        .ok_or(BakePlanError::MissingField("ami_image_file"))?;
    let region = spec
        .region
        .as_deref()
        .ok_or(BakePlanError::MissingField("ami_region"))?;
    let fleet_digest = spec
        .fleet_digest
        .as_deref()
        .ok_or(BakePlanError::MissingField("ami_fleet_digest"))?;
    let keep_last = spec
        .keep_last
        .ok_or(BakePlanError::MissingField("ami_keep_last"))?;

    if !is_ami_id(base) {
        return Err(BakePlanError::BadBaseAmi(base.to_string()));
    }
    if image_file.trim().is_empty() {
        return Err(BakePlanError::MissingField("ami_image_file"));
    }
    if is_bad_image_path(image_file) {
        return Err(BakePlanError::BadImageFile(
            image_file.escape_debug().to_string(),
        ));
    }
    if is_secret_file(image_file) {
        return Err(BakePlanError::SecretImageFile(image_file.to_string()));
    }
    if !is_region(region) {
        return Err(BakePlanError::BadRegion(region.to_string()));
    }
    if !is_fleet_digest(fleet_digest) {
        return Err(BakePlanError::BadDigest(fleet_digest.to_string()));
    }
    if keep_last == 0 {
        return Err(BakePlanError::KeepLastZero);
    }

    Ok(vec![
        BakeStep::LaunchSpot {
            base: base.to_string(),
            region: region.to_string(),
        },
        BakeStep::Bootstrap,
        BakeStep::LoadImage {
            file: image_file.to_string(),
        },
        BakeStep::StopBuilder,
        BakeStep::RegisterImage {
            region: region.to_string(),
            fleet_digest: fleet_digest.to_string(),
        },
        BakeStep::TerminateBuilder,
        BakeStep::CleanupOld { keep_last },
    ])
}

/// Run a plan in order. Stops at the first failing step. If the builder was
/// launched and not yet terminated, a failure terminates it before returning,
/// so a spot instance is never leaked and never terminated twice.
pub fn run_bake(steps: &[BakeStep], backend: &mut dyn BakeBackend) -> Result<(), (usize, String)> {
    let mut launched = false;
    let mut terminated = false;
    for (i, step) in steps.iter().enumerate() {
        let r = match step {
            BakeStep::LaunchSpot { base, region } => backend.launch_spot(base, region),
            BakeStep::Bootstrap => backend.bootstrap(),
            BakeStep::LoadImage { file } => backend.load_image(file),
            BakeStep::StopBuilder => backend.stop_builder(),
            BakeStep::RegisterImage {
                region,
                fleet_digest,
            } => backend.register_image(region, fleet_digest),
            BakeStep::TerminateBuilder => backend.terminate_builder(),
            BakeStep::CleanupOld { keep_last } => backend.cleanup_old(*keep_last),
        };
        match r {
            Ok(()) => match step {
                BakeStep::LaunchSpot { .. } => launched = true,
                BakeStep::TerminateBuilder => terminated = true,
                _ => {}
            },
            Err(e) => {
                // A failed termination step already tried once: no second attempt.
                let own_terminate = matches!(step, BakeStep::TerminateBuilder);
                if launched && !terminated && !own_terminate {
                    backend
                        .terminate_builder()
                        .map_err(|t| (i, format!("{e}; terminate failed: {t}")))?;
                }
                return Err((i, e));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
