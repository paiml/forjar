use super::package::*;
use crate::core::types::{MachineTarget, Resource, ResourceType};

pub(super) fn make_apt_resource(packages: &[&str]) -> Resource {
    Resource {
        resource_type: ResourceType::Package,
        machine: MachineTarget::Single("m1".to_string()),
        state: None,
        depends_on: vec![],
        provider: Some("apt".to_string()),
        packages: packages.iter().map(|s| s.to_string()).collect(),
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
        inputs: std::collections::HashMap::new(),
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

#[test]
fn test_fj006_check_apt() {
    let r = make_apt_resource(&["curl", "wget"]);
    let script = check_script(&r);
    assert!(script.contains("dpkg -l 'curl'"));
    assert!(script.contains("dpkg -l 'wget'"));
}

#[test]
fn test_fj006_apply_apt_present() {
    let r = make_apt_resource(&["curl"]);
    let script = apply_script(&r);
    assert!(script.contains("apt-get install"));
    assert!(script.contains("'curl'"));
    assert!(script.contains("set -euo pipefail"));
    assert!(script.contains("DEBIAN_FRONTEND=noninteractive"));
    assert!(script.contains("sudo apt-get"));
    assert!(script.contains("apt-get install"));
}

#[test]
fn test_fj006_apply_apt_absent() {
    let mut r = make_apt_resource(&["curl"]);
    r.state = Some("absent".to_string());
    let script = apply_script(&r);
    assert!(script.contains("apt-get remove"));
    assert!(script.contains("sudo apt-get"));
}

#[test]
fn test_fj006_apply_apt_sudo_detection() {
    let r = make_apt_resource(&["curl"]);
    let script = apply_script(&r);
    assert!(script.contains("id -u"));
    assert!(script.contains("sudo apt-get"));
}

#[test]
fn test_fj006_cargo_check() {
    let mut r = make_apt_resource(&["batuta"]);
    r.provider = Some("cargo".to_string());
    let script = check_script(&r);
    assert!(script.contains("command -v 'batuta'"));
}

#[test]
fn test_fj006_cargo_install() {
    let mut r = make_apt_resource(&["batuta"]);
    r.provider = Some("cargo".to_string());
    let script = apply_script(&r);
    // FJ-51: crate installs use --root staging for cache integration
    assert!(script.contains("cargo install --force --locked --root"));
    assert!(script.contains("'batuta'"));
}

/// PMAT-007: cargo install must use --force for idempotent convergence.
/// Without --force, `cargo install` fails when a binary already exists
/// (e.g. from a previous symlink or workspace build).
#[test]
fn test_fj007_cargo_install_uses_force() {
    let mut r = make_apt_resource(&["trueno-rag-cli"]);
    r.provider = Some("cargo".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("cargo install --force"),
        "cargo install must use --force for idempotent installs, got: {script}"
    );
}

#[test]
fn test_fj006_state_query_apt() {
    let r = make_apt_resource(&["curl"]);
    let script = state_query_script(&r);
    assert!(script.contains("dpkg-query"));
}

#[test]
fn test_fj006_quoted_packages() {
    // Verify all package names are single-quoted (injection prevention)
    let r = make_apt_resource(&["curl", "lib; rm -rf /"]);
    let script = apply_script(&r);
    assert!(script.contains("'lib; rm -rf /'"));
    // The semicolon is inside quotes — safe
}

#[test]
fn test_fj006_state_query_cargo() {
    let mut r = make_apt_resource(&["batuta", "renacer"]);
    r.provider = Some("cargo".to_string());
    let script = state_query_script(&r);
    assert!(script.contains("command -v 'batuta'"));
    assert!(script.contains("command -v 'renacer'"));
    assert!(script.contains("installed"));
}

#[test]
fn test_fj006_state_query_unknown_provider() {
    let mut r = make_apt_resource(&["tool"]);
    r.provider = Some("snap".to_string());
    let script = state_query_script(&r);
    assert!(script.contains("unsupported provider: snap"));
}

#[test]
fn test_fj006_check_unsupported_provider() {
    let mut r = make_apt_resource(&["foo"]);
    r.provider = Some("snap".to_string());
    let script = check_script(&r);
    assert!(script.contains("unsupported provider"));
}

#[test]
fn test_fj006_apply_unsupported_combo() {
    let mut r = make_apt_resource(&["foo"]);
    r.provider = Some("snap".to_string());
    r.state = Some("present".to_string());
    let script = apply_script(&r);
    assert!(script.contains("unsupported"));
}

#[test]
fn test_fj006_uv_check() {
    let mut r = make_apt_resource(&["ruff", "mypy"]);
    r.provider = Some("uv".to_string());
    let script = check_script(&r);
    assert!(script.contains("uv tool list"));
    assert!(script.contains("grep -q '^ruff'"));
    assert!(script.contains("echo 'installed:ruff'"));
    assert!(script.contains("echo 'missing:mypy'"));
}

#[test]
fn test_fj006_uv_install() {
    let mut r = make_apt_resource(&["ruff"]);
    r.provider = Some("uv".to_string());
    let script = apply_script(&r);
    assert!(script.contains("set -euo pipefail"));
    assert!(script.contains("uv tool install --force 'ruff'"));
}

#[test]
fn test_fj006_uv_absent() {
    let mut r = make_apt_resource(&["ruff"]);
    r.provider = Some("uv".to_string());
    r.state = Some("absent".to_string());
    let script = apply_script(&r);
    assert!(script.contains("uv tool uninstall 'ruff'"));
}

#[test]
fn test_fj006_uv_state_query() {
    let mut r = make_apt_resource(&["ruff", "mypy"]);
    r.provider = Some("uv".to_string());
    let script = state_query_script(&r);
    assert!(script.contains("uv tool list"));
    assert!(script.contains("echo 'ruff=installed'"));
    assert!(script.contains("echo 'mypy=MISSING'"));
}

#[test]
fn test_fj006_uv_install_multi() {
    let mut r = make_apt_resource(&["ruff", "mypy", "black"]);
    r.provider = Some("uv".to_string());
    let script = apply_script(&r);
    assert!(script.contains("uv tool install --force 'ruff'"));
    assert!(script.contains("uv tool install --force 'mypy'"));
    assert!(script.contains("uv tool install --force 'black'"));
}

/// Verify single-quoting prevents injection in uv provider.
#[test]
fn test_fj006_uv_quoted_packages() {
    let mut r = make_apt_resource(&["ruff; rm -rf /"]);
    r.provider = Some("uv".to_string());
    let script = apply_script(&r);
    assert!(script.contains("'ruff; rm -rf /'"));
}

/// BH-MUT-0001: Kill mutation of cargo check_script boolean logic.
/// Verify installed/missing output format to catch && / || flip.
#[test]
fn test_fj006_cargo_check_output_format() {
    let mut r = make_apt_resource(&["ripgrep"]);
    r.provider = Some("cargo".to_string());
    let script = check_script(&r);
    // Must contain both installed AND missing branches — flipping && to || would break
    assert!(script.contains("echo 'installed:ripgrep'"));
    assert!(script.contains("echo 'missing:ripgrep'"));
}

/// BH-MUT-0001: Kill mutation of apt check_script boolean logic.
#[test]
fn test_fj006_apt_check_output_format() {
    let r = make_apt_resource(&["curl"]);
    let script = check_script(&r);
    assert!(script.contains("echo 'installed:curl'"));
    assert!(script.contains("echo 'missing:curl'"));
}

// --- FJ-1398: Homebrew provider tests ---

#[test]
fn test_fj1398_brew_check() {
    let mut r = make_apt_resource(&["jq", "ripgrep"]);
    r.provider = Some("brew".to_string());
    let script = check_script(&r);
    assert!(script.contains("brew list 'jq'"));
    assert!(script.contains("brew list 'ripgrep'"));
    assert!(script.contains("echo 'installed:jq'"));
    assert!(script.contains("echo 'missing:ripgrep'"));
}

#[test]
fn test_fj1398_brew_install() {
    let mut r = make_apt_resource(&["jq"]);
    r.provider = Some("brew".to_string());
    let script = apply_script(&r);
    assert!(script.contains("set -euo pipefail"));
    assert!(script.contains("brew install 'jq'"));
    assert!(script.contains("NEED_INSTALL"));
}

#[test]
fn test_fj1398_brew_install_versioned() {
    let mut r = make_apt_resource(&["python"]);
    r.provider = Some("brew".to_string());
    r.version = Some("3.12".to_string());
    let script = apply_script(&r);
    assert!(script.contains("brew install 'python@3.12'"));
}

#[test]
fn test_fj1398_brew_absent() {
    let mut r = make_apt_resource(&["jq"]);
    r.provider = Some("brew".to_string());
    r.state = Some("absent".to_string());
    let script = apply_script(&r);
    assert!(script.contains("brew uninstall 'jq'"));
}

#[test]
fn test_fj1398_brew_state_query() {
    let mut r = make_apt_resource(&["jq", "fd"]);
    r.provider = Some("brew".to_string());
    let script = state_query_script(&r);
    assert!(script.contains("brew list --versions 'jq'"));
    assert!(script.contains("brew list --versions 'fd'"));
    assert!(script.contains("MISSING"));
}

#[test]
fn test_fj1398_brew_check_output_format() {
    let mut r = make_apt_resource(&["ripgrep"]);
    r.provider = Some("brew".to_string());
    let script = check_script(&r);
    assert!(script.contains("echo 'installed:ripgrep'"));
    assert!(script.contains("echo 'missing:ripgrep'"));
}

#[test]
fn test_fj1398_brew_multi_install() {
    let mut r = make_apt_resource(&["jq", "fd", "bat"]);
    r.provider = Some("brew".to_string());
    let script = apply_script(&r);
    assert!(script.contains("brew install 'jq'"));
    assert!(script.contains("brew install 'fd'"));
    assert!(script.contains("brew install 'bat'"));
}

/// Verify single-quoting prevents injection in brew provider.
#[test]
fn test_fj1398_brew_quoted_packages() {
    let mut r = make_apt_resource(&["jq; rm -rf /"]);
    r.provider = Some("brew".to_string());
    let script = apply_script(&r);
    assert!(script.contains("'jq; rm -rf /'"));
}
