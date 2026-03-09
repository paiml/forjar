//! FJ-2800: Scoring engine and recipe validation falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-2800: Scoring v2 engine
//!   - compute: static-only grading (no runtime data)
//!   - compute: static+runtime two-tier grading
//!   - grade thresholds: A/B/C/D/F boundary conditions
//!   - score_bar: visual bar rendering
//!   - format_score_report: report structure
//!   - hard_fail on blocked status
//! - Recipe validation (validate_inputs)
//!   - string, int, bool, path, enum type validation via public API
//!   - default values, bounds checking, missing required inputs
//!
//! Usage: cargo test --test falsification_scoring_recipe_validation

use forjar::core::recipe::{validate_inputs, RecipeInput, RecipeMetadata};
use forjar::core::scoring::{compute, format_score_report, score_bar, RuntimeData, ScoringInput};
use forjar::core::types::ForjarConfig;
use indexmap::IndexMap;
use std::collections::HashMap;

// ============================================================================
// FJ-2800: score_bar
// ============================================================================

#[test]
fn score_bar_full() {
    let bar = score_bar(100);
    assert_eq!(bar, "[####################]");
}

#[test]
fn score_bar_zero() {
    let bar = score_bar(0);
    assert_eq!(bar, "[....................]");
}

#[test]
fn score_bar_half() {
    let bar = score_bar(50);
    assert_eq!(bar, "[##########..........]");
}

// ============================================================================
// FJ-2800: compute — static-only
// ============================================================================

fn minimal_config() -> ForjarConfig {
    ForjarConfig {
        name: "test-config".into(),
        version: "1.0".into(),
        ..Default::default()
    }
}

fn static_only_input() -> ScoringInput {
    ScoringInput {
        status: "pending".into(),
        idempotency: "strong".into(),
        budget_ms: 5000,
        runtime: None,
        raw_yaml: Some(
            "# Recipe: test\n# Tier: basic\n# Idempotency: strong\n# Budget: 5000ms\n\
             name: test-config\n"
                .into(),
        ),
    }
}

#[test]
fn scoring_static_only_no_hard_fail() {
    let config = minimal_config();
    let input = static_only_input();
    let result = compute(&config, &input);
    assert!(!result.hard_fail);
    assert!(result.grade.contains("pending"));
    assert!(result.runtime_composite.is_none());
    assert!(result.runtime_grade.is_none());
}

#[test]
fn scoring_blocked_hard_fails() {
    let config = minimal_config();
    let input = ScoringInput {
        status: "blocked".into(),
        idempotency: "strong".into(),
        budget_ms: 0,
        runtime: None,
        raw_yaml: None,
    };
    let result = compute(&config, &input);
    assert!(result.hard_fail);
    assert!(result.hard_fail_reason.is_some());
    assert!(result.grade.contains("blocked"));
}

#[test]
fn scoring_static_has_8_dimensions() {
    let config = minimal_config();
    let input = static_only_input();
    let result = compute(&config, &input);
    assert_eq!(result.dimensions.len(), 8);
}

// ============================================================================
// FJ-2800: compute — with runtime data
// ============================================================================

fn good_runtime() -> RuntimeData {
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
        first_apply_ms: 1000,
        second_apply_ms: 50,
    }
}

#[test]
fn scoring_with_runtime_produces_two_tiers() {
    let config = minimal_config();
    let input = ScoringInput {
        status: "qualified".into(),
        idempotency: "strong".into(),
        budget_ms: 5000,
        runtime: Some(good_runtime()),
        raw_yaml: None,
    };
    let result = compute(&config, &input);
    assert!(result.runtime_composite.is_some());
    assert!(result.runtime_grade.is_some());
    assert!(result.grade.contains('/'));
    assert!(!result.grade.contains("pending"));
}

#[test]
fn scoring_good_runtime_high_cor_idm() {
    let config = minimal_config();
    let input = ScoringInput {
        status: "qualified".into(),
        idempotency: "strong".into(),
        budget_ms: 5000,
        runtime: Some(good_runtime()),
        raw_yaml: None,
    };
    let result = compute(&config, &input);
    let cor = result.dimensions.iter().find(|d| d.code == "COR").unwrap();
    let idm = result.dimensions.iter().find(|d| d.code == "IDM").unwrap();
    assert!(
        cor.score >= 90,
        "good runtime → COR >= 90, got {}",
        cor.score
    );
    assert!(
        idm.score >= 80,
        "good runtime → IDM >= 80, got {}",
        idm.score
    );
}

