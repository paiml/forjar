//! FJ-044/1387: Compliance benchmarks & Docker→pepita migration falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-1387: CIS benchmark (world-writable, root-tmp, service restart, version pin)
//! - FJ-1387: NIST 800-53 (AC-3, AC-6, CM-6, SC-28, SI-7)
//! - FJ-1387: SOC2 (CC6.1, CC7.2)
//! - FJ-1387: HIPAA (164.312a, 164.312e)
//! - FJ-1387: Unknown benchmark handling, severity counting
//! - FJ-044: Docker→pepita state mapping, port→netns, volumes/env/restart warnings
//! - FJ-044: Full config migration preserving non-Docker resources
//!
//! Usage: cargo test --test falsification_compliance_migration

use forjar::core::compliance::{
    count_by_severity, evaluate_benchmark, supported_benchmarks, FindingSeverity,
};
use forjar::core::migrate::{docker_to_pepita, migrate_config};
use forjar::core::types::{ForjarConfig, MachineTarget, Policy, Resource, ResourceType};
use indexmap::IndexMap;
use std::collections::HashMap;

// ============================================================================
// Helpers
// ============================================================================

fn empty_config() -> ForjarConfig {
    ForjarConfig {
        version: "1.0".to_string(),
        name: "test".to_string(),
        description: None,
        params: HashMap::new(),
        machines: IndexMap::new(),
        resources: IndexMap::new(),
        policy: Policy::default(),
        outputs: IndexMap::new(),
        policies: vec![],
        data: IndexMap::new(),
        includes: vec![],
        include_provenance: HashMap::new(),
        checks: IndexMap::new(),
        moved: vec![],
        secrets: Default::default(),
        environments: IndexMap::new(),
    }
}

fn file_resource(path: &str, mode: Option<&str>, owner: Option<&str>) -> Resource {
    let mut r = default_resource();
    r.resource_type = ResourceType::File;
    r.path = Some(path.to_string());
    r.mode = mode.map(|s| s.to_string());
    r.owner = owner.map(|s| s.to_string());
    r
}

fn service_resource(owner: Option<&str>) -> Resource {
    let mut r = default_resource();
    r.resource_type = ResourceType::Service;
    r.name = Some("nginx".to_string());
    r.owner = owner.map(|s| s.to_string());
    r
}

fn package_resource() -> Resource {
    let mut r = default_resource();
    r.resource_type = ResourceType::Package;
    r.packages = vec!["nginx".to_string()];
    r
}

fn docker_resource(image: &str) -> Resource {
    let mut r = default_resource();
    r.resource_type = ResourceType::Docker;
    r.image = Some(image.to_string());
    r.name = Some("web".to_string());
    r.state = Some("running".to_string());
    r
}

