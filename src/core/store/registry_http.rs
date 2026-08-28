//! GH-228: the HTTP transport the OCI Distribution v1.1 push runs on.
//!
//! Every registry verb used to be a `curl` subprocess whose raw `-D -` header
//! dump was re-parsed back into a status code and whose exit code stood in for
//! "did the registry accept this?". That made an external binary an undeclared
//! runtime dependency of `forjar build --push` (GH-224), and it put the two
//! load-bearing correctness rules of this module one remove from the code,
//! expressed as argv strings rather than as checks on a response.
//!
//! This module is the only place `ureq` is named. The two rules now live in
//! [`agent_for_url`] as configuration, and every caller gates on
//! [`RegistryResponse::status`]:
//!
//! * `http_status_as_error(false)` — a 4xx must arrive as a response with its
//!   body intact, not as an `Err` with the registry's diagnostic discarded.
//!   That diagnostic is what `--fail-with-body` was added for in #154.
//! * `max_redirects(0)` — **the #210 gate**. `docker.io` answers the upload
//!   POST with a 301 to its marketing site. ureq follows up to 10 redirects by
//!   default, so leaving that on would hand `initiate_upload` a 200 from a web
//!   page and reopen exactly the hole #210 closed.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::time::Duration;
use ureq::http::request::Builder;
use ureq::http::Response;
use ureq::tls::{RootCerts, TlsConfig};
use ureq::{Agent, AsSendBody, Body, Proxy, SendBody};

/// Connect timeout for registry requests.
///
/// Only the CONNECT phase is bounded — never the transfer — so a slow 64 MB
/// upload is unaffected while an unroutable registry fails fast instead of
/// hanging. This is why no global timeout is configured.
pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

/// How much of a failing registry's response body is quoted back in an error.
const MAX_ERROR_BODY: u64 = 8 * 1024;

/// What a registry answered, reduced to the four facts the push protocol needs.
pub(crate) struct RegistryResponse {
    /// The HTTP status. Every caller gates on this and nothing else.
    pub status: u16,
    /// `Location`, the upload-session URL on a 202.
    pub location: Option<String>,
    /// `Docker-Content-Digest`, used to prove a tag resolves to what we pushed.
    pub docker_content_digest: Option<String>,
    /// The response body, read only for a non-2xx and capped at 8 KiB — this
    /// is the registry's own error text (`BLOB_UPLOAD_INVALID`, a token hint),
    /// which is the thing `--fail-with-body` existed to surface.
    pub body: String,
}

/// What to send as the request body.
pub(crate) enum RequestBody<'a> {
    /// No body (HEAD, and the POST that opens an upload session).
    Empty,
    /// An in-memory body — the manifest JSON.
    Bytes(&'a [u8]),
    /// A whole file, streamed. `Content-Length` comes from its metadata, so a
    /// 64 MB blob is never buffered.
    File(&'a Path),
    /// A byte range of a file, streamed — one chunk of a chunked PATCH upload.
    /// The caller must set `Content-Length: len`; without it ureq falls back to
    /// chunked transfer-encoding, which some registries reject on PATCH.
    FileRange {
        /// File to read the chunk from.
        path: &'a Path,
        /// Offset of the chunk within the file.
        offset: u64,
        /// Length of the chunk in bytes.
        len: u64,
    },
}

/// Build the agent for requests aimed at `url`.
///
/// Proxy handling is the one place this differs per URL. A loopback registry
/// (the usual way to test a push without handing credentials to anyone) must
/// never be proxied: `curl` honored an ambient `HTTP(S)_PROXY` even for
/// `127.0.0.1`, with no way to say otherwise short of mutating process-global
/// env, which is why all four in-process-registry tests had to be `#[ignore]`d.
/// Configuring the proxy per agent removes that, so those tests can run.
pub(crate) fn agent_for_url(url: &str) -> Agent {
    let loopback = super::registry_push_http::registry_scheme(authority(url)) == "http";
    let proxy = if loopback {
        None
    } else {
        Proxy::try_from_env()
    };
    Agent::config_builder()
        .http_status_as_error(false)
        .max_redirects(0)
        .timeout_global(None)
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .proxy(proxy)
        .tls_config(
            TlsConfig::builder()
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .build()
        .new_agent()
}

/// The `host[:port]` of an absolute URL, or the whole string if it is not one.
pub(crate) fn authority(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    rest.split(['/', '?', '#']).next().unwrap_or(rest)
}

/// Perform one registry request. `Err` means the transport never produced a
/// response; any HTTP status — including 4xx and 5xx — comes back as `Ok`, so
/// the caller decides what the status means.
pub(crate) fn send(
    agent: &Agent,
    method: &str,
    url: &str,
    headers: &[(&str, String)],
    body: RequestBody<'_>,
) -> Result<RegistryResponse, String> {
    let mut builder = ureq::http::Request::builder().method(method).uri(url);
    for (name, value) in headers {
        builder = builder.header(*name, value);
    }
    Ok(collect(dispatch(agent, builder, body)?))
}

/// One arm per body shape; the type of the body differs in each, which is why
/// this is a match over four `run` calls rather than one.
fn dispatch(
    agent: &Agent,
    builder: Builder,
    body: RequestBody<'_>,
) -> Result<Response<Body>, String> {
    match body {
        RequestBody::Empty => run(agent, builder, ()),
        RequestBody::Bytes(bytes) => run(agent, builder, bytes),
        RequestBody::File(path) => run(agent, builder, open(path)?),
        RequestBody::FileRange { path, offset, len } => {
            let mut file = open(path)?;
            file.seek(SeekFrom::Start(offset))
                .map_err(|e| format!("seek {} to {offset}: {e}", path.display()))?;
            run(agent, builder, SendBody::from_owned_reader(file.take(len)))
        }
    }
}

/// Open a blob for upload, naming the file in the error.
fn open(path: &Path) -> Result<File, String> {
    File::open(path).map_err(|e| format!("open {}: {e}", path.display()))
}

fn run<B: AsSendBody>(agent: &Agent, builder: Builder, body: B) -> Result<Response<Body>, String> {
    let request = builder
        .body(body)
        .map_err(|e| format!("malformed request: {e}"))?;
    agent.run(request).map_err(|e| e.to_string())
}

/// Reduce a live response to [`RegistryResponse`], draining the body only when
/// the status says something went wrong.
fn collect(mut response: Response<Body>) -> RegistryResponse {
    let status = response.status().as_u16();
    let location = header(&response, "location");
    let docker_content_digest = header(&response, "docker-content-digest");
    let body = if is_success(status) {
        String::new()
    } else {
        read_error_body(&mut response)
    };
    RegistryResponse {
        status,
        location,
        docker_content_digest,
        body,
    }
}

/// Did the registry accept the request?
pub(crate) fn is_success(status: u16) -> bool {
    (200..300).contains(&status)
}

fn header(response: &Response<Body>, name: &str) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
}

/// Read at most [`MAX_ERROR_BODY`] bytes of the body. Truncates rather than
/// failing: a body that is too long, or not UTF-8, must not cost us the
/// diagnostic entirely.
fn read_error_body(response: &mut Response<Body>) -> String {
    let mut buf = Vec::new();
    let _ = response
        .body_mut()
        .as_reader()
        .take(MAX_ERROR_BODY)
        .read_to_end(&mut buf);
    String::from_utf8_lossy(&buf).trim().to_string()
}

/// The registry's own words about a failure, for an error message.
pub(crate) fn detail(response: &RegistryResponse) -> String {
    if response.body.is_empty() {
        "no response body".to_string()
    } else {
        response.body.clone()
    }
}