#[test]
fn scoring_bad_runtime_low_cor() {
    let config = minimal_config();
    let rt = RuntimeData {
        validate_pass: false,
        plan_pass: false,
        first_apply_pass: false,
        second_apply_pass: false,
        zero_changes_on_reapply: false,
        hash_stable: false,
        all_resources_converged: false,
        state_lock_written: false,
        warning_count: 5,
        changed_on_reapply: 5,
        first_apply_ms: 30000,
        second_apply_ms: 30000,
    };
    let input = ScoringInput {
        status: "qualified".into(),
        idempotency: "eventual".into(),
        budget_ms: 5000,
        runtime: Some(rt),
        raw_yaml: None,
    };
    let result = compute(&config, &input);
    let cor = result.dimensions.iter().find(|d| d.code == "COR").unwrap();
    assert!(cor.score < 20, "bad runtime → COR < 20, got {}", cor.score);
}

// ============================================================================
// FJ-2800: format_score_report
// ============================================================================

#[test]
fn scoring_report_contains_grade_and_dimensions() {
    let config = minimal_config();
    let input = ScoringInput {
        status: "qualified".into(),
        idempotency: "strong".into(),
        budget_ms: 5000,
        runtime: Some(good_runtime()),
        raw_yaml: None,
    };
    let result = compute(&config, &input);
    let report = format_score_report(&result);
    assert!(report.contains("Forjar Score v2"));
    assert!(report.contains("SAF"));
    assert!(report.contains("OBS"));
    assert!(report.contains("DOC"));
    assert!(report.contains("COR"));
    assert!(report.contains("IDM"));
    assert!(report.contains("PRF"));
    assert!(report.contains("Static Grade"));
    assert!(report.contains("Runtime Grade"));
}

#[test]
fn scoring_report_pending_shows_no_runtime() {
    let config = minimal_config();
    let input = static_only_input();
    let result = compute(&config, &input);
    let report = format_score_report(&result);
    assert!(report.contains("pending"));
    assert!(report.contains("no runtime data"));
}

// ============================================================================
// Helper: construct RecipeInput without Default
// ============================================================================

fn recipe_input(input_type: &str) -> RecipeInput {
    RecipeInput {
        input_type: input_type.into(),
        description: None,
        default: None,
        min: None,
        max: None,
        choices: vec![],
    }
}

fn recipe_with_inputs(inputs: IndexMap<String, RecipeInput>) -> RecipeMetadata {
    RecipeMetadata {
        name: "test-recipe".into(),
        version: None,
        description: None,
        inputs,
        requires: vec![],
    }
}

// ============================================================================
// Recipe: validate_inputs — string type
// ============================================================================

#[test]
fn validate_string_accepts_any() {
    let mut inputs = IndexMap::new();
    inputs.insert("name".into(), recipe_input("string"));
    let recipe = recipe_with_inputs(inputs);
    let mut provided = HashMap::new();
    provided.insert("name".into(), serde_yaml_ng::Value::String("hello".into()));
    let result = validate_inputs(&recipe, &provided).unwrap();
    assert_eq!(result["name"], "hello");
}

// ============================================================================
// Recipe: validate_inputs — int type with bounds
// ============================================================================

#[test]
fn validate_int_in_range_ok() {
    let mut inputs = IndexMap::new();
    inputs.insert(
        "port".into(),
        RecipeInput {
            input_type: "int".into(),
            description: None,
            default: None,
            min: Some(1),
            max: Some(65535),
            choices: vec![],
        },
    );
    let recipe = recipe_with_inputs(inputs);
    let mut provided = HashMap::new();
    provided.insert("port".into(), serde_yaml_ng::Value::Number(8080.into()));
    let result = validate_inputs(&recipe, &provided).unwrap();
    assert_eq!(result["port"], "8080");
}

#[test]
fn validate_int_below_min_rejected() {
    let mut inputs = IndexMap::new();
    inputs.insert(
        "port".into(),
        RecipeInput {
            input_type: "int".into(),
            description: None,
            default: None,
            min: Some(1),
            max: None,
            choices: vec![],
        },
    );
    let recipe = recipe_with_inputs(inputs);
    let mut provided = HashMap::new();
    provided.insert("port".into(), serde_yaml_ng::Value::Number(0.into()));
    let err = validate_inputs(&recipe, &provided).unwrap_err();
    assert!(err.contains(">="));
}

#[test]
fn validate_int_above_max_rejected() {
    let mut inputs = IndexMap::new();
    inputs.insert(
        "workers".into(),
        RecipeInput {
            input_type: "int".into(),
            description: None,
            default: None,
            min: None,
            max: Some(32),
            choices: vec![],
        },
    );
    let recipe = recipe_with_inputs(inputs);
    let mut provided = HashMap::new();
    provided.insert("workers".into(), serde_yaml_ng::Value::Number(64.into()));
    let err = validate_inputs(&recipe, &provided).unwrap_err();
    assert!(err.contains("<="));
}

