//! FJ-3104: Webhook event source — config, request type, and validation policy.
//!
//! Signature primitives live in [`crate::core::webhook_sig`]; framing and response
//! construction in [`crate::core::webhook_http`].
//!
//! # Fail closed
//!
//! This receiver turns an inbound HTTP request into an [`InfraEvent`] that drives
//! the rules engine, which can run scripts on machines. Two independent fail-open
//! defaults used to guard that: `secret` defaulted to `None` (skipping signature
//! checking entirely), and an EMPTY `allowed_paths` meant allow-every-path —
//! measured, `POST /anything-at-all` returned `Valid`. So the configuration an
//! operator would write to lock the endpoint down was the least restrictive one
//! available. Both now deny.

use crate::core::types::{EventType, InfraEvent};
use crate::core::webhook_sig::{
    self, canonical_payload, timestamp_is_fresh, unix_now, DEFAULT_TOLERANCE_SECS,
};
use std::collections::HashMap;

/// Header carrying forjar's own `t=…,v1=…` signature.
pub const SIG_HEADER: &str = "x-forjar-signature";
/// GitHub's signature header, accepted for interoperability.
pub const GITHUB_SIG_HEADER: &str = "x-hub-signature-256";

/// Configuration for a webhook endpoint.
#[derive(Clone)]
pub struct WebhookConfig {
    /// Address to bind. Defaults to loopback; a non-loopback bind must be paired
    /// with `tls_terminated_upstream`.
    pub bind: String,
    /// Port to listen on.
    pub port: u16,
    /// HMAC-SHA256 shared secret. `None` is only legal with
    /// `allow_unauthenticated`.
    pub secret: Option<String>,
    /// Maximum request body size in bytes.
    pub max_body_bytes: usize,
    /// Allowed request paths. EMPTY MEANS DENY ALL.
    pub allowed_paths: Vec<String>,
    /// Freshness window for `t=` in seconds.
    pub signature_tolerance_secs: u64,
    /// Operator must say so out loud to run without a secret.
    pub allow_unauthenticated: bool,
    /// Operator asserts TLS is terminated in front of a non-loopback bind.
    pub tls_terminated_upstream: bool,
    /// Machine name stamped onto produced events.
    pub machine: Option<String>,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1".to_string(),
            port: 8484,
            secret: None,
            max_body_bytes: 1024 * 64, // 64 KiB
            allowed_paths: vec!["/webhook".to_string()],
            signature_tolerance_secs: DEFAULT_TOLERANCE_SECS,
            allow_unauthenticated: false,
            tls_terminated_upstream: false,
            machine: None,
        }
    }
}

/// Hand-written so the shared secret never reaches a log or a panic message.
/// `derive(Debug)` printed it verbatim, and `dispatch_request` used to interpolate
/// `{validation:?}` straight into the HTTP response body.
impl std::fmt::Debug for WebhookConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebhookConfig")
            .field("bind", &self.bind)
            .field("port", &self.port)
            .field(
                "secret",
                &self.secret.as_ref().map(|_| "<redacted>").unwrap_or("None"),
            )
            .field("max_body_bytes", &self.max_body_bytes)
            .field("allowed_paths", &self.allowed_paths)
            .field("signature_tolerance_secs", &self.signature_tolerance_secs)
            .field("allow_unauthenticated", &self.allow_unauthenticated)
            .field("tls_terminated_upstream", &self.tls_terminated_upstream)
            .field("machine", &self.machine)
            .finish()
    }
}

impl WebhookConfig {
    /// Reject a configuration that would expose an unauthenticated or plaintext
    /// endpoint, BEFORE the listener binds.
    ///
    /// Refusing at startup rather than warning: the failure mode is an
    /// arbitrary-event-injection endpoint feeding a script-executing rules engine,
    /// and a warning in a log nobody reads is not a control.
    pub fn validate_startup(&self) -> Result<(), String> {
        if self.secret.is_none() && !self.allow_unauthenticated {
            return Err("webhook: no `secret` configured. Set one, or set \
                 `allow_unauthenticated: true` to accept unsigned requests \
                 (every accepted request can trigger rulebook actions)."
                .to_string());
        }
        if let Some(s) = &self.secret {
            if s.is_empty() {
                return Err("webhook: `secret` is empty; omit it or set a real value".to_string());
            }
        }
        let loopback = self.bind == "127.0.0.1" || self.bind == "::1" || self.bind == "localhost";
        if !loopback && !self.tls_terminated_upstream {
            return Err(format!(
                "webhook: refusing to bind {} without `tls_terminated_upstream: true`. \
                 Signatures authenticate the sender but do not encrypt the payload.",
                self.bind
            ));
        }
        if self.allowed_paths.is_empty() {
            return Err(
                "webhook: `allowed_paths` is empty, which denies every request. \
                 List the paths you intend to serve."
                    .to_string(),
            );
        }
        Ok(())
    }
}

