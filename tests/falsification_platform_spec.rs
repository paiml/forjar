//! Popperian falsification tests for FJ-2000 through FJ-2803.
//!
//! Each test directly implements a rejection criterion from the platform spec.
//! If a test passes, the implementation survives falsification. If it fails,
//! the spec claim is falsified and must be corrected.

#![allow(clippy::field_reassign_with_default)]

use tempfile::TempDir;

// ============================================================================
// FJ-2001/FJ-2004: SQLite Query Engine
// ============================================================================

/// F-2001-1: open_state_db creates a working SQLite DB with FTS5.
#[test]
fn f_2001_1_sqlite_opens_with_fts5() {
    use forjar::core::store::db::{fts5_search, open_state_db};

    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("state.db");
    let conn = open_state_db(&db_path).unwrap();

    // Empty FTS5 search returns empty vec, not error
    let results = fts5_search(&conn, "nginx", 10).unwrap();
    assert!(results.is_empty());
}

/// F-2001-2: Schema version constant exists and is positive.
#[test]
fn f_2001_2_schema_version_defined() {
    use forjar::core::store::db::SCHEMA_VERSION;

    let ver = SCHEMA_VERSION;
    assert!(ver > 0, "SCHEMA_VERSION must be positive");
}

/// F-2001-3: FTS5 search returns results after inserting data.
#[test]
fn f_2001_3_fts5_search_returns_matches() {
    use forjar::core::store::db::{fts5_search, open_state_db};

    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("state.db");
    let conn = open_state_db(&db_path).unwrap();

    conn.execute(
        "INSERT INTO machines (id, name, hostname, transport, first_seen, last_seen) \
         VALUES (1, 'web', 'web-01', 'ssh', '2026-03-09', '2026-03-09')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO generations (id, generation_num, run_id, config_hash, created_at) \
         VALUES (1, 1, 'run-1', 'hash0', '2026-03-09')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO resources (machine_id, resource_id, generation_id, resource_type, status, state_hash, applied_at) \
         VALUES (1, 'nginx-config', 1, 'file', 'converged', 'hash1', '2026-03-09')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO resources_fts (resource_id, resource_type, path) \
         VALUES ('nginx-config', 'file', '/etc/nginx/nginx.conf')",
        [],
    )
    .unwrap();

    let results = fts5_search(&conn, "nginx", 10).unwrap();
    assert!(!results.is_empty(), "FTS5 must find 'nginx'");
    assert_eq!(results[0].resource_id, "nginx-config");
}

// ============================================================================
// FJ-2002/FJ-2003: Generation Model & Undo
// ============================================================================

/// F-2002-1: GenerationMeta captures config hash, git ref, action, and deltas.
#[test]
fn f_2002_1_generation_meta_captures_all_fields() {
    use forjar::core::types::{GenerationMeta, MachineDelta};

    let mut meta = GenerationMeta::new(5, "2026-03-09T12:00:00Z".into());
    meta.config_hash = Some("blake3:abc123".into());
    meta.git_ref = Some("main@abc1234".into());
    meta.parent_generation = Some(4);
    meta.operator = Some("root".into());
    meta.forjar_version = Some("0.9.0".into());

    let delta = MachineDelta {
        created: vec!["nginx".into(), "redis".into()],
        updated: vec!["config".into()],
        destroyed: vec![],
        unchanged: 5,
    };
    meta.record_machine("web", delta);

    assert_eq!(meta.generation, 5);
    assert_eq!(meta.config_hash.as_deref(), Some("blake3:abc123"));
    assert_eq!(meta.git_ref.as_deref(), Some("main@abc1234"));
    assert_eq!(meta.parent_generation, Some(4));
    assert_eq!(meta.machines.len(), 1);
    assert_eq!(meta.total_changes(), 3);
}

/// F-2002-2: GenerationMeta YAML roundtrip preserves all fields.
#[test]
fn f_2002_2_generation_meta_yaml_roundtrip() {
    use forjar::core::types::{GenerationMeta, MachineDelta};

    let mut meta = GenerationMeta::new_undo(3, "2026-03-09T10:00:00Z".into(), 2);
    meta.config_hash = Some("blake3:def456".into());
    meta.git_ref = Some("feature@789".into());
    meta.operator = Some("admin".into());
    meta.forjar_version = Some("0.9.1".into());
    meta.bashrs_version = Some("1.0.0".into());

    let delta = MachineDelta {
        created: vec![],
        updated: vec!["a".into(), "b".into(), "c".into()],
        destroyed: vec!["old".into()],
        unchanged: 0,
    };
    meta.record_machine("gpu", delta);

    let yaml = serde_yaml_ng::to_string(&meta).unwrap();
    let parsed: GenerationMeta = serde_yaml_ng::from_str(&yaml).unwrap();

    assert_eq!(parsed.generation, meta.generation);
    assert_eq!(parsed.config_hash, meta.config_hash);
    assert_eq!(parsed.git_ref, meta.git_ref);
    assert_eq!(parsed.action, "undo");
    assert_eq!(parsed.parent_generation, Some(2));
    assert_eq!(parsed.machines.len(), 1);
    assert!(parsed.is_undo());
}

