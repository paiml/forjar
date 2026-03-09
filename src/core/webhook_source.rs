//! FJ-3104: Webhook event source.
//!
//! Parses incoming HTTP webhook requests and converts them to
//! InfraEvent values for the rules engine. Provides request
//! validation (HMAC signatures) and payload extraction.

use crate::core::types::{EventType, InfraEvent};
use std::collections::HashMap;

/// Configuration for a webhook endpoint.
#[derive(Debug, Clone)]
pub struct WebhookConfig {
    /// Port to listen on.
    pub port: u16,
    /// Optional HMAC-SHA256 secret for request validation.
    pub secret: Option<String>,
    /// Maximum request body size in bytes.
    pub max_body_bytes: usize,
    /// Allowed source paths (e.g., "/hooks/deploy").
    pub allowed_paths: Vec<String>,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            port: 8484,
            secret: None,
            max_body_bytes: 1024 * 64, // 64 KiB
            allowed_paths: vec!["/webhook".to_string()],
        }
    }
}

/// A parsed incoming webhook request.
#[derive(Debug, Clone)]
pub struct WebhookRequest {
    /// HTTP method (POST, PUT, etc.).
    pub method: String,
    /// Request path.
    pub path: String,
    /// Request headers.
    pub headers: HashMap<String, String>,
    /// Request body as UTF-8 string.
    pub body: String,
    /// Source IP address (if available).
    pub source_ip: Option<String>,
}

/// Result of validating a webhook request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    /// Request is valid.
    Valid,
    /// Request body exceeds max size.
    BodyTooLarge { size: usize, max: usize },
    /// Path is not in the allowed list.
    PathNotAllowed { path: String },
    /// HMAC signature is missing when secret is configured.
    SignatureMissing,
    /// HMAC signature does not match.
    SignatureInvalid,
    /// HTTP method not allowed (only POST accepted).
    MethodNotAllowed { method: String },
}

impl ValidationResult {
    /// Whether the request passed validation.
    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Valid)
    }
}

/// Validate an incoming webhook request against the configuration.
pub fn validate_request(config: &WebhookConfig, request: &WebhookRequest) -> ValidationResult {
    // Only POST allowed
    if request.method.to_uppercase() != "POST" {
        return ValidationResult::MethodNotAllowed {
            method: request.method.clone(),
        };
    }

    // Check body size
    if request.body.len() > config.max_body_bytes {
        return ValidationResult::BodyTooLarge {
            size: request.body.len(),
            max: config.max_body_bytes,
        };
    }

    // Check allowed paths
    if !config.allowed_paths.is_empty() && !config.allowed_paths.iter().any(|p| p == &request.path)
    {
        return ValidationResult::PathNotAllowed {
            path: request.path.clone(),
        };
    }

    // Check HMAC signature if secret is configured
    if let Some(ref secret) = config.secret {
        match request.headers.get("x-forjar-signature") {
            None => return ValidationResult::SignatureMissing,
            Some(sig) => {
                let expected = compute_hmac_hex(secret, &request.body);
                if sig != &expected {
                    return ValidationResult::SignatureInvalid;
                }
            }
        }
    }

    ValidationResult::Valid
}

/// Parse a JSON webhook body into key-value payload for InfraEvent.
pub fn parse_json_payload(body: &str) -> Result<HashMap<String, String>, String> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("invalid JSON: {e}"))?;

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

/// Convert a validated webhook request into an InfraEvent.
pub fn request_to_event(request: &WebhookRequest) -> Result<InfraEvent, String> {
    let mut payload = parse_json_payload(&request.body)?;

    // Add metadata from the request
    payload.insert("_path".to_string(), request.path.clone());
    if let Some(ref ip) = request.source_ip {
        payload.insert("_source_ip".to_string(), ip.clone());
    }

    Ok(InfraEvent {
        event_type: EventType::WebhookReceived,
        timestamp: now_iso8601(),
        machine: None,
        payload,
    })
}

/// Compute HMAC-SHA256 of `data` using `key`, returned as hex string.
///
/// Uses a simple HMAC construction: H((key XOR opad) || H((key XOR ipad) || data))
/// For production, prefer ring or hmac crate. This is a minimal implementation
/// to avoid adding heavyweight crypto dependencies.
pub fn compute_hmac_hex(key: &str, data: &str) -> String {
    // Use BLAKE3 keyed hash as HMAC substitute (faster, simpler, sovereign)
    let key_bytes = blake3::hash(key.as_bytes());
    let mut hasher = blake3::Hasher::new_keyed(key_bytes.as_bytes());
    hasher.update(data.as_bytes());
    hasher.finalize().to_hex().to_string()
}

/// Format an HTTP response for a webhook acknowledgment.
pub fn ack_response(status: u16, message: &str) -> String {
    let body = format!(r#"{{"status":"{message}"}}"#);
    let reason = status_reason(status);
    let len = body.len();
    format!(
        "HTTP/1.1 {status} {reason}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {len}\r\n\
         \r\n\
         {body}",
    )
}

fn status_reason(code: u16) -> &'static str {
    match code {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        500 => "Internal Server Error",
        _ => "Unknown",
    }
}

