//! FJ-3104/3105/3108: Event source & rules engine falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-3108: Rulebook YAML validation (names, events, actions, cooldowns)
//! - FJ-3108: ValidationSummary error/warning counting
//! - FJ-3108: Event type coverage analysis
//! - FJ-3105: Metric threshold evaluation (gt, gte, lt, lte, eq)
//! - FJ-3105: Consecutive violation tracking
//! - FJ-3105: Multi-metric evaluation
//! - FJ-3104: Webhook request validation (method, body, path, HMAC)
//! - FJ-3104: JSON payload parsing
//! - FJ-3104: Request-to-event conversion
//!
//! Usage: cargo test --test falsification_event_sources

use forjar::core::metric_source::{
    evaluate_metrics, evaluate_threshold, MetricThreshold, ThresholdOp, ThresholdTracker,
};
use forjar::core::rules_engine::{
    event_type_coverage, validate_rulebook_yaml, IssueSeverity,
    ValidationSummary,
};
use forjar::core::types::{EventType, RulebookConfig};
use forjar::core::webhook_source::{
    ack_response, compute_hmac_hex, parse_json_payload, request_to_event, validate_request,
    ValidationResult, WebhookConfig, WebhookRequest,
};
use std::collections::HashMap;

// ============================================================================
// FJ-3108: Rulebook YAML Validation
// ============================================================================

fn valid_rulebook_yaml() -> &'static str {
    r#"
rulebooks:
  - name: config-repair
    events:
      - type: file_changed
        match:
          path: /etc/nginx/nginx.conf
    actions:
      - apply:
          file: forjar.yaml
          tags: [config]
    cooldown_secs: 60
"#
}

#[test]
fn rulebook_valid_yaml_no_issues() {
    let issues = validate_rulebook_yaml(valid_rulebook_yaml()).unwrap();
    assert!(issues.is_empty(), "unexpected: {issues:?}");
}

#[test]
fn rulebook_parse_error() {
    assert!(validate_rulebook_yaml("not: valid: [yaml").is_err());
}

#[test]
fn rulebook_no_events_error() {
    let yaml = r#"
rulebooks:
  - name: bad
    events: []
    actions:
      - script: "echo ok"
"#;
    let issues = validate_rulebook_yaml(yaml).unwrap();
    assert!(issues.iter().any(|i| i.message.contains("no event")));
    assert!(issues.iter().any(|i| i.severity == IssueSeverity::Error));
}

#[test]
fn rulebook_no_actions_error() {
    let yaml = r#"
rulebooks:
  - name: bad
    events:
      - type: manual
    actions: []
"#;
    let issues = validate_rulebook_yaml(yaml).unwrap();
    assert!(issues.iter().any(|i| i.message.contains("no actions")));
}

#[test]
fn rulebook_duplicate_names_error() {
    let yaml = r#"
rulebooks:
  - name: dupe
    events: [{type: manual}]
    actions: [{script: "echo 1"}]
  - name: dupe
    events: [{type: manual}]
    actions: [{script: "echo 2"}]
"#;
    let issues = validate_rulebook_yaml(yaml).unwrap();
    assert!(issues.iter().any(|i| i.message.contains("duplicate")));
}

#[test]
fn rulebook_empty_apply_file_error() {
    let yaml = r#"
rulebooks:
  - name: bad-apply
    events: [{type: manual}]
    actions:
      - apply:
          file: ""
"#;
    let issues = validate_rulebook_yaml(yaml).unwrap();
    assert!(issues
        .iter()
        .any(|i| i.message.contains("apply.file is empty")));
}

#[test]
fn rulebook_zero_cooldown_warning() {
    let yaml = r#"
rulebooks:
  - name: rapid
    events: [{type: manual}]
    actions: [{script: "echo ok"}]
    cooldown_secs: 0
"#;
    let issues = validate_rulebook_yaml(yaml).unwrap();
    assert!(issues.iter().any(|i| {
        i.severity == IssueSeverity::Warning && i.message.contains("cooldown_secs=0")
    }));
}

