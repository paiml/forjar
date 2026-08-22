//! Image reference parsing for registry push (Refs #210).
//!
//! `build --push` used to derive its push target with a two-line heuristic
//! (`name.unwrap_or("app")`, split at the FIRST `/`), which disagreed with the
//! reference the build itself had just stamped into the image and mis-parsed
//! every un-prefixed Docker Hub name (`myorg/app` became registry `myorg`).
//! Parsing lives here, pure and testable, so the push targets exactly what was
//! built.

/// A parsed, push-ready image reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageRef {
    /// Registry hostname as declared (e.g. `ghcr.io`, `docker.io`).
    pub registry: String,
    /// Repository path (`library/` normalized for single-name Docker Hub refs).
    pub repository: String,
    /// Tag (never empty; defaults to `latest`).
    pub tag: String,
}

/// Registry assumed when the reference carries no hostname.
const DEFAULT_REGISTRY: &str = "docker.io";
/// Tag assumed when the reference carries none.
const DEFAULT_TAG: &str = "latest";
/// Docker Hub's canonical name; NOT the OCI Distribution endpoint.
const DOCKER_HUB_ALIASES: [&str; 2] = ["docker.io", "index.docker.io"];
/// The host that actually speaks OCI Distribution v1.1 for Docker Hub.
const DOCKER_HUB_API: &str = "registry-1.docker.io";

impl ImageRef {
    /// Host to speak OCI Distribution v1.1 to.
    ///
    /// `docker.io` is a website: it answers a `POST /v2/.../blobs/uploads/`
    /// with a 301 to the marketing site, whose `Location` header the old push
    /// happily mistook for an upload session. The API endpoint is
    /// `registry-1.docker.io`.
    pub fn api_host(&self) -> &str {
        if DOCKER_HUB_ALIASES.contains(&self.registry.as_str()) {
            DOCKER_HUB_API
        } else {
            &self.registry
        }
    }

    /// Render back to a canonical `registry/repository:tag` reference.
    pub fn to_reference(&self) -> String {
        format!("{}/{}:{}", self.registry, self.repository, self.tag)
    }
}

/// True when a reference's first path component names a registry host.
///
/// Docker's own rule: a component is a host iff it contains `.` or `:`, or is
/// exactly `localhost`. Anything else is the first segment of the repository
/// (`myorg/app` is Docker Hub's `myorg/app`, not host `myorg`).
fn is_registry_host(component: &str) -> bool {
    component == "localhost" || component.contains('.') || component.contains(':')
}

/// Split a reference into its name part and its tag.
fn split_tag(reference: &str) -> Result<(&str, &str), String> {
    let last_slash = reference.rfind('/').map_or(0, |i| i + 1);
    match reference[last_slash..].rfind(':') {
        Some(rel) => {
            let idx = last_slash + rel;
            let (name, tag) = (&reference[..idx], &reference[idx + 1..]);
            if tag.is_empty() {
                return Err(format!("image reference '{reference}' has an empty tag"));
            }
            Ok((name, tag))
        }
        None => Ok((reference, DEFAULT_TAG)),
    }
}

/// Reject a reference that names something no push can honestly target.
fn reject_unpushable(reference: &str) -> Result<(), String> {
    if reference.is_empty() {
        return Err("image reference is empty".to_string());
    }
    if reference.contains(char::is_whitespace) {
        return Err(format!("image reference '{reference}' contains whitespace"));
    }
    if reference.contains("://") {
        return Err(format!(
            "image reference '{reference}' is a URL; expected registry/repository:tag"
        ));
    }
    if reference.contains('@') {
        return Err(format!(
            "image reference '{reference}' is digest-pinned; \
             a digest reference names content that already exists and cannot be pushed to"
        ));
    }
    Ok(())
}

/// Parse an image reference into the registry, repository and tag to push to.
///
/// Rejects (rather than guesses at) references it cannot push honestly:
/// digest-pinned references, empty names, whitespace, and uppercase repository
/// paths that every registry rejects with a 400.
///
/// # Examples
///
/// ```
/// use forjar::core::store::image_ref::parse_image_ref;
///
/// let r = parse_image_ref("ghcr.io/foo/bar:1.2.3").unwrap();
/// assert_eq!(r.registry, "ghcr.io");
/// assert_eq!(r.repository, "foo/bar");
/// assert_eq!(r.tag, "1.2.3");
///
/// // No hostname: Docker Hub, with the implicit `library/` namespace.
/// let r = parse_image_ref("nginx").unwrap();
/// assert_eq!(r.to_reference(), "docker.io/library/nginx:latest");
/// assert_eq!(r.api_host(), "registry-1.docker.io");
/// ```
pub fn parse_image_ref(reference: &str) -> Result<ImageRef, String> {
    let reference = reference.trim();
    reject_unpushable(reference)?;

    let (name, tag) = split_tag(reference)?;
    let (registry, repository) = match name.split_once('/') {
        Some((head, rest)) if is_registry_host(head) => (head.to_string(), rest.to_string()),
        _ if name.contains('/') => (DEFAULT_REGISTRY.to_string(), name.to_string()),
        _ => (DEFAULT_REGISTRY.to_string(), format!("library/{name}")),
    };

    if repository.is_empty() || repository.starts_with('/') || repository.ends_with('/') {
        return Err(format!(
            "image reference '{reference}' has an empty repository path"
        ));
    }
    if registry.is_empty() {
        return Err(format!(
            "image reference '{reference}' has an empty registry"
        ));
    }
    if repository.chars().any(|c| c.is_ascii_uppercase()) {
        return Err(format!(
            "image reference '{reference}' has an uppercase repository path; \
             registries require lowercase repository names"
        ));
    }

    Ok(ImageRef {
        registry,
        repository,
        tag: tag.to_string(),
    })
}