/// F-2003-1: Generation diff computes correct resource differences.
#[test]
fn f_2003_1_generation_diff_correct() {
    use forjar::core::types::{diff_resource_sets, DiffAction};

    let old = vec![
        ("nginx", "file", "hash_a"),
        ("redis", "package", "hash_b"),
        ("gone", "service", "hash_c"),
    ];
    let new = vec![
        ("nginx", "file", "hash_a"),       // unchanged
        ("redis", "package", "hash_x"),    // modified
        ("postgres", "package", "hash_d"), // added
    ];

    let diffs = diff_resource_sets(&old, &new);

    let added = diffs
        .iter()
        .filter(|d| d.action == DiffAction::Added)
        .count();
    let modified = diffs
        .iter()
        .filter(|d| d.action == DiffAction::Modified)
        .count();
    let removed = diffs
        .iter()
        .filter(|d| d.action == DiffAction::Removed)
        .count();

    assert_eq!(added, 1, "postgres should be added");
    assert_eq!(modified, 1, "redis should be modified");
    assert_eq!(removed, 1, "gone should be removed");
}

/// F-2003-2: GenerationDiff summary counts are correct.
#[test]
fn f_2003_2_generation_diff_summary_counts() {
    use forjar::core::types::{GenerationDiff, ResourceDiff};

    let diff = GenerationDiff {
        gen_from: 3,
        gen_to: 7,
        machine: "web".into(),
        resources: vec![
            ResourceDiff::added("new-pkg", "package"),
            ResourceDiff::removed("old-svc", "service"),
            ResourceDiff::modified("config", "file"),
            ResourceDiff::unchanged("stable", "file"),
        ],
    };

    assert_eq!(diff.added_count(), 1);
    assert_eq!(diff.removed_count(), 1);
    assert_eq!(diff.modified_count(), 1);
    assert_eq!(diff.unchanged_count(), 1);
    assert_eq!(diff.change_count(), 3);
    assert!(diff.has_changes());
}

// ============================================================================
// FJ-2006: Idempotency — CQRS flat files as source of truth
// ============================================================================

/// F-2006-1: State.db can be rebuilt from scratch (CQRS derived read model).
#[test]
fn f_2006_1_state_db_is_derived_read_model() {
    use forjar::core::store::db::{list_all_resources, open_state_db};

    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("state.db");
    let conn = open_state_db(&db_path).unwrap();

    let resources = list_all_resources(&conn, 100).unwrap();
    assert!(resources.is_empty(), "fresh database = zero resources");
}

// ============================================================================
// FJ-2500: Config Validation — Unknown Field Detection
// ============================================================================

/// F-2500-1: Unknown fields produce warnings.
#[test]
fn f_2500_1_unknown_field_detected() {
    use forjar::core::parser::check_unknown_fields;

    let yaml = r#"
version: "1.0"
name: test
machnes:
  web:
    hostname: web-01
    addr: 127.0.0.1
resources:
  nginx:
    type: package
    machine: web
    provider: apt
    packages: [nginx]
"#;

    let issues = check_unknown_fields(yaml);
    assert!(
        !issues.is_empty(),
        "'machnes' should be detected as unknown"
    );
}

/// F-2500-2: Valid fields produce zero warnings.
#[test]
fn f_2500_2_valid_fields_no_warnings() {
    use forjar::core::parser::check_unknown_fields;

    let yaml = r#"
version: "1.0"
name: test
machines:
  web:
    hostname: web-01
    addr: 127.0.0.1
resources:
  nginx:
    type: package
    machine: web
    provider: apt
    packages: [nginx]
"#;

    let issues = check_unknown_fields(yaml);
    assert!(issues.is_empty(), "valid config: {issues:?}");
}

// ============================================================================
// FJ-2700: Task Framework — Quality Gates
// ============================================================================

/// F-2700-1: Default gate blocks on non-zero exit code.
#[test]
fn f_2700_1_exit_code_gate_blocks() {
    use forjar::core::task::{evaluate_gate, GateResult};
    use forjar::core::types::QualityGate;

    let gate = QualityGate::default();
    assert_eq!(evaluate_gate(&gate, 0, ""), GateResult::Pass);
    assert!(matches!(
        evaluate_gate(&gate, 1, ""),
        GateResult::Fail(_, _)
    ));
}

