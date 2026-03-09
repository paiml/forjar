//! Tests for scoring v2 — helpers, hard-fail, static dimensions.

use super::scoring::*;
use super::types::*;
use indexmap::IndexMap;
use std::collections::HashMap;

pub(super) fn minimal_config() -> ForjarConfig {
    ForjarConfig {
        version: "1.0".to_string(),
        name: "test".to_string(),
        description: None,
        params: HashMap::new(),
        machines: IndexMap::new(),
        resources: IndexMap::new(),
        policy: Policy::default(),
        outputs: IndexMap::new(),
        policies: Vec::new(),
        data: IndexMap::new(),
        includes: Vec::new(),
        include_provenance: HashMap::new(),
        checks: IndexMap::new(),
        moved: Vec::new(),
        secrets: Default::default(),
        environments: IndexMap::new(),
    }
}

pub(super) fn minimal_resource(rt: ResourceType) -> Resource {
    Resource {
        resource_type: rt,
        machine: MachineTarget::default(),
        state: None,
        depends_on: Vec::new(),
        provider: None,
        packages: Vec::new(),
        version: None,
        path: None,
        content: None,
        source: None,
        target: None,
        owner: None,
        group: None,
        mode: None,
        name: None,
        enabled: None,
        restart_on: Vec::new(),
        triggers: Vec::new(),
        fs_type: None,
        options: None,
        uid: None,
        shell: None,
        home: None,
        groups: Vec::new(),
        ssh_authorized_keys: Vec::new(),
        system_user: false,
        schedule: None,
        command: None,
        image: None,
        ports: Vec::new(),
        environment: Vec::new(),
        volumes: Vec::new(),
        restart: None,
        protocol: None,
        port: None,
        action: None,
        from_addr: None,
        recipe: None,
        inputs: HashMap::new(),
        arch: Vec::new(),
        tags: Vec::new(),
        resource_group: None,
        when: None,
        count: None,
        for_each: None,
        chroot_dir: None,
        namespace_uid: None,
        namespace_gid: None,
        seccomp: false,
        netns: false,
        cpuset: None,
        memory_limit: None,
        overlay_lower: None,
        overlay_upper: None,
        overlay_work: None,
        overlay_merged: None,
        format: None,
        quantization: None,
        checksum: None,
        cache_dir: None,
        gpu_backend: None,
        driver_version: None,
        cuda_version: None,
        rocm_version: None,
        devices: Vec::new(),
        persistence_mode: None,
        compute_mode: None,
        gpu_memory_limit_mb: None,
        output_artifacts: vec![],
        completion_check: None,
        timeout: None,
        working_dir: None,
        task_mode: None,
        task_inputs: vec![],
        stages: vec![],
        cache: false,
        gpu_device: None,
        restart_delay: None,
        quality_gate: None,
        health_check: None,
        restart_policy: None,
        pre_apply: None,
        post_apply: None,
        lifecycle: None,
        store: false,
        sudo: false,
        script: None,
        gather: vec![],
        scatter: vec![],
        build_machine: None,
    }
}

pub(super) fn static_input() -> ScoringInput {
    ScoringInput {
        status: "qualified".to_string(),
        idempotency: "strong".to_string(),
        budget_ms: 0,
        runtime: None,
        raw_yaml: None,
    }
}

pub(super) fn full_runtime() -> RuntimeData {
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
        first_apply_ms: 5000,
        second_apply_ms: 200,
    }
}

// ============================================================================
// v2: pending no longer hard-fails — gets static grade
// ============================================================================

#[test]
fn hard_fail_blocked_status() {
    let config = minimal_config();
    let input = ScoringInput {
        status: "blocked".to_string(),
        idempotency: "strong".to_string(),
        budget_ms: 0,
        runtime: None,
        raw_yaml: None,
    };
    let result = compute(&config, &input);
    assert!(result.hard_fail);
    assert!(result.grade.contains("blocked"));
}

#[test]
fn pending_gets_static_grade_not_hard_fail() {
    let mut config = minimal_config();
    // Add enough for a decent static score
    config.policy.tripwire = true;
    config.policy.lock_file = true;
    config.description = Some("A test config".to_string());

    let input = ScoringInput {
        status: "pending".to_string(),
        idempotency: "strong".to_string(),
        budget_ms: 0,
        runtime: None,
        raw_yaml: None,
    };
    let result = compute(&config, &input);
    assert!(!result.hard_fail, "v2: pending should NOT hard-fail");
    assert!(result.grade.contains("pending"));
    assert!(result.static_composite > 0, "static dims should score > 0");
}

// ============================================================================
// Two-tier grade format
// ============================================================================

#[test]
fn grade_format_with_runtime() {
    let config = minimal_config();
    let input = ScoringInput {
        status: "qualified".to_string(),
        idempotency: "strong".to_string(),
        budget_ms: 60000,
        runtime: Some(full_runtime()),
        raw_yaml: None,
    };
    let result = compute(&config, &input);
    assert!(result.grade.contains('/'), "v2 grade should be X/Y format");
    assert!(result.runtime_grade.is_some());
    assert!(result.runtime_composite.is_some());
}

