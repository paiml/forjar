//! #1266 (SPOT-CAPACITY T2, slice 2): the `ami_bake` spot builder, as a plan.
//!
//! `plan_bake` turns one declared `AmiBakeSpec` into the ordered steps that
//! bake a fleet AMI on a spot builder: launch from the stock base, bootstrap
//! Ubuntu + Docker + runner + warm caches, load the fleet image file, screen
//! for secrets, stop, and register the image tagged with the fleet digest.
//! Old-AMI cleanup is
//! planned from `keep_last`.
//!
//! The plan runs through the `BakeBackend` trait, the only seam to AWS. The
//! guards live here, not in a backend: the builder is screened for secrets
//! before it is registered, the image is registered with `register_tags`, and
//! `cleanup_victims` alone picks what is deregistered (#1310). No backend in
//! this module talks to AWS: the only implementations are the test fakes.

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
    /// Refuse the bake if a secret-looking file is on the builder's disk.
    VerifyNoSecrets,
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
    CleanupOld { region: String, keep_last: u32 },
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

/// A failure from one backend step, typed so it crosses the trait boundary
/// without a bare `String` in the signature. The text is for the operator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BakeFailure(pub String);

impl std::fmt::Display for BakeFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The side effects a bake needs. Implemented for real in a later slice.
pub trait BakeBackend {
    fn launch_spot(&mut self, base: &str, region: &str) -> Result<(), BakeFailure>;
    fn bootstrap(&mut self) -> Result<(), BakeFailure>;
    fn load_image(&mut self, file: &str) -> Result<(), BakeFailure>;
    /// Paths of every file under `roots` on the builder's disk.
    fn builder_files(&mut self, roots: &[&str]) -> Result<Vec<String>, BakeFailure>;
    fn stop_builder(&mut self) -> Result<(), BakeFailure>;
    /// Register the image carrying `tags`; returns the new AMI id.
    fn register_image(
        &mut self,
        region: &str,
        tags: &[(&'static str, String)],
    ) -> Result<String, BakeFailure>;
    /// The account's own images in `region`, with their forjar tags.
    fn list_images(&mut self, region: &str) -> Result<Vec<BakedImage>, BakeFailure>;
    fn deregister_image(&mut self, region: &str, ami_id: &str) -> Result<(), BakeFailure>;
    /// Tear down the builder when a bake fails after launch, so no spot instance is leaked.
    fn terminate_builder(&mut self) -> Result<(), BakeFailure>;
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
    // Windows separators, trailing slashes and trailing whitespace must not hide the basename.
    let norm = path.trim().replace('\\', "/");
    let norm = norm.trim_end_matches('/');
    let name = norm.rsplit('/').next().unwrap_or(norm).to_ascii_lowercase();
    name == ".env"
        || name.starts_with(".env.")
        || name.contains("credentials")
        || name.ends_with(".pem")
        || name.ends_with(".key")
        || name.starts_with("id_rsa")
        || name.starts_with("id_ed25519")
        || name.starts_with("id_ecdsa")
        || name.starts_with("id_dsa")
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
        BakeStep::VerifyNoSecrets,
        BakeStep::StopBuilder,
        BakeStep::RegisterImage {
            region: region.to_string(),
            fleet_digest: fleet_digest.to_string(),
        },
        BakeStep::TerminateBuilder,
        BakeStep::CleanupOld {
            region: region.to_string(),
            keep_last,
        },
    ])
}

/// Directories on the builder screened for secrets before it is registered:
/// the places a bake's user, runner and scratch files land. The system trust
/// store under `/etc` and `/usr` is left out: its `.pem` files are public.
pub const SECRET_SCAN_ROOTS: &[&str] = &["/root", "/home", "/opt", "/tmp", "/var/tmp"];

/// What one run has established so far.
#[derive(Default)]
struct BakeRun {
    /// True only while nothing has written to the builder since the last screen.
    screened: bool,
    /// The stock base the builder was launched from: never a cleanup victim.
    base: Option<String>,
    /// The region the image was registered in: the only region cleanup may touch.
    region: Option<String>,
    registered: Option<String>,
}

impl BakeRun {
    fn step(&mut self, step: &BakeStep, backend: &mut dyn BakeBackend) -> Result<(), BakeFailure> {
        match step {
            BakeStep::LaunchSpot { base, region } => {
                self.screened = false;
                backend.launch_spot(base, region)?;
                self.base = Some(base.clone());
                Ok(())
            }
            BakeStep::Bootstrap => {
                self.screened = false;
                backend.bootstrap()
            }
            BakeStep::LoadImage { file } => {
                self.screened = false;
                backend.load_image(file)
            }
            BakeStep::VerifyNoSecrets => self.verify_no_secrets(backend),
            BakeStep::StopBuilder => backend.stop_builder(),
            BakeStep::RegisterImage {
                region,
                fleet_digest,
            } => self.register(backend, region, fleet_digest),
            BakeStep::TerminateBuilder => backend.terminate_builder(),
            BakeStep::CleanupOld { region, keep_last } => self.cleanup(backend, region, *keep_last),
        }
    }

    fn verify_no_secrets(&mut self, backend: &mut dyn BakeBackend) -> Result<(), BakeFailure> {
        let files = backend.builder_files(SECRET_SCAN_ROOTS)?;
        let found: Vec<String> = files
            .iter()
            .map(String::as_str)
            .filter(|p| is_secret_file(p))
            .map(|p| p.escape_debug().to_string())
            .collect();
        if !found.is_empty() {
            return Err(BakeFailure(format!(
                "ami_bake: secrets on the builder, not registering: {}",
                found.join(", ")
            )));
        }
        self.screened = true;
        Ok(())
    }