fn default_resource() -> Resource {
    Resource {
        resource_type: ResourceType::File,
        machine: MachineTarget::Single("m1".to_string()),
        state: None,
        depends_on: vec![],
        provider: None,
        packages: vec![],
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
        restart_on: vec![],
        triggers: vec![],
        fs_type: None,
        options: None,
        uid: None,
        shell: None,
        home: None,
        groups: vec![],
        ssh_authorized_keys: vec![],
        system_user: false,
        schedule: None,
        command: None,
        image: None,
        ports: vec![],
        environment: vec![],
        volumes: vec![],
        restart: None,
        protocol: None,
        port: None,
        action: None,
        from_addr: None,
        recipe: None,
        inputs: HashMap::new(),
        arch: vec![],
        tags: vec![],
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
        devices: vec![],
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

// ============================================================================
// FJ-1387: Supported Benchmarks
// ============================================================================

#[test]
fn supported_benchmarks_list() {
    let benchmarks = supported_benchmarks();
    assert!(benchmarks.contains(&"cis"));
    assert!(benchmarks.contains(&"nist-800-53"));
    assert!(benchmarks.contains(&"soc2"));
    assert!(benchmarks.contains(&"hipaa"));
    assert_eq!(benchmarks.len(), 4);
}

#[test]
fn unknown_benchmark_returns_info() {
    let config = empty_config();
    let findings = evaluate_benchmark("nonexistent", &config);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, FindingSeverity::Info);
    assert!(findings[0].message.contains("unknown benchmark"));
}

// ============================================================================
// FJ-1387: CIS Benchmark
// ============================================================================

#[test]
fn cis_world_writable_detected() {
    let mut config = empty_config();
    config.resources.insert(
        "bad-file".into(),
        file_resource("/etc/config", Some("0777"), None),
    );
    let findings = evaluate_benchmark("cis", &config);
    assert!(findings.iter().any(|f| f.rule_id == "CIS-6.1.1"));
}

#[test]
fn cis_world_writable_mode_6() {
    let mut config = empty_config();
    config.resources.insert(
        "bad-file".into(),
        file_resource("/etc/config", Some("0666"), None),
    );
    let findings = evaluate_benchmark("cis", &config);
    assert!(findings.iter().any(|f| f.rule_id == "CIS-6.1.1"));
}

#[test]
fn cis_safe_mode_no_finding() {
    let mut config = empty_config();
    config.resources.insert(
        "safe".into(),
        file_resource("/etc/config", Some("0644"), None),
    );
    let findings = evaluate_benchmark("cis", &config);
    assert!(!findings.iter().any(|f| f.rule_id == "CIS-6.1.1"));
}

#[test]
fn cis_root_tmp_detected() {
    let mut config = empty_config();
    config.resources.insert(
        "tmp-file".into(),
        file_resource("/tmp/data", None, Some("root")),
    );
    let findings = evaluate_benchmark("cis", &config);
    assert!(findings.iter().any(|f| f.rule_id == "CIS-1.1.5"));
}

#[test]
fn cis_service_no_restart_policy() {
    let mut config = empty_config();
    config
        .resources
        .insert("svc".into(), service_resource(None));
    let findings = evaluate_benchmark("cis", &config);
    assert!(findings.iter().any(|f| f.rule_id == "CIS-5.2.1"));
}

#[test]
fn cis_package_no_version_pin() {
    let mut config = empty_config();
    config.resources.insert("pkg".into(), package_resource());
    let findings = evaluate_benchmark("cis", &config);
    assert!(findings.iter().any(|f| f.rule_id == "CIS-6.2.1"));
}

#[test]
fn cis_empty_config_no_findings() {
    let config = empty_config();
    let findings = evaluate_benchmark("cis", &config);
    assert!(findings.is_empty());
}

// ============================================================================
// FJ-1387: NIST 800-53
// ============================================================================

#[test]
fn nist_ac3_file_missing_owner() {
    let mut config = empty_config();
    config
        .resources
        .insert("f1".into(), file_resource("/etc/app.conf", None, None));
    let findings = evaluate_benchmark("nist", &config);
    assert!(findings.iter().any(|f| f.rule_id == "NIST-AC-3.1"));
}

#[test]
fn nist_ac3_file_missing_mode() {
    let mut config = empty_config();
    config.resources.insert(
        "f1".into(),
        file_resource("/etc/app.conf", None, Some("root")),
    );
    let findings = evaluate_benchmark("nist-800-53", &config);
    assert!(findings.iter().any(|f| f.rule_id == "NIST-AC-3.2"));
}

#[test]
fn nist_ac6_service_as_root() {
    let mut config = empty_config();
    config
        .resources
        .insert("svc".into(), service_resource(Some("root")));
    let findings = evaluate_benchmark("nist", &config);
    assert!(findings.iter().any(|f| f.rule_id == "NIST-AC-6"));
}

#[test]
fn nist_sc28_sensitive_path_no_mode() {
    let mut config = empty_config();
    config.resources.insert(
        "ssh".into(),
        file_resource("/etc/ssh/sshd_config", None, None),
    );
    let findings = evaluate_benchmark("nist", &config);
    assert!(findings.iter().any(|f| f.rule_id == "NIST-SC-28"));
}

#[test]
fn nist_cm6_docker_no_ports() {
    let mut config = empty_config();
    let mut d = docker_resource("nginx");
    d.ports = vec![];
    d.port = None;
    config.resources.insert("docker".into(), d);
    let findings = evaluate_benchmark("nist", &config);
    assert!(findings.iter().any(|f| f.rule_id == "NIST-CM-6"));
}

// ============================================================================
// FJ-1387: SOC2
// ============================================================================

#[test]
fn soc2_file_missing_owner() {
    let mut config = empty_config();
    config
        .resources
        .insert("f1".into(), file_resource("/etc/app.conf", None, None));
    let findings = evaluate_benchmark("soc2", &config);
    assert!(findings.iter().any(|f| f.rule_id == "SOC2-CC6.1"));
}

#[test]
fn soc2_service_no_restart_on() {
    let mut config = empty_config();
    config
        .resources
        .insert("svc".into(), service_resource(None));
    let findings = evaluate_benchmark("soc2", &config);
    assert!(findings.iter().any(|f| f.rule_id == "SOC2-CC7.2"));
}

// ============================================================================
// FJ-1387: HIPAA
// ============================================================================

#[test]
fn hipaa_other_access_detected() {
    let mut config = empty_config();
    config.resources.insert(
        "f1".into(),
        file_resource("/data/health", Some("0644"), None),
    );
    let findings = evaluate_benchmark("hipaa", &config);
    assert!(findings.iter().any(|f| f.rule_id == "HIPAA-164.312a"));
}

#[test]
fn hipaa_safe_mode_no_finding() {
    let mut config = empty_config();
    config.resources.insert(
        "f1".into(),
        file_resource("/data/health", Some("0640"), None),
    );
    let findings = evaluate_benchmark("hipaa", &config);
    assert!(!findings.iter().any(|f| f.rule_id == "HIPAA-164.312a"));
}

#[test]
fn hipaa_unencrypted_port() {
    let mut config = empty_config();
    let mut r = default_resource();
    r.resource_type = ResourceType::Network;
    r.port = Some("80".into());
    config.resources.insert("http".into(), r);
    let findings = evaluate_benchmark("hipaa", &config);
    assert!(findings.iter().any(|f| f.rule_id == "HIPAA-164.312e"));
}

#[test]
fn hipaa_encrypted_port_no_finding() {
    let mut config = empty_config();
    let mut r = default_resource();
    r.resource_type = ResourceType::Network;
    r.port = Some("443".into());
    config.resources.insert("https".into(), r);
    let findings = evaluate_benchmark("hipaa", &config);
    assert!(!findings.iter().any(|f| f.rule_id == "HIPAA-164.312e"));
}

// ============================================================================
// FJ-1387: Severity Counting
// ============================================================================

#[test]
fn count_by_severity_mixed() {
    let findings = vec![
        forjar::core::compliance::ComplianceFinding {
            rule_id: "R1".into(),
            benchmark: "cis".into(),
            severity: FindingSeverity::Critical,
            resource_id: "r1".into(),
            message: "critical".into(),
        },
        forjar::core::compliance::ComplianceFinding {
            rule_id: "R2".into(),
            benchmark: "cis".into(),
            severity: FindingSeverity::High,
            resource_id: "r2".into(),
            message: "high".into(),
        },
        forjar::core::compliance::ComplianceFinding {
            rule_id: "R3".into(),
            benchmark: "cis".into(),
            severity: FindingSeverity::Medium,
            resource_id: "r3".into(),
            message: "medium".into(),
        },
        forjar::core::compliance::ComplianceFinding {
            rule_id: "R4".into(),
            benchmark: "cis".into(),
            severity: FindingSeverity::Low,
            resource_id: "r4".into(),
            message: "low".into(),
        },
    ];
    let (critical, high, medium, low) = count_by_severity(&findings);
    assert_eq!(critical, 1);
    assert_eq!(high, 1);
    assert_eq!(medium, 1);
    assert_eq!(low, 1);
}

#[test]
fn count_by_severity_empty() {
    let (c, h, m, l) = count_by_severity(&[]);
    assert_eq!((c, h, m, l), (0, 0, 0, 0));
}

// ============================================================================
// FJ-044: Docker → Pepita Conversion
// ============================================================================

#[test]
fn docker_basic_conversion() {
    let docker = docker_resource("nginx:latest");
    let result = docker_to_pepita("web", &docker);
    assert_eq!(result.resource.resource_type, ResourceType::Pepita);
    assert_eq!(result.resource.state.as_deref(), Some("present"));
    assert!(result.resource.image.is_none());
}

#[test]
fn docker_ports_enable_netns() {
    let mut docker = docker_resource("nginx:latest");
    docker.ports = vec!["8080:80".into()];
    let result = docker_to_pepita("web", &docker);
    assert!(result.resource.netns);
    assert!(result.resource.ports.is_empty());
    assert!(result.warnings.iter().any(|w| w.contains("iptables")));
}

#[test]
fn docker_absent_state() {
    let mut docker = docker_resource("nginx:latest");
    docker.state = Some("absent".into());
    let result = docker_to_pepita("old", &docker);
    assert_eq!(result.resource.state.as_deref(), Some("absent"));
}

#[test]
fn docker_stopped_maps_to_absent() {
    let mut docker = docker_resource("app:v1");
    docker.state = Some("stopped".into());
    let result = docker_to_pepita("app", &docker);
    assert_eq!(result.resource.state.as_deref(), Some("absent"));
    assert!(result.warnings.iter().any(|w| w.contains("stopped")));
}

#[test]
fn docker_default_state_maps_to_present() {
    let mut docker = docker_resource("nginx:latest");
    docker.state = None;
    let result = docker_to_pepita("web", &docker);
    assert_eq!(result.resource.state.as_deref(), Some("present"));
}

#[test]
fn docker_unknown_state_warning() {
    let mut docker = docker_resource("app:v1");
    docker.state = Some("restarting".into());
    let result = docker_to_pepita("app", &docker);
    assert_eq!(result.resource.state.as_deref(), Some("present"));
    assert!(result.warnings.iter().any(|w| w.contains("restarting")));
}

#[test]
fn docker_image_warning() {
    let docker = docker_resource("nginx:latest");
    let result = docker_to_pepita("web", &docker);
    assert!(result.warnings.iter().any(|w| w.contains("nginx:latest")));
    assert!(result.warnings.iter().any(|w| w.contains("overlay_lower")));
}

#[test]
fn docker_volumes_warning() {
    let mut docker = docker_resource("postgres:16");
    docker.volumes = vec!["/data:/var/lib/postgresql".into()];
    let result = docker_to_pepita("db", &docker);
    assert!(result.resource.volumes.is_empty());
    assert!(result.warnings.iter().any(|w| w.contains("volumes")));
}

#[test]
fn docker_environment_warning() {
    let mut docker = docker_resource("app:v1");
    docker.environment = vec!["DB_HOST=localhost".into()];
    let result = docker_to_pepita("app", &docker);
    assert!(result.resource.environment.is_empty());
    assert!(result.warnings.iter().any(|w| w.contains("environment")));
}

#[test]
fn docker_restart_warning() {
    let mut docker = docker_resource("nginx:latest");
    docker.restart = Some("unless-stopped".into());
    let result = docker_to_pepita("web", &docker);
    assert!(result.resource.restart.is_none());
    assert!(result.warnings.iter().any(|w| w.contains("restart")));
}

#[test]
fn docker_preserves_depends_on() {
    let mut docker = docker_resource("app:v1");
    docker.depends_on = vec!["db".into()];
    let result = docker_to_pepita("app", &docker);
    assert_eq!(result.resource.depends_on, vec!["db"]);
}

#[test]
fn docker_preserves_tags() {
    let mut docker = docker_resource("nginx:latest");
    docker.tags = vec!["web".into(), "critical".into()];
    let result = docker_to_pepita("web", &docker);
    assert_eq!(result.resource.tags, vec!["web", "critical"]);
}

// ============================================================================
// FJ-044: Full Config Migration
// ============================================================================

#[test]
fn migrate_config_converts_docker() {
    let mut config = empty_config();
    config
        .resources
        .insert("web".into(), docker_resource("nginx:latest"));
    let mut pkg = package_resource();
    pkg.resource_type = ResourceType::Package;
    config.resources.insert("tools".into(), pkg);

    let (migrated, warnings) = migrate_config(&config);
    assert_eq!(
        migrated.resources["web"].resource_type,
        ResourceType::Pepita
    );
    assert_eq!(
        migrated.resources["tools"].resource_type,
        ResourceType::Package
    );
    assert!(!warnings.is_empty());
}

#[test]
fn migrate_config_no_docker_no_warnings() {
    let mut config = empty_config();
    config.resources.insert("pkg".into(), package_resource());

    let (migrated, warnings) = migrate_config(&config);
    assert!(warnings.is_empty());
    assert_eq!(
        migrated.resources["pkg"].resource_type,
        ResourceType::Package
    );
}

#[test]
fn migrate_config_empty() {
    let config = empty_config();
    let (migrated, warnings) = migrate_config(&config);
    assert!(migrated.resources.is_empty());
    assert!(warnings.is_empty());
}