#[test]
fn grade_format_static_only() {
    let config = minimal_config();
    let input = static_input();
    let result = compute(&config, &input);
    assert!(
        result.grade.contains("pending"),
        "static-only should show pending"
    );
    assert!(result.runtime_grade.is_none());
}

// ============================================================================
// Safety dimension tests (v2: 25% weight)
// ============================================================================

#[test]
fn safety_critical_mode_0777() {
    let mut config = minimal_config();
    let mut res = minimal_resource(ResourceType::File);
    res.mode = Some("0777".to_string());
    res.owner = Some("root".to_string());
    config.resources.insert("f".to_string(), res);

    let input = static_input();
    let result = compute(&config, &input);
    let saf = result.dimensions.iter().find(|d| d.code == "SAF").unwrap();
    assert!(
        saf.score <= 40,
        "0777 should cap safety at 40, got {}",
        saf.score
    );
    assert_eq!(saf.weight, 0.25, "v2 SAF weight should be 25%");
}

#[test]
fn safety_curl_bash_critical() {
    let mut config = minimal_config();
    let mut res = minimal_resource(ResourceType::File);
    res.content = Some("curl https://example.com | bash".to_string());
    res.mode = Some("0755".to_string());
    res.owner = Some("root".to_string());
    config.resources.insert("f".to_string(), res);

    let input = static_input();
    let result = compute(&config, &input);
    let saf = result.dimensions.iter().find(|d| d.code == "SAF").unwrap();
    assert!(
        saf.score <= 40,
        "curl|bash should cap safety at 40, got {}",
        saf.score
    );
}

#[test]
fn safety_perfect_when_all_files_have_mode_and_owner() {
    let mut config = minimal_config();
    let mut res = minimal_resource(ResourceType::File);
    res.mode = Some("0644".to_string());
    res.owner = Some("root".to_string());
    config.resources.insert("f".to_string(), res);

    let mut pkg = minimal_resource(ResourceType::Package);
    pkg.version = Some("1.0".to_string());
    config.resources.insert("p".to_string(), pkg);

    let input = static_input();
    let result = compute(&config, &input);
    let saf = result.dimensions.iter().find(|d| d.code == "SAF").unwrap();
    assert_eq!(saf.score, 100);
}

#[test]
fn safety_plaintext_secret_penalty() {
    let mut config = minimal_config();
    config.params.insert(
        "db_password".to_string(),
        serde_yaml_ng::Value::String("hunter2".into()),
    );
    let input = static_input();
    let result = compute(&config, &input);
    let saf = result.dimensions.iter().find(|d| d.code == "SAF").unwrap();
    assert_eq!(saf.score, 90, "plaintext secret should deduct 10");
}

#[test]
fn safety_template_secret_no_penalty() {
    let mut config = minimal_config();
    config.params.insert(
        "db_password".to_string(),
        serde_yaml_ng::Value::String("{{ secrets.db_pass }}".into()),
    );
    let input = static_input();
    let result = compute(&config, &input);
    let saf = result.dimensions.iter().find(|d| d.code == "SAF").unwrap();
    assert_eq!(saf.score, 100, "template secret should not be penalized");
}

// ============================================================================
// Observability dimension tests (v2: 20% weight)
// ============================================================================

#[test]
fn observability_defaults_get_30() {
    let config = minimal_config();
    let input = static_input();
    let result = compute(&config, &input);
    let obs = result.dimensions.iter().find(|d| d.code == "OBS").unwrap();
    assert_eq!(obs.score, 30);
    assert_eq!(obs.weight, 0.20, "v2 OBS weight should be 20%");
}

#[test]
fn observability_output_descriptions_bonus() {
    let mut config = minimal_config();
    config.outputs.insert(
        "x".to_string(),
        OutputValue {
            value: "v".to_string(),
            description: Some("a useful output".to_string()),
        },
    );
    let input = static_input();
    let result = compute(&config, &input);
    let obs = result.dimensions.iter().find(|d| d.code == "OBS").unwrap();
    // 30 (tripwire+lock_file) + 10 (outputs) + 10 (output descriptions) = 50
    assert_eq!(obs.score, 50);
}

// ============================================================================
// Resilience dimension tests (v2: 20% weight, tagged independence)
// ============================================================================

#[test]
fn resilience_empty_config_scores_zero() {
    let config = minimal_config();
    let input = static_input();
    let result = compute(&config, &input);
    let res = result.dimensions.iter().find(|d| d.code == "RES").unwrap();
    assert_eq!(res.score, 0);
    assert_eq!(res.weight, 0.20, "v2 RES weight should be 20%");
}