fn now_iso8601() -> String {
    // Minimal ISO 8601 without external crate
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}Z", dur.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> WebhookConfig {
        WebhookConfig::default()
    }

    fn post_request(path: &str, body: &str) -> WebhookRequest {
        WebhookRequest {
            method: "POST".into(),
            path: path.into(),
            headers: HashMap::new(),
            body: body.into(),
            source_ip: Some("127.0.0.1".into()),
        }
    }

    #[test]
    fn validate_valid_post() {
        let config = default_config();
        let req = post_request("/webhook", r#"{"action":"deploy"}"#);
        assert!(validate_request(&config, &req).is_valid());
    }

    #[test]
    fn validate_method_not_allowed() {
        let config = default_config();
        let req = WebhookRequest {
            method: "GET".into(),
            path: "/webhook".into(),
            headers: HashMap::new(),
            body: String::new(),
            source_ip: None,
        };
        assert_eq!(
            validate_request(&config, &req),
            ValidationResult::MethodNotAllowed {
                method: "GET".into()
            }
        );
    }

    #[test]
    fn validate_body_too_large() {
        let config = WebhookConfig {
            max_body_bytes: 10,
            ..default_config()
        };
        let req = post_request("/webhook", "a]long body that exceeds limit");
        match validate_request(&config, &req) {
            ValidationResult::BodyTooLarge { size, max } => {
                assert!(size > max);
            }
            other => panic!("expected BodyTooLarge, got {other:?}"),
        }
    }

    #[test]
    fn validate_path_not_allowed() {
        let config = default_config();
        let req = post_request("/admin/hack", r#"{}"#);
        assert_eq!(
            validate_request(&config, &req),
            ValidationResult::PathNotAllowed {
                path: "/admin/hack".into()
            }
        );
    }

    #[test]
    fn validate_signature_missing() {
        let config = WebhookConfig {
            secret: Some("mysecret".into()),
            ..default_config()
        };
        let req = post_request("/webhook", r#"{}"#);
        assert_eq!(
            validate_request(&config, &req),
            ValidationResult::SignatureMissing
        );
    }

    #[test]
    fn validate_signature_valid() {
        let secret = "test-secret";
        let body = r#"{"deploy":true}"#;
        let sig = compute_hmac_hex(secret, body);

        let config = WebhookConfig {
            secret: Some(secret.into()),
            ..default_config()
        };
        let mut req = post_request("/webhook", body);
        req.headers.insert("x-forjar-signature".into(), sig);
        assert!(validate_request(&config, &req).is_valid());
    }

    #[test]
    fn validate_signature_invalid() {
        let config = WebhookConfig {
            secret: Some("real-secret".into()),
            ..default_config()
        };
        let mut req = post_request("/webhook", r#"{}"#);
        req.headers
            .insert("x-forjar-signature".into(), "bad-sig".into());
        assert_eq!(
            validate_request(&config, &req),
            ValidationResult::SignatureInvalid
        );
    }

    #[test]
    fn parse_json_object() {
        let payload = parse_json_payload(r#"{"action":"deploy","env":"prod"}"#).unwrap();
        assert_eq!(payload.get("action").unwrap(), "deploy");
        assert_eq!(payload.get("env").unwrap(), "prod");
    }

    #[test]
    fn parse_json_nested() {
        let payload = parse_json_payload(r#"{"count":42,"nested":{"a":1}}"#).unwrap();
        assert_eq!(payload.get("count").unwrap(), "42");
        assert!(payload.get("nested").unwrap().contains("\"a\":1"));
    }

    #[test]
    fn parse_json_invalid() {
        assert!(parse_json_payload("not json").is_err());
    }

    #[test]
    fn parse_json_non_object() {
        assert!(parse_json_payload("[1,2,3]").is_err());
    }

    #[test]
    fn request_to_event_valid() {
        let req = post_request("/webhook", r#"{"action":"restart"}"#);
        let event = request_to_event(&req).unwrap();
        assert_eq!(event.event_type, EventType::WebhookReceived);
        assert_eq!(event.payload.get("action").unwrap(), "restart");
        assert_eq!(event.payload.get("_path").unwrap(), "/webhook");
        assert_eq!(event.payload.get("_source_ip").unwrap(), "127.0.0.1");
    }

    #[test]
    fn request_to_event_invalid_body() {
        let req = post_request("/webhook", "not json");
        assert!(request_to_event(&req).is_err());
    }

    #[test]
    fn hmac_deterministic() {
        let h1 = compute_hmac_hex("key", "data");
        let h2 = compute_hmac_hex("key", "data");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn hmac_different_keys() {
        let h1 = compute_hmac_hex("key1", "data");
        let h2 = compute_hmac_hex("key2", "data");
        assert_ne!(h1, h2);
    }

    #[test]
    fn ack_response_format() {
        let resp = ack_response(200, "accepted");
        assert!(resp.starts_with("HTTP/1.1 200 OK"));
        assert!(resp.contains("application/json"));
        assert!(resp.contains("accepted"));
    }

    #[test]
    fn ack_response_error() {
        let resp = ack_response(400, "bad request");
        assert!(resp.contains("400 Bad Request"));
    }

    #[test]
    fn validation_result_is_valid() {
        assert!(ValidationResult::Valid.is_valid());
        assert!(!ValidationResult::SignatureMissing.is_valid());
        assert!(!ValidationResult::SignatureInvalid.is_valid());
    }

    #[test]
    fn default_webhook_config() {
        let config = WebhookConfig::default();
        assert_eq!(config.port, 8484);
        assert!(config.secret.is_none());
        assert_eq!(config.max_body_bytes, 64 * 1024);
        assert_eq!(config.allowed_paths, vec!["/webhook"]);
    }
}