#[test]
fn rulebook_high_retries_warning() {
    let yaml = r#"
rulebooks:
  - name: retry
    events: [{type: manual}]
    actions: [{script: "echo ok"}]
    max_retries: 50
"#;
    let issues = validate_rulebook_yaml(yaml).unwrap();
    assert!(issues.iter().any(|i| i.message.contains("unusually high")));
}

#[test]
fn rulebook_empty_notify_channel_error() {
    let yaml = r#"
rulebooks:
  - name: bad-notify
    events: [{type: manual}]
    actions:
      - notify:
          channel: ""
          message: "test"
"#;
    let issues = validate_rulebook_yaml(yaml).unwrap();
    assert!(issues
        .iter()
        .any(|i| i.message.contains("notify.channel is empty")));
}

#[test]
fn rulebook_secret_leak_in_script_error() {
    let yaml = r#"
rulebooks:
  - name: leaky
    events: [{type: manual}]
    actions: [{script: "echo $PASSWORD"}]
"#;
    let issues = validate_rulebook_yaml(yaml).unwrap();
    assert!(
        issues.iter().any(|i| i.message.contains("secret leak")),
        "issues: {issues:?}"
    );
}

// ============================================================================
// FJ-3108: IssueSeverity
// ============================================================================

#[test]
fn issue_severity_display() {
    assert_eq!(IssueSeverity::Error.to_string(), "error");
    assert_eq!(IssueSeverity::Warning.to_string(), "warning");
}

#[test]
fn issue_severity_eq() {
    assert_eq!(IssueSeverity::Error, IssueSeverity::Error);
    assert_ne!(IssueSeverity::Error, IssueSeverity::Warning);
}

// ============================================================================
// FJ-3108: ValidationSummary
// ============================================================================

#[test]
fn validation_summary_counts() {
    let issues = vec![
        forjar::core::rules_engine::RuleIssue {
            rulebook: "a".into(),
            severity: IssueSeverity::Error,
            message: "err".into(),
        },
        forjar::core::rules_engine::RuleIssue {
            rulebook: "b".into(),
            severity: IssueSeverity::Warning,
            message: "warn".into(),
        },
    ];
    let summary = ValidationSummary::new(2, issues);
    assert_eq!(summary.error_count(), 1);
    assert_eq!(summary.warning_count(), 1);
    assert!(!summary.passed());
}

#[test]
fn validation_summary_passed() {
    let summary = ValidationSummary::new(1, vec![]);
    assert!(summary.passed());
    assert_eq!(summary.error_count(), 0);
    assert_eq!(summary.warning_count(), 0);
}

// ============================================================================
// FJ-3108: Event Type Coverage
// ============================================================================

#[test]
fn event_type_coverage_counts() {
    let yaml = r#"
rulebooks:
  - name: r1
    events:
      - {type: file_changed}
      - {type: manual}
    actions: [{script: "echo 1"}]
  - name: r2
    events:
      - {type: file_changed}
    actions: [{script: "echo 2"}]
"#;
    let config: RulebookConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let coverage = event_type_coverage(&config);
    let fc = coverage
        .iter()
        .find(|(et, _)| *et == EventType::FileChanged);
    assert_eq!(fc.unwrap().1, 2);
    let m = coverage.iter().find(|(et, _)| *et == EventType::Manual);
    assert_eq!(m.unwrap().1, 1);
    let cr = coverage.iter().find(|(et, _)| *et == EventType::CronFired);
    assert_eq!(cr.unwrap().1, 0);
}

#[test]
fn event_type_coverage_all_six_types() {
    let config = RulebookConfig { rulebooks: vec![] };
    let coverage = event_type_coverage(&config);
    assert_eq!(coverage.len(), 6);
}