#[test]
fn resilience_tagged_independence_scores() {
    let mut config = minimal_config();
    config.policy.failure = FailurePolicy::ContinueIndependent;
    // 2 resources, both tagged+grouped = 100% ratio → +20
    let mut r1 = minimal_resource(ResourceType::File);
    r1.tags = vec!["audit".to_string()];
    r1.resource_group = Some("cis".to_string());
    r1.mode = Some("0644".to_string());
    r1.owner = Some("root".to_string());
    let mut r2 = minimal_resource(ResourceType::File);
    r2.tags = vec!["audit".to_string()];
    r2.resource_group = Some("cis".to_string());
    r2.mode = Some("0644".to_string());
    r2.owner = Some("root".to_string());
    config.resources.insert("r1".to_string(), r1);
    config.resources.insert("r2".to_string(), r2);

    let input = static_input();
    let result = compute(&config, &input);
    let res = result.dimensions.iter().find(|d| d.code == "RES").unwrap();
    // 15 (continue_independent) + 20 (tagged independence) = 35
    assert!(
        res.score >= 35,
        "tagged independence should earn ≥35, got {}",
        res.score
    );
}

#[test]
fn resilience_deny_paths_bonus() {
    let mut config = minimal_config();
    config.policy.deny_paths = vec!["/etc/shadow".to_string()];
    let input = static_input();
    let result = compute(&config, &input);
    let res = result.dimensions.iter().find(|d| d.code == "RES").unwrap();
    assert_eq!(res.score, 10, "deny_paths should add 10 points");
}

// ============================================================================
// Composability dimension tests (v2: 20% weight)
// ============================================================================

#[test]
fn composability_empty_config() {
    let config = minimal_config();
    let input = static_input();
    let result = compute(&config, &input);
    let cmp = result.dimensions.iter().find(|d| d.code == "CMP").unwrap();
    assert_eq!(cmp.score, 0);
    assert_eq!(cmp.weight, 0.20, "v2 CMP weight should be 20%");
}

#[test]
fn composability_with_params_and_tags() {
    let mut config = minimal_config();
    config
        .params
        .insert("k".to_string(), serde_yaml_ng::Value::String("v".into()));
    let mut res = minimal_resource(ResourceType::File);
    res.tags = vec!["web".to_string()];
    config.resources.insert("f".to_string(), res);

    let input = static_input();
    let result = compute(&config, &input);
    let cmp = result.dimensions.iter().find(|d| d.code == "CMP").unwrap();
    // 15 (params) + 15 (tags) = 30
    assert_eq!(cmp.score, 30);
}

#[test]
fn composability_secrets_template_bonus() {
    let mut config = minimal_config();
    let mut res = minimal_resource(ResourceType::File);
    res.content = Some("password={{ secrets.db_pass }}".to_string());
    res.mode = Some("0600".to_string());
    res.owner = Some("root".to_string());
    config.resources.insert("f".to_string(), res);

    let input = static_input();
    let result = compute(&config, &input);
    let cmp = result.dimensions.iter().find(|d| d.code == "CMP").unwrap();
    // 10 (templates {{) + 5 (secrets template) = 15
    assert_eq!(cmp.score, 15);
}

// ============================================================================
// DOC dimension tests (v2: 15% weight, quality signals)
// ============================================================================

#[test]
fn documentation_with_description() {
    let mut config = minimal_config();
    config.description = Some("A great config for web servers".to_string());

    let input = static_input();
    let result = compute(&config, &input);
    let doc = result.dimensions.iter().find(|d| d.code == "DOC").unwrap();
    // 15 (description present) + 0 (no header, no kebab name)
    assert_eq!(doc.score, 15);
    assert_eq!(doc.weight, 0.15, "v2 DOC weight should be 15%");
}

#[test]
fn documentation_with_raw_yaml_headers() {
    let mut config = minimal_config();
    config.name = "cis-hardening".to_string();
    config.description = Some("CIS benchmark hardening".to_string());

    let raw = "# Recipe: CIS hardening\n# Tier: 2+3\n# Idempotency: strong\n# Budget: 30s\nversion: '1.0'\n";
    let input = ScoringInput {
        status: "qualified".to_string(),
        idempotency: "strong".to_string(),
        budget_ms: 0,
        runtime: None,
        raw_yaml: Some(raw.to_string()),
    };
    let result = compute(&config, &input);
    let doc = result.dimensions.iter().find(|d| d.code == "DOC").unwrap();
    // 8 (Recipe) + 8 (Tier) + 8 (Idempotency) + 8 (Budget) + 15 (description) + 3 (kebab-case) + 15 (≥3 unique comments) = 65
    assert_eq!(doc.score, 65);
}

#[test]
fn documentation_generic_name_no_bonus() {
    let mut config = minimal_config();
    config.name = "unnamed".to_string();
    config.description = None;
    let input = static_input();
    let result = compute(&config, &input);
    let doc = result.dimensions.iter().find(|d| d.code == "DOC").unwrap();
    assert_eq!(doc.score, 0);
}