/// A parsed incoming webhook request.
#[derive(Debug, Clone)]
pub struct WebhookRequest {
    /// HTTP method.
    pub method: String,
    /// Request path.
    pub path: String,
    /// Lowercase-keyed headers.
    pub headers: HashMap<String, String>,
    /// EXACT body octets as received.
    ///
    /// `Vec<u8>`, not `String`. The server used to hand over
    /// `String::from_utf8_lossy(..)`, so any non-UTF-8 byte became U+FFFD (3
    /// bytes) before the MAC was computed — a sender that correctly signed the
    /// wire bytes could never match, and `body.len()` measured the inflated
    /// replacement string rather than the octets received.
    pub body: Vec<u8>,
    /// Peer address, if known.
    pub source_ip: Option<String>,
}

/// Result of validating a webhook request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    /// Request is valid.
    Valid,
    /// Body exceeds the configured maximum.
    BodyTooLarge {
        /// Observed size in bytes.
        size: usize,
        /// Configured maximum.
        max: usize,
    },
    /// Path is not in the allow-list (or the list is empty).
    PathNotAllowed {
        /// The rejected path.
        path: String,
    },
    /// A secret is configured but no signature header was sent.
    SignatureMissing,
    /// Signature present but does not verify.
    SignatureInvalid,
    /// `t=` is outside the tolerance window (replay or badly-skewed clock).
    SignatureStale {
        /// Absolute skew in seconds.
        skew_secs: u64,
    },
    /// Only POST is accepted.
    MethodNotAllowed {
        /// The rejected method.
        method: String,
    },
}

impl ValidationResult {
    /// Whether the request passed validation.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Valid)
    }

    /// HTTP status for this outcome.
    ///
    /// Every failure used to collapse to 403 while the 401/405/413 arms of the
    /// status table sat unreachable.
    #[must_use]
    pub fn status(&self) -> u16 {
        match self {
            Self::Valid => 200,
            Self::BodyTooLarge { .. } => 413,
            Self::PathNotAllowed { .. } => 404,
            Self::SignatureMissing | Self::SignatureInvalid | Self::SignatureStale { .. } => 401,
            Self::MethodNotAllowed { .. } => 405,
        }
    }

    /// Stable reason code for the response body.
    ///
    /// Fixed strings: the old code put `format!("{validation:?}")` in the body,
    /// which both echoed the attacker's path back and produced invalid JSON
    /// (`{"status":"PathNotAllowed { path: "/evil" }"}`). The Debug form is still
    /// logged server-side, where operators can see it and attackers cannot.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Valid => "accepted",
            Self::BodyTooLarge { .. } => "body_too_large",
            Self::PathNotAllowed { .. } => "not_found",
            Self::SignatureMissing => "signature_missing",
            Self::SignatureInvalid => "signature_invalid",
            Self::SignatureStale { .. } => "signature_stale",
            Self::MethodNotAllowed { .. } => "method_not_allowed",
        }
    }
}

/// Validate an incoming webhook request against the configuration.
///
/// Order matters: cheap structural checks first, then the MAC, so an unauthorized
/// sender cannot make the receiver do expensive work.
#[must_use]
pub fn validate_request(config: &WebhookConfig, request: &WebhookRequest) -> ValidationResult {
    validate_request_at(config, request, unix_now())
}