// ============================================================================
// FJ-3105: Metric Threshold Evaluation
// ============================================================================

fn threshold(name: &str, op: ThresholdOp, value: f64) -> MetricThreshold {
    MetricThreshold {
        name: name.into(),
        operator: op,
        value,
        consecutive: 1,
    }
}

#[test]
fn threshold_gt() {
    let t = threshold("cpu", ThresholdOp::Gt, 80.0);
    assert!(evaluate_threshold(&t, 81.0));
    assert!(!evaluate_threshold(&t, 80.0));
    assert!(!evaluate_threshold(&t, 79.0));
}

#[test]
fn threshold_gte() {
    let t = threshold("cpu", ThresholdOp::Gte, 80.0);
    assert!(evaluate_threshold(&t, 80.0));
    assert!(evaluate_threshold(&t, 81.0));
    assert!(!evaluate_threshold(&t, 79.0));
}

#[test]
fn threshold_lt() {
    let t = threshold("disk", ThresholdOp::Lt, 10.0);
    assert!(evaluate_threshold(&t, 5.0));
    assert!(!evaluate_threshold(&t, 10.0));
    assert!(!evaluate_threshold(&t, 15.0));
}

#[test]
fn threshold_lte() {
    let t = threshold("disk", ThresholdOp::Lte, 10.0);
    assert!(evaluate_threshold(&t, 10.0));
    assert!(evaluate_threshold(&t, 5.0));
    assert!(!evaluate_threshold(&t, 15.0));
}

#[test]
fn threshold_eq() {
    let t = threshold("replicas", ThresholdOp::Eq, 3.0);
    assert!(evaluate_threshold(&t, 3.0));
    assert!(!evaluate_threshold(&t, 4.0));
}

#[test]
fn threshold_op_display() {
    assert_eq!(ThresholdOp::Gt.to_string(), ">");
    assert_eq!(ThresholdOp::Gte.to_string(), ">=");
    assert_eq!(ThresholdOp::Lt.to_string(), "<");
    assert_eq!(ThresholdOp::Lte.to_string(), "<=");
    assert_eq!(ThresholdOp::Eq.to_string(), "==");
}

#[test]
fn threshold_serde_roundtrip() {
    let t = MetricThreshold {
        name: "cpu_percent".into(),
        operator: ThresholdOp::Gt,
        value: 80.0,
        consecutive: 3,
    };
    let json = serde_json::to_string(&t).unwrap();
    let parsed: MetricThreshold = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.name, "cpu_percent");
    assert_eq!(parsed.operator, ThresholdOp::Gt);
    assert_eq!(parsed.value, 80.0);
    assert_eq!(parsed.consecutive, 3);
}

// ============================================================================
// FJ-3105: Threshold Tracker
// ============================================================================

#[test]
fn tracker_single_violation_fires() {
    let mut tracker = ThresholdTracker::default();
    assert!(tracker.record("cpu", true, 1));
}

#[test]
fn tracker_consecutive_violations() {
    let mut tracker = ThresholdTracker::default();
    assert!(!tracker.record("cpu", true, 3));
    assert!(!tracker.record("cpu", true, 3));
    assert!(tracker.record("cpu", true, 3));
}

#[test]
fn tracker_reset_on_ok() {
    let mut tracker = ThresholdTracker::default();
    tracker.record("cpu", true, 3);
    tracker.record("cpu", true, 3);
    tracker.record("cpu", false, 3); // resets
    assert_eq!(tracker.count("cpu"), 0);
    assert!(!tracker.record("cpu", true, 3)); // starts over
}

#[test]
fn tracker_count_increments() {
    let mut tracker = ThresholdTracker::default();
    assert_eq!(tracker.count("cpu"), 0);
    tracker.record("cpu", true, 5);
    assert_eq!(tracker.count("cpu"), 1);
    tracker.record("cpu", true, 5);
    assert_eq!(tracker.count("cpu"), 2);
}