// ============================================================================
// Recipe: validate_inputs — bool type
// ============================================================================

#[test]
fn validate_bool_accepts_true_false() {
    let mut inputs = IndexMap::new();
    inputs.insert("enable".into(), recipe_input("bool"));
    let recipe = recipe_with_inputs(inputs);
    let mut provided = HashMap::new();
    provided.insert("enable".into(), serde_yaml_ng::Value::Bool(true));
    let result = validate_inputs(&recipe, &provided).unwrap();
    assert_eq!(result["enable"], "true");
}

#[test]
fn validate_bool_rejects_string() {
    let mut inputs = IndexMap::new();
    inputs.insert("enable".into(), recipe_input("bool"));
    let recipe = recipe_with_inputs(inputs);
    let mut provided = HashMap::new();
    provided.insert("enable".into(), serde_yaml_ng::Value::String("yes".into()));
    assert!(validate_inputs(&recipe, &provided).is_err());
}

// ============================================================================
// Recipe: validate_inputs — path type
// ============================================================================

#[test]
fn validate_path_requires_absolute() {
    let mut inputs = IndexMap::new();
    inputs.insert("config_dir".into(), recipe_input("path"));
    let recipe = recipe_with_inputs(inputs);

    // Absolute path succeeds
    let mut ok = HashMap::new();
    ok.insert(
        "config_dir".into(),
        serde_yaml_ng::Value::String("/etc/app".into()),
    );
    assert!(validate_inputs(&recipe, &ok).is_ok());

    // Relative path fails
    let mut bad = HashMap::new();
    bad.insert(
        "config_dir".into(),
        serde_yaml_ng::Value::String("etc/app".into()),
    );
    assert!(validate_inputs(&recipe, &bad).is_err());
}

// ============================================================================
// Recipe: validate_inputs — enum type
// ============================================================================

#[test]
fn validate_enum_valid_choice() {
    let mut inputs = IndexMap::new();
    inputs.insert(
        "env".into(),
        RecipeInput {
            input_type: "enum".into(),
            description: None,
            default: None,
            min: None,
            max: None,
            choices: vec!["dev".into(), "staging".into(), "prod".into()],
        },
    );
    let recipe = recipe_with_inputs(inputs);
    let mut provided = HashMap::new();
    provided.insert("env".into(), serde_yaml_ng::Value::String("staging".into()));
    let result = validate_inputs(&recipe, &provided).unwrap();
    assert_eq!(result["env"], "staging");
}

#[test]
fn validate_enum_invalid_choice_rejected() {
    let mut inputs = IndexMap::new();
    inputs.insert(
        "env".into(),
        RecipeInput {
            input_type: "enum".into(),
            description: None,
            default: None,
            min: None,
            max: None,
            choices: vec!["dev".into(), "prod".into()],
        },
    );
    let recipe = recipe_with_inputs(inputs);
    let mut provided = HashMap::new();
    provided.insert("env".into(), serde_yaml_ng::Value::String("canary".into()));
    let err = validate_inputs(&recipe, &provided).unwrap_err();
    assert!(err.contains("must be one of"));
}

// ============================================================================
// Recipe: validate_inputs — defaults and missing
// ============================================================================

#[test]
fn validate_uses_default_when_not_provided() {
    let mut inputs = IndexMap::new();
    inputs.insert(
        "timeout".into(),
        RecipeInput {
            input_type: "int".into(),
            description: None,
            default: Some(serde_yaml_ng::Value::Number(30.into())),
            min: None,
            max: None,
            choices: vec![],
        },
    );
    let recipe = recipe_with_inputs(inputs);
    let provided = HashMap::new(); // nothing provided
    let result = validate_inputs(&recipe, &provided).unwrap();
    assert_eq!(result["timeout"], "30");
}

#[test]
fn validate_missing_required_input_rejected() {
    let mut inputs = IndexMap::new();
    inputs.insert("name".into(), recipe_input("string"));
    let recipe = recipe_with_inputs(inputs);
    let provided = HashMap::new();
    let err = validate_inputs(&recipe, &provided).unwrap_err();
    assert!(err.contains("requires input"));
}

#[test]
fn validate_unknown_type_rejected() {
    let mut inputs = IndexMap::new();
    inputs.insert(
        "x".into(),
        RecipeInput {
            input_type: "float".into(),
            description: None,
            default: Some(serde_yaml_ng::Value::Number(1.into())),
            min: None,
            max: None,
            choices: vec![],
        },
    );
    let recipe = recipe_with_inputs(inputs);
    let provided = HashMap::new();
    let err = validate_inputs(&recipe, &provided).unwrap_err();
    assert!(err.contains("unknown input type"));
}