    fn register(
        &mut self,
        backend: &mut dyn BakeBackend,
        region: &str,
        fleet_digest: &str,
    ) -> Result<(), BakeFailure> {
        if !self.screened {
            return Err(BakeFailure(
                "ami_bake: the builder was not screened for secrets".into(),
            ));
        }
        if !is_region(region) {
            return Err(BakeFailure(format!(
                "ami_bake: region '{}' has no region shape",
                region.escape_debug()
            )));
        }
        let tags = register_tags(fleet_digest).map_err(|e| BakeFailure(e.to_string()))?;
        let ami_id = backend.register_image(region, &tags)?;
        if !is_ami_id(&ami_id) {
            return Err(BakeFailure(format!(
                "ami_bake: register returned '{}', not an AMI id",
                ami_id.escape_debug()
            )));
        }
        self.region = Some(region.to_string());
        self.registered = Some(ami_id);
        Ok(())
    }

    /// Deregister the victims in the registered region, never the image this
    /// run just registered, and never the stock base it was launched from.
    fn cleanup(
        &self,
        backend: &mut dyn BakeBackend,
        region: &str,
        keep_last: u32,
    ) -> Result<(), BakeFailure> {
        let Some(registered_in) = self.region.as_deref() else {
            return Err(BakeFailure(
                "ami_bake: cleanup before any image was registered".into(),
            ));
        };
        if !is_region(region) || region != registered_in {
            return Err(BakeFailure(format!(
                "ami_bake: cleanup region '{}' is not the registered region",
                region.escape_debug()
            )));
        }
        let images = backend.list_images(region)?;
        let victims =
            cleanup_victims(&images, keep_last).map_err(|e| BakeFailure(e.to_string()))?;
        for ami_id in victims
            .iter()
            .filter(|id| self.registered.as_ref() != Some(*id) && self.base.as_ref() != Some(*id))
        {
            backend.deregister_image(region, ami_id)?;
        }
        Ok(())
    }
}

/// Run a plan in order. Stops at the first failing step. If the builder was
/// launched and not yet terminated, a failure terminates it before returning,
/// so a spot instance is never leaked and never terminated twice.
pub fn run_bake(steps: &[BakeStep], backend: &mut dyn BakeBackend) -> Result<(), (usize, String)> {
    let mut launched = false;
    let mut terminated = false;
    let mut run = BakeRun::default();
    for (i, step) in steps.iter().enumerate() {
        let r = run.step(step, backend);
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
                return Err((i, e.0));
            }
        }
    }
    Ok(())
}

/// Tag key carrying the fleet digest a bake was registered with.
pub const DIGEST_TAG: &str = "forjar:fleet-digest";
/// Tag key marking an image as a forjar bake. Cleanup only ever deregisters tagged bakes.
pub const KIND_TAG: &str = "forjar:kind";
/// Value of `KIND_TAG` on every bake.
pub const KIND_VALUE: &str = "ami_bake";

/// Tags a registered image must carry: the fleet digest and the bake kind.
/// Refuses a digest that is not `sha256:<64 hex>`, so an image is never
/// registered without a digest it can later be recognised by.
pub fn register_tags(fleet_digest: &str) -> Result<Vec<(&'static str, String)>, BakePlanError> {
    if !is_fleet_digest(fleet_digest) {
        return Err(BakePlanError::BadDigest(fleet_digest.to_string()));
    }
    Ok(vec![
        (DIGEST_TAG, fleet_digest.to_string()),
        (KIND_TAG, KIND_VALUE.to_string()),
    ])
}

/// One registered image as read back from the cloud, before any cleanup decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BakedImage {
    pub ami_id: String,
    /// Creation time, epoch seconds. Newer is larger.
    pub created: u64,
    /// The `forjar:kind` tag, if present.
    pub kind: Option<String>,
    /// The `forjar:fleet-digest` tag, if present.
    pub fleet_digest: Option<String>,
}

/// The AMI ids a cleanup may deregister: every bake except the newest `keep_last`.
/// Only images tagged as a bake with a valid fleet digest are candidates, so an
/// untagged or hand-made AMI is never touched. Newest first by `created`, ties
/// broken by `ami_id`, so the same listing always yields the same victims.
pub fn cleanup_victims(
    images: &[BakedImage],
    keep_last: u32,
) -> Result<Vec<String>, BakePlanError> {
    if keep_last == 0 {
        return Err(BakePlanError::KeepLastZero);
    }
    let mut bakes: Vec<&BakedImage> = images
        .iter()
        .filter(|i| i.kind.as_deref() == Some(KIND_VALUE))
        .filter(|i| i.fleet_digest.as_deref().is_some_and(is_fleet_digest))
        .filter(|i| is_ami_id(&i.ami_id))
        .collect();
    bakes.sort_by(|a, b| {
        b.created
            .cmp(&a.created)
            .then_with(|| a.ami_id.cmp(&b.ami_id))
    });
    // A listing may repeat an image (paginated results): count each id once,
    // so a duplicate of the kept image is never a victim.
    let mut seen = std::collections::BTreeSet::new();
    bakes.retain(|i| seen.insert(i.ami_id.as_str()));
    Ok(bakes
        .into_iter()
        .skip(keep_last as usize)
        .map(|i| i.ami_id.clone())
        .collect())
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_images;
#[cfg(test)]
mod tests_register;