#[test]
fn tracker_reset_all() {
    let mut tracker = ThresholdTracker::default();
    tracker.record("cpu", true, 5);
    tracker.record("mem", true, 5);
    tracker.reset();
    assert_eq!(tracker.count("cpu"), 0);
    assert_eq!(tracker.count("mem"), 0);
}

#[test]
fn tracker_independent_metrics() {
    let mut tracker = ThresholdTracker::default();
    tracker.record("cpu", true, 2);
    tracker.record("mem", true, 2);
    assert_eq!(tracker.count("cpu"), 1);
    assert_eq!(tracker.count("mem"), 1);
}

// ============================================================================
// FJ-3105: Multi-Metric Evaluation
// ============================================================================

#[test]
fn evaluate_multiple_metrics_mixed() {
    let thresholds = vec![
        threshold("cpu", ThresholdOp::Gt, 80.0),
        threshold("mem", ThresholdOp::Gt, 90.0),
        threshold("disk", ThresholdOp::Lt, 10.0),
    ];
    let mut values = HashMap::new();
    values.insert("cpu".into(), 85.0); // violated
    values.insert("mem".into(), 70.0); // ok
    values.insert("disk".into(), 5.0); // violated

    let mut tracker = ThresholdTracker::default();
    let results = evaluate_metrics(&thresholds, &values, &mut tracker);
    assert_eq!(results.len(), 3);
    assert!(results[0].violated);
    assert!(!results[1].violated);
    assert!(results[2].violated);
}

#[test]
fn evaluate_missing_metric_skipped() {
    let thresholds = vec![threshold("missing", ThresholdOp::Gt, 50.0)];
    let values = HashMap::new();
    let mut tracker = ThresholdTracker::default();
    let results = evaluate_metrics(&thresholds, &values, &mut tracker);
    assert!(results.is_empty());
}

#[test]
fn evaluate_should_fire_consecutive() {
    let mut t = threshold("cpu", ThresholdOp::Gt, 80.0);
    t.consecutive = 3;
    let thresholds = vec![t];

    let mut values = HashMap::new();
    values.insert("cpu".into(), 85.0);

    let mut tracker = ThresholdTracker::default();
    let r1 = evaluate_metrics(&thresholds, &values, &mut tracker);
    assert!(r1[0].violated);
    assert!(!r1[0].should_fire); // 1/3

    let r2 = evaluate_metrics(&thresholds, &values, &mut tracker);
    assert!(!r2[0].should_fire); // 2/3

    let r3 = evaluate_metrics(&thresholds, &values, &mut tracker);
    assert!(r3[0].should_fire); // 3/3 — fires!
}

// ============================================================================
// FJ-3104: Webhook Request Validation
// ============================================================================

