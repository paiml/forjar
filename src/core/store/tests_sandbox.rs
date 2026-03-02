//! Tests for FJ-1315: Build sandbox configuration.

use super::sandbox::{
    blocks_network, cgroup_path, enforces_fs_isolation, parse_sandbox_config, preset_profile,
    validate_config, BindMount, EnvVar, SandboxConfig, SandboxLevel,
};

#[test]
fn test_fj1315_preset_full() {
    let cfg = preset_profile("full").unwrap();
    assert_eq!(cfg.level, SandboxLevel::Full);
    assert_eq!(cfg.memory_mb, 2048);
    assert_eq!(cfg.cpus, 4.0);
    assert_eq!(cfg.timeout, 600);
    assert!(cfg.bind_mounts.is_empty());
    assert!(cfg.env.is_empty());
}

#[test]
fn test_fj1315_preset_network_only() {
    let cfg = preset_profile("network-only").unwrap();
    assert_eq!(cfg.level, SandboxLevel::NetworkOnly);
    assert_eq!(cfg.memory_mb, 4096);
    assert_eq!(cfg.cpus, 8.0);
}

#[test]
fn test_fj1315_preset_minimal() {
    let cfg = preset_profile("minimal").unwrap();
    assert_eq!(cfg.level, SandboxLevel::Minimal);
    assert_eq!(cfg.memory_mb, 1024);
    assert_eq!(cfg.cpus, 2.0);
}

#[test]
fn test_fj1315_preset_gpu() {
    let cfg = preset_profile("gpu").unwrap();
    assert_eq!(cfg.level, SandboxLevel::NetworkOnly);
    assert_eq!(cfg.memory_mb, 16384);
    assert_eq!(cfg.bind_mounts.len(), 1);
    assert_eq!(cfg.bind_mounts[0].source, "/dev/nvidia0");
    assert!(!cfg.bind_mounts[0].readonly);
    assert_eq!(cfg.env.len(), 1);
    assert_eq!(cfg.env[0].name, "NVIDIA_VISIBLE_DEVICES");
}

#[test]
fn test_fj1315_preset_unknown() {
    assert!(preset_profile("nonexistent").is_none());
}

#[test]
fn test_fj1315_validate_valid() {
    let cfg = preset_profile("full").unwrap();
    let errors = validate_config(&cfg);
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
}

#[test]
fn test_fj1315_validate_zero_memory() {
    let mut cfg = preset_profile("full").unwrap();
    cfg.memory_mb = 0;
    let errors = validate_config(&cfg);
    assert!(errors.iter().any(|e| e.contains("memory_mb")));
}

#[test]
fn test_fj1315_validate_zero_cpus() {
    let mut cfg = preset_profile("full").unwrap();
    cfg.cpus = 0.0;
    let errors = validate_config(&cfg);
    assert!(errors.iter().any(|e| e.contains("cpus")));
}

#[test]
fn test_fj1315_validate_zero_timeout() {
    let mut cfg = preset_profile("full").unwrap();
    cfg.timeout = 0;
    let errors = validate_config(&cfg);
    assert!(errors.iter().any(|e| e.contains("timeout")));
}

#[test]
fn test_fj1315_validate_excessive_memory() {
    let mut cfg = preset_profile("full").unwrap();
    cfg.memory_mb = 2_000_000;
    let errors = validate_config(&cfg);
    assert!(errors.iter().any(|e| e.contains("1 TiB")));
}

#[test]
fn test_fj1315_validate_excessive_cpus() {
    let mut cfg = preset_profile("full").unwrap();
    cfg.cpus = 9999.0;
    let errors = validate_config(&cfg);
    assert!(errors.iter().any(|e| e.contains("1024")));
}

#[test]
fn test_fj1315_validate_empty_bind_mount() {
    let mut cfg = preset_profile("full").unwrap();
    cfg.bind_mounts.push(BindMount {
        source: String::new(),
        target: "/mnt".to_string(),
        readonly: true,
    });
    let errors = validate_config(&cfg);
    assert!(errors.iter().any(|e| e.contains("source")));
}

#[test]
fn test_fj1315_parse_yaml() {
    let yaml = r#"
level: full
memory_mb: 4096
cpus: 8.0
timeout: 900
"#;
    let cfg = parse_sandbox_config(yaml).unwrap();
    assert_eq!(cfg.level, SandboxLevel::Full);
    assert_eq!(cfg.memory_mb, 4096);
    assert_eq!(cfg.cpus, 8.0);
    assert_eq!(cfg.timeout, 900);
}

#[test]
fn test_fj1315_parse_yaml_with_mounts() {
    let yaml = r#"
level: network-only
memory_mb: 2048
cpus: 4.0
timeout: 600
bind_mounts:
  - source: /data/models
    target: /inputs/models
    readonly: true
env:
  - name: CUDA_HOME
    value: /usr/local/cuda
"#;
    let cfg = parse_sandbox_config(yaml).unwrap();
    assert_eq!(cfg.bind_mounts.len(), 1);
    assert!(cfg.bind_mounts[0].readonly);
    assert_eq!(cfg.env[0].name, "CUDA_HOME");
}

#[test]
fn test_fj1315_parse_yaml_invalid() {
    assert!(parse_sandbox_config("not: [valid: yaml: config").is_err());
}

#[test]
fn test_fj1315_parse_yaml_defaults() {
    let yaml = "level: minimal\n";
    let cfg = parse_sandbox_config(yaml).unwrap();
    assert_eq!(cfg.memory_mb, 2048);
    assert_eq!(cfg.cpus, 4.0);
    assert_eq!(cfg.timeout, 600);
}

#[test]
fn test_fj1315_serde_roundtrip() {
    let cfg = SandboxConfig {
        level: SandboxLevel::Full,
        memory_mb: 8192,
        cpus: 16.0,
        timeout: 1800,
        bind_mounts: vec![BindMount {
            source: "/src".to_string(),
            target: "/build/src".to_string(),
            readonly: true,
        }],
        env: vec![EnvVar {
            name: "CC".to_string(),
            value: "gcc".to_string(),
        }],
    };
    let yaml = serde_yaml_ng::to_string(&cfg).unwrap();
    let parsed: SandboxConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(cfg, parsed);
}

#[test]
fn test_fj1315_blocks_network() {
    assert!(blocks_network(SandboxLevel::Full));
    assert!(!blocks_network(SandboxLevel::NetworkOnly));
    assert!(!blocks_network(SandboxLevel::Minimal));
    assert!(!blocks_network(SandboxLevel::None));
}

#[test]
fn test_fj1315_enforces_fs_isolation() {
    assert!(enforces_fs_isolation(SandboxLevel::Full));
    assert!(enforces_fs_isolation(SandboxLevel::NetworkOnly));
    assert!(enforces_fs_isolation(SandboxLevel::Minimal));
    assert!(!enforces_fs_isolation(SandboxLevel::None));
}

#[test]
fn test_fj1315_cgroup_path() {
    let path = cgroup_path("blake3:a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4");
    assert!(path.starts_with("/sys/fs/cgroup/forjar-build-"));
    assert!(path.contains("a1b2c3d4e5f6a1b2"));
}

#[test]
fn test_fj1315_cgroup_path_no_prefix() {
    let path = cgroup_path("abcdef1234567890");
    assert!(path.contains("abcdef1234567890"));
}

#[test]
fn test_fj1315_sandbox_level_json_roundtrip() {
    let cfg = preset_profile("full").unwrap();
    let json = serde_json::to_string(&cfg).unwrap();
    let parsed: SandboxConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(cfg.level, parsed.level);
}
