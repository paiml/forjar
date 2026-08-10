//! HTTP status handling and post-push verification for registry push (Refs #210).
//!
//! The push used to gate on "did a `Location:` header appear?" and to treat a
//! missing one as "registry unreachable — skipped", exit 0. Both halves were
//! wrong: `docker.io` answers the upload POST with a **301 to the marketing
//! site**, whose `Location` is not an upload session, and an unreachable
//! registry is a failed push, not a skipped one. Everything here exists so a
//! push is judged by the registry's own answer.

use super::registry_push::RegistryPushConfig;

/// Connect timeout (seconds) for the short control-plane requests.
///
/// Only the CONNECT phase is bounded — never the transfer — so a slow upload
/// is unaffected while an unroutable registry fails fast instead of hanging.
pub(crate) const CONNECT_TIMEOUT_SECS: &str = "15";

/// Scheme to address a registry with.
///
/// Loopback registries (a `registry:2` on `localhost`, the usual way to test a
/// push without handing credentials to anyone) serve plain HTTP; everything
/// else is HTTPS. This mirrors the "localhost is an insecure registry by
/// default" rule Docker and containerd already use.
pub(crate) fn registry_scheme(registry: &str) -> &'static str {
    let host = registry.split(':').next().unwrap_or(registry);
    if matches!(host, "localhost" | "127.0.0.1" | "::1") {
        "http"
    } else {
        "https"
    }
}

/// Build a registry URL for `path` (no leading slash).
pub(crate) fn registry_url(registry: &str, path: &str) -> String {
    format!("{}://{registry}/{path}", registry_scheme(registry))
}

/// Append `digest=` to an upload-session URL.
///
/// Refs #210: registries hand back a session URL that already carries a query
/// string (`?_state=…` for distribution/registry:2, ECR, GHCR). Concatenating
/// a second `?digest=` produced an unparseable URL and the registry answered
/// BLOB_UPLOAD_INVALID — i.e. the monolithic upload could never complete
/// against a real registry.
pub(crate) fn with_digest_query(upload_url: &str, digest: &str) -> String {
    let separator = if upload_url.contains('?') { '&' } else { '?' };
    format!("{upload_url}{separator}digest={digest}")
}

/// Parse the response status code out of a raw header dump (`curl -D -`).
///
/// Returns the last non-informational status line, so a `100 Continue`
/// preamble does not mask the real answer.
pub(crate) fn parse_status_code(headers: &str) -> Option<u16> {
    let mut last = None;
    for line in headers.lines() {
        if !line.starts_with("HTTP/") {
            continue;
        }
        let code = line
            .split_whitespace()
            .nth(1)
            .and_then(|c| c.parse::<u16>().ok());
        if let Some(code) = code {
            if (100..200).contains(&code) && last.is_some() {
                continue;
            }
            last = Some(code);
        }
    }
    last
}

/// Resolve a `Location` header against the registry host.
///
/// Registries may answer with an absolute URL or an absolute path.
pub(crate) fn resolve_location(registry: &str, location: &str) -> String {
    if location.starts_with("http://") || location.starts_with("https://") {
        location.to_string()
    } else {
        registry_url(registry, location.trim_start_matches('/'))
    }
}

/// Turn a non-success HTTP status into an error a human can act on.
///
/// 401/403 is called out by name: forjar implements no registry
/// authentication, so an authenticated registry is a refusal, never a skip.
pub(crate) fn describe_status(action: &str, code: u16) -> String {
    match code {
        401 | 403 => format!(
            "{action} rejected with HTTP {code}: this registry requires authentication, \
             and forjar does not implement registry credentials \
             (no Bearer-token exchange, no ~/.docker/config.json). \
             Use `forjar build --load` followed by `docker push`, or `--far`, \
             or push to a registry that accepts anonymous writes"
        ),
        300..=399 => format!(
            "{action} answered with HTTP {code} (a redirect), not an OCI upload session — \
             check the registry hostname; `docker.io` is a website, \
             the Docker Hub API endpoint is `registry-1.docker.io`"
        ),
        404 => format!(
            "{action} rejected with HTTP 404: the repository does not exist \
             or this registry does not speak OCI Distribution v1.1 at /v2/"
        ),
        _ => format!("{action} rejected with HTTP {code}"),
    }
}

/// Post-push verification: does the tag actually resolve at the registry now?
///
/// This is the independent read-back that licenses the "Push complete" line.
/// It asks the registry — not forjar's own intent — whether the manifest is
/// there, and (when the registry echoes `Docker-Content-Digest`) whether it is
/// the manifest we just pushed.
pub fn verify_manifest_pushed(
    config: &RegistryPushConfig,
    expected_digest: &str,
) -> Result<(), String> {
    let url = registry_url(
        &config.registry,
        &format!("v2/{}/manifests/{}", config.name, config.tag),
    );
    let output = std::process::Command::new("curl")
        .args([
            "-s",
            "-D",
            "-",
            "-o",
            "/dev/null",
            "--connect-timeout",
            CONNECT_TIMEOUT_SECS,
            "--head",
            "-H",
            "Accept: application/vnd.oci.image.manifest.v1+json, \
             application/vnd.docker.distribution.manifest.v2+json",
            &url,
        ])
        .output()
        .map_err(|e| format!("push verification (HEAD manifest): {e}"))?;

    let headers = String::from_utf8_lossy(&output.stdout);
    let code = parse_status_code(&headers).ok_or_else(|| {
        format!(
            "push verification failed: no HTTP response from {url} \
             (the upload reported success but the tag cannot be read back)"
        )
    })?;
    if code != 200 {
        return Err(format!(
            "push verification failed: {}",
            describe_status(&format!("HEAD {url}"), code)
        ));
    }

    if let Some(served) = header_value(&headers, "docker-content-digest") {
        if served != expected_digest {
            return Err(format!(
                "push verification failed: {url} resolves to {served}, \
                 not the manifest just pushed ({expected_digest})"
            ));
        }
    }
    Ok(())
}

/// Case-insensitive lookup of a single header value in a raw header dump.
pub(crate) fn header_value(headers: &str, name: &str) -> Option<String> {
    let want = format!("{}:", name.to_lowercase());
    headers
        .lines()
        .find(|l| l.to_lowercase().starts_with(&want))
        .map(|l| l[want.len()..].trim().to_string())
}

/// Compose curl failure detail. With `--fail-with-body` curl prints the HTTP
/// response body to stdout on a >= 400 status; stderr carries transport-level
/// diagnostics. Surface both so the registry error (401 token, 413, …) shows.
pub(crate) fn curl_error_detail(output: &std::process::Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let body = stdout.trim();
    let err = stderr.trim();
    match (body.is_empty(), err.is_empty()) {
        (false, false) => format!("{err}: {body}"),
        (false, true) => body.to_string(),
        (true, false) => err.to_string(),
        (true, true) => "no response body".to_string(),
    }
}