/// [`validate_request`] with an injected clock, so freshness is testable without
/// sleeping or mocking time.
#[must_use]
pub fn validate_request_at(
    config: &WebhookConfig,
    request: &WebhookRequest,
    now: i64,
) -> ValidationResult {
    if !request.method.eq_ignore_ascii_case("POST") {
        return ValidationResult::MethodNotAllowed {
            method: request.method.clone(),
        };
    }

    if request.body.len() > config.max_body_bytes {
        return ValidationResult::BodyTooLarge {
            size: request.body.len(),
            max: config.max_body_bytes,
        };
    }

    // Empty allow-list denies. Compare against the path WITHOUT its query string,
    // so `/webhook?x=1` matches `/webhook` — exact matching on the raw target made
    // any query string a rejection.
    let path_only = request.path.split('?').next().unwrap_or(&request.path);
    if !config.allowed_paths.iter().any(|p| p == path_only) {
        return ValidationResult::PathNotAllowed {
            path: path_only.to_string(),
        };
    }

    match &config.secret {
        None => ValidationResult::Valid,
        Some(secret) => verify_signature(config, request, secret, now),
    }
}

/// Check forjar's own signature, falling back to GitHub's header.
fn verify_signature(
    config: &WebhookConfig,
    request: &WebhookRequest,
    secret: &str,
    now: i64,
) -> ValidationResult {
    let key = secret.as_bytes();

    if let Some(raw) = request.headers.get(SIG_HEADER) {
        let header = webhook_sig::parse_forjar_signature(raw);
        // A header with a `t` but no `v1` is malformed, not unsigned — treating it
        // as unsigned would let a sender opt out of authentication by sending
        // junk.
        if !header.has_v1() {
            return ValidationResult::SignatureInvalid;
        }
        let Some(t) = header.timestamp else {
            return ValidationResult::SignatureInvalid;
        };
        if !timestamp_is_fresh(t, now, config.signature_tolerance_secs) {
            return ValidationResult::SignatureStale {
                skew_secs: now.saturating_sub(t).unsigned_abs(),
            };
        }
        let signed = canonical_payload(t, &request.method, &request.path, &request.body);
        // Accept if ANY v1 verifies, so a secret rotation can overlap.
        if header
            .v1
            .iter()
            .any(|sig| webhook_sig::verify_hex(key, &signed, sig))
        {
            return ValidationResult::Valid;
        }
        return ValidationResult::SignatureInvalid;
    }

    // GitHub cannot be told to sign a custom canonical form, so its header is
    // verified over the bare body. That means no timestamp binding and no path
    // binding for GitHub senders — acceptable because GitHub delivers to one
    // configured URL, and the replay guard still makes a delivery single-use.
    if let Some(raw) = request.headers.get(GITHUB_SIG_HEADER) {
        return match webhook_sig::parse_github_signature(raw) {
            Some(hex) if webhook_sig::verify_hex(key, &request.body, &hex) => {
                ValidationResult::Valid
            }
            _ => ValidationResult::SignatureInvalid,
        };
    }

    ValidationResult::SignatureMissing
}

/// Parse a JSON webhook body into a flat key-value payload.
pub fn parse_json_payload(body: &[u8]) -> Result<HashMap<String, String>, String> {
    let text = std::str::from_utf8(body).map_err(|e| format!("body is not valid UTF-8: {e}"))?;
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("invalid JSON: {e}"))?;

    let mut payload = HashMap::new();
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map {
                let str_val = match val {
                    serde_json::Value::String(s) => s,
                    other => other.to_string(),
                };
                payload.insert(key, str_val);
            }
        }
        _ => return Err("webhook body must be a JSON object".to_string()),
    }

    Ok(payload)
}

/// Convert a validated webhook request into an [`InfraEvent`].
pub fn request_to_event(
    request: &WebhookRequest,
    machine: Option<&str>,
    event_id: Option<&str>,
) -> Result<InfraEvent, String> {
    let mut payload = parse_json_payload(&request.body)?;

    // Written AFTER parsing, so these always win over same-named body keys.
    payload.insert("_path".to_string(), request.path.clone());
    if let Some(ref ip) = request.source_ip {
        payload.insert("_source_ip".to_string(), ip.clone());
    }
    if let Some(id) = event_id {
        payload.insert("_event_id".to_string(), id.to_string());
    }

    Ok(InfraEvent {
        event_type: EventType::WebhookReceived,
        // The one real clock. This module used to carry its own `now_iso8601`
        // returning `format!("{}Z", secs)` — epoch seconds with a Z suffix, not
        // ISO 8601 — so webhook events wrote `1785348550Z` into the same
        // rulebook-log field where `cli::trigger` wrote `2026-07-29T…Z`.
        timestamp: crate::tripwire::eventlog::now_iso8601(),
        machine: machine.map(str::to_string),
        payload,
    })
}