/// F-2700-2: JSON field gate evaluates min threshold.
#[test]
fn f_2700_2_json_gate_threshold() {
    use forjar::core::task::{evaluate_gate, GateResult};
    use forjar::core::types::QualityGate;

    let gate = QualityGate {
        parse: Some("json".into()),
        field: Some("coverage".into()),
        min: Some(80.0),
        ..Default::default()
    };

    assert_eq!(
        evaluate_gate(&gate, 0, r#"{"coverage": 95.0}"#),
        GateResult::Pass
    );
    assert!(matches!(
        evaluate_gate(&gate, 0, r#"{"coverage": 75.0}"#),
        GateResult::Fail(_, _)
    ));
}

/// F-2700-3: Regex gate matches stdout patterns.
#[test]
fn f_2700_3_regex_gate_matches() {
    use forjar::core::task::{evaluate_gate, GateResult};
    use forjar::core::types::QualityGate;

    let gate = QualityGate {
        regex: Some("SUCCESS".into()),
        ..Default::default()
    };

    assert_eq!(
        evaluate_gate(&gate, 0, "Build: SUCCESS in 5s"),
        GateResult::Pass
    );
    assert!(matches!(
        evaluate_gate(&gate, 0, "Build: FAILED in 5s"),
        GateResult::Fail(_, _)
    ));
}

// ============================================================================
// FJ-2800–FJ-2803: ForjarScore v2 Popperian Falsification
// ============================================================================

use forjar::core::scoring::{compute, RuntimeData, ScoringInput};
use forjar::core::types::{ForjarConfig, OutputValue, Resource, ResourceType};

fn base_config() -> ForjarConfig {
    let mut config = ForjarConfig::default();
    config.name = "test-recipe".into();
    config.version = "1.0".into();
    config
}

fn base_input() -> ScoringInput {
    ScoringInput {
        status: "qualified".into(),
        idempotency: "strong".into(),
        budget_ms: 60_000,
        runtime: None,
        raw_yaml: None,
    }
}

#[allow(dead_code)]
fn full_runtime() -> RuntimeData {
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
        first_apply_ms: 20_000,
        second_apply_ms: 500,
    }
}

// --- SAF Dimension ---

/// FJ-2803 SAF Falsifier: mode:0777 on secret file must score SAF <= 40.
#[test]
fn f_2803_saf_0777_secret_file_capped_at_40() {
    let mut config = base_config();
    let mut file = Resource::default();
    file.resource_type = ResourceType::File;
    file.mode = Some("0777".into());
    file.content = Some("{{ secrets.db_password }}".into());
    config.resources.insert("dangerous".into(), file);

    let result = compute(&config, &base_input());
    let saf = result.dimensions.iter().find(|d| d.code == "SAF").unwrap();
    assert!(saf.score <= 40, "SAF must be <= 40, got: {}", saf.score);
}

/// FJ-2803 SAF Boundary: Zero file resources must score SAF=100.
#[test]
fn f_2803_saf_no_files_scores_100() {
    let mut config = base_config();
    let mut pkg = Resource::default();
    pkg.resource_type = ResourceType::Package;
    pkg.version = Some("1.0".into());
    config.resources.insert("nginx".into(), pkg);

    let result = compute(&config, &base_input());
    let saf = result.dimensions.iter().find(|d| d.code == "SAF").unwrap();
    assert_eq!(saf.score, 100, "SAF=100 with no files, got: {}", saf.score);
}

/// FJ-2803 SAF: curl|bash triggers critical safety cap.
#[test]
fn f_2803_saf_curl_bash_critical() {
    let mut config = base_config();
    let mut file = Resource::default();
    file.resource_type = ResourceType::File;
    file.content = Some("curl https://example.com/install.sh | bash".into());
    config.resources.insert("installer".into(), file);

    let result = compute(&config, &base_input());
    let saf = result.dimensions.iter().find(|d| d.code == "SAF").unwrap();
    assert!(
        saf.score <= 40,
        "curl|bash must cap SAF at 40, got: {}",
        saf.score
    );
}

// --- OBS Dimension ---

/// FJ-2803 OBS Falsifier: Full observability config must score OBS >= 90.
#[test]
fn f_2803_obs_full_config_at_least_90() {
    let mut config = base_config();
    config.policy.tripwire = true;
    config.policy.lock_file = true;
    config.policy.notify.on_success = Some("echo ok".into());
    config.policy.notify.on_failure = Some("echo fail".into());
    config.policy.notify.on_drift = Some("echo drift".into());

    let out = OutputValue {
        value: "{{params.result}}".into(),
        description: Some("Result output".into()),
    };
    config.outputs.insert("result".into(), out);

    let mut file = Resource::default();
    file.resource_type = ResourceType::File;
    file.mode = Some("0644".into());
    file.owner = Some("root".into());
    config.resources.insert("etc-conf".into(), file);

    let result = compute(&config, &base_input());
    let obs = result.dimensions.iter().find(|d| d.code == "OBS").unwrap();
    assert!(
        obs.score >= 90,
        "full OBS must score >= 90, got: {}",
        obs.score
    );
}

/// FJ-2803 OBS Boundary: Disabled policy, no outputs must score OBS <= 15.
#[test]
fn f_2803_obs_disabled_at_most_15() {
    let mut config = base_config();
    config.policy.tripwire = false;
    config.policy.lock_file = false;
    let result = compute(&config, &base_input());
    let obs = result.dimensions.iter().find(|d| d.code == "OBS").unwrap();
    assert!(
        obs.score <= 15,
        "disabled OBS must score <= 15, got: {}",
        obs.score
    );
}