fn webhook_config() -> WebhookConfig {
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
fn webhook_valid_post() {
    let config = webhook_config();
    let req = post_request("/webhook", r#"{"action":"deploy"}"#);
    assert!(validate_request(&config, &req).is_valid());
}

#[test]
fn webhook_method_not_allowed() {
    let config = webhook_config();
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
fn webhook_body_too_large() {
    let config = WebhookConfig {
        max_body_bytes: 10,
        ..webhook_config()
    };
    let req = post_request("/webhook", "a long body that exceeds limit");
    match validate_request(&config, &req) {
        ValidationResult::BodyTooLarge { size, max } => {
            assert!(size > max);
        }
        other => panic!("expected BodyTooLarge, got {other:?}"),
    }
}

#[test]
fn webhook_path_not_allowed() {
    let config = webhook_config();
    let req = post_request("/admin/hack", r#"{}"#);
    assert_eq!(
        validate_request(&config, &req),
        ValidationResult::PathNotAllowed {
            path: "/admin/hack".into()
        }
    );
}

#[test]
fn webhook_signature_missing() {
    let config = WebhookConfig {
        secret: Some("mysecret".into()),
        ..webhook_config()
    };
    let req = post_request("/webhook", r#"{}"#);
    assert_eq!(
        validate_request(&config, &req),
        ValidationResult::SignatureMissing
    );
}

#[test]
fn webhook_signature_valid() {
    let secret = "test-secret";
    let body = r#"{"deploy":true}"#;
    let sig = compute_hmac_hex(secret, body);

    let config = WebhookConfig {
        secret: Some(secret.into()),
        ..webhook_config()
    };
    let mut req = post_request("/webhook", body);
    req.headers.insert("x-forjar-signature".into(), sig);
    assert!(validate_request(&config, &req).is_valid());
}

#[test]
fn webhook_signature_invalid() {
    let config = WebhookConfig {
        secret: Some("real-secret".into()),
        ..webhook_config()
    };
    let mut req = post_request("/webhook", r#"{}"#);
    req.headers
        .insert("x-forjar-signature".into(), "bad-sig".into());
    assert_eq!(
        validate_request(&config, &req),
        ValidationResult::SignatureInvalid
    );
}

// ============================================================================
// FJ-3104: HMAC
// ============================================================================

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
fn hmac_different_data() {
    let h1 = compute_hmac_hex("key", "data1");
    let h2 = compute_hmac_hex("key", "data2");
    assert_ne!(h1, h2);
}

// ============================================================================
// FJ-3104: JSON Payload Parsing
// ============================================================================

#[test]
fn parse_json_object() {
    let payload = parse_json_payload(r#"{"action":"deploy","env":"prod"}"#).unwrap();
    assert_eq!(payload.get("action").unwrap(), "deploy");
    assert_eq!(payload.get("env").unwrap(), "prod");
}

#[test]
fn parse_json_nested_stringified() {
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
fn parse_json_empty_object() {
    let payload = parse_json_payload("{}").unwrap();
    assert!(payload.is_empty());
}

// ============================================================================
// FJ-3104: Request to Event
// ============================================================================

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
fn request_to_event_no_source_ip() {
    let req = WebhookRequest {
        method: "POST".into(),
        path: "/webhook".into(),
        headers: HashMap::new(),
        body: r#"{"action":"deploy"}"#.into(),
        source_ip: None,
    };
    let event = request_to_event(&req).unwrap();
    assert!(event.payload.get("_source_ip").is_none());
}

#[test]
fn request_to_event_invalid_body() {
    let req = post_request("/webhook", "not json");
    assert!(request_to_event(&req).is_err());
}

// ============================================================================
// FJ-3104: Ack Response
// ============================================================================

#[test]
fn ack_response_200() {
    let resp = ack_response(200, "accepted");
    assert!(resp.starts_with("HTTP/1.1 200 OK"));
    assert!(resp.contains("application/json"));
    assert!(resp.contains("accepted"));
}

#[test]
fn ack_response_400() {
    let resp = ack_response(400, "bad request");
    assert!(resp.contains("400 Bad Request"));
}

#[test]
fn ack_response_401() {
    let resp = ack_response(401, "unauthorized");
    assert!(resp.contains("401 Unauthorized"));
}

// ============================================================================
// FJ-3104: WebhookConfig Default
// ============================================================================

#[test]
fn webhook_config_default() {
    let config = WebhookConfig::default();
    assert_eq!(config.port, 8484);
    assert!(config.secret.is_none());
    assert_eq!(config.max_body_bytes, 64 * 1024);
    assert_eq!(config.allowed_paths, vec!["/webhook"]);
}

// ============================================================================
// FJ-3104: ValidationResult
// ============================================================================

#[test]
fn validation_result_is_valid() {
    assert!(ValidationResult::Valid.is_valid());
    assert!(!ValidationResult::SignatureMissing.is_valid());
    assert!(!ValidationResult::SignatureInvalid.is_valid());
}
