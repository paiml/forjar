//! Tests for the scoring module — part 1: helpers, hard-fail, static dimensions.

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
        pre_apply: None,
        post_apply: None,
        lifecycle: None,
        store: false,
        sudo: false,
        script: None,
        gather: vec![],
        scatter: vec![],
    }
}

pub(super) fn static_input() -> ScoringInput {
    ScoringInput {
        status: "qualified".to_string(),
        idempotency: "strong".to_string(),
        budget_ms: 0,
        runtime: None,
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
// Hard-fail tests
// ============================================================================

#[test]
fn hard_fail_blocked_status() {
    let config = minimal_config();
    let input = ScoringInput {
        status: "blocked".to_string(),
        idempotency: "strong".to_string(),
        budget_ms: 0,
        runtime: None,
    };
    let result = compute(&config, &input);
    assert_eq!(result.grade, 'F');
    assert!(result.hard_fail);
    assert!(result.hard_fail_reason.unwrap().contains("blocked"));
}

#[test]
fn hard_fail_pending_status() {
    let config = minimal_config();
    let input = ScoringInput {
        status: "pending".to_string(),
        idempotency: "strong".to_string(),
        budget_ms: 0,
        runtime: None,
    };
    let result = compute(&config, &input);
    assert_eq!(result.grade, 'F');
    assert!(result.hard_fail);
}

// ============================================================================
// Grade gate tests
// ============================================================================

#[test]
fn grade_a_requires_all_dimensions_above_80() {
    let mut config = minimal_config();
    let input = static_input();
    let result = compute(&config, &input);
    assert_ne!(result.grade, 'A');
    assert!(result.composite <= 100);

    config.description = Some("A well-documented config".to_string());
    config.policy.failure = FailurePolicy::ContinueIndependent;
    config.policy.ssh_retries = 3;
    config.policy.pre_apply = Some("echo pre".to_string());
    config.policy.post_apply = Some("echo post".to_string());
    config.policy.notify.on_success = Some("echo ok".to_string());
    config.policy.notify.on_failure = Some("echo fail".to_string());
    config.policy.notify.on_drift = Some("echo drift".to_string());

    let mut file_res = minimal_resource(ResourceType::File);
    file_res.mode = Some("0644".to_string());
    file_res.owner = Some("root".to_string());
    file_res.tags = vec!["web".to_string()];
    file_res.resource_group = Some("infra".to_string());
    file_res.content = Some("# {{params.name}}\nconfig".to_string());
    file_res.depends_on = vec!["pkg".to_string()];

    let mut pkg_res = minimal_resource(ResourceType::Package);
    pkg_res.version = Some("1.0".to_string());
    pkg_res.tags = vec!["base".to_string()];

    config.resources.insert("cfg".to_string(), file_res);
    config.resources.insert("pkg".to_string(), pkg_res);
    config.params.insert(
        "name".to_string(),
        serde_yaml_ng::Value::String("test".into()),
    );
    config.outputs.insert(
        "out".to_string(),
        OutputValue {
            value: "{{params.name}}".to_string(),
            description: Some("output".to_string()),
        },
    );

    let input = ScoringInput {
        status: "qualified".to_string(),
        idempotency: "strong".to_string(),
        budget_ms: 60000,
        runtime: Some(full_runtime()),
    };
    let result = compute(&config, &input);
    assert!(result.composite >= 75, "composite={}", result.composite);
}

// ============================================================================
// Safety dimension tests
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

// ============================================================================
// Observability dimension tests
// ============================================================================

#[test]
fn observability_defaults_get_30() {
    let config = minimal_config();
    let input = static_input();
    let result = compute(&config, &input);
    let obs = result.dimensions.iter().find(|d| d.code == "OBS").unwrap();
    assert_eq!(obs.score, 30);
}

#[test]
fn observability_with_outputs_and_notify() {
    let mut config = minimal_config();
    config.outputs.insert(
        "x".to_string(),
        OutputValue {
            value: "v".to_string(),
            description: None,
        },
    );
    config.policy.notify.on_success = Some("echo ok".to_string());
    config.policy.notify.on_failure = Some("echo fail".to_string());
    config.policy.notify.on_drift = Some("echo drift".to_string());

    let input = static_input();
    let result = compute(&config, &input);
    let obs = result.dimensions.iter().find(|d| d.code == "OBS").unwrap();
    assert_eq!(obs.score, 60);
}

// ============================================================================
// Resilience dimension tests
// ============================================================================

#[test]
fn resilience_empty_config_scores_zero() {
    let config = minimal_config();
    let input = static_input();
    let result = compute(&config, &input);
    let res = result.dimensions.iter().find(|d| d.code == "RES").unwrap();
    assert_eq!(res.score, 0);
}

#[test]
fn resilience_continue_independent_adds_20() {
    let mut config = minimal_config();
    config.policy.failure = FailurePolicy::ContinueIndependent;

    let input = static_input();
    let result = compute(&config, &input);
    let res = result.dimensions.iter().find(|d| d.code == "RES").unwrap();
    assert_eq!(res.score, 20);
}

// ============================================================================
// Composability dimension tests
// ============================================================================

#[test]
fn composability_empty_config() {
    let config = minimal_config();
    let input = static_input();
    let result = compute(&config, &input);
    let cmp = result.dimensions.iter().find(|d| d.code == "CMP").unwrap();
    assert_eq!(cmp.score, 0);
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
    assert_eq!(cmp.score, 35);
}
