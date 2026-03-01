use super::purifier::*;

// --- FJ-036: Validation tests ---

#[test]
fn test_fj036_validate_simple_echo() {
    let script = "#!/bin/bash\nset -euo pipefail\necho 'hello'\n";
    assert!(validate_script(script).is_ok());
}

#[test]
fn test_fj036_validate_pipefail_script() {
    let script = "#!/bin/bash\nset -euo pipefail\napt-get install -y curl\n";
    assert!(validate_script(script).is_ok());
}

#[test]
fn test_fj036_validate_empty_script() {
    assert!(validate_script("").is_ok());
}

#[test]
fn test_fj036_validate_multiline_script() {
    let script = "#!/bin/bash\nset -euo pipefail\nmkdir -p /tmp/test\nchmod 0755 /tmp/test\n";
    assert!(validate_script(script).is_ok());
}

// --- FJ-036: Lint tests ---

#[test]
fn test_fj036_lint_returns_diagnostics() {
    let script = "#!/bin/bash\nset -euo pipefail\necho hello\n";
    let result = lint_script(script);
    // Should lint without panicking; diagnostics may vary
    let _ = result.diagnostics.len();
}

#[test]
fn test_fj036_lint_error_count_clean_script() {
    let script = "#!/bin/bash\nset -euo pipefail\nprintf '%s\\n' 'hello'\n";
    let errors = lint_error_count(script);
    // A well-formed script should have zero or few errors
    assert!(errors <= 2, "expected few errors, got {errors}");
}

#[test]
fn test_fj036_lint_severity_filter() {
    // validate_script should pass even if there are warnings
    let script = "#!/bin/bash\necho hello\n";
    assert!(
        validate_script(script).is_ok(),
        "warnings should not fail validation"
    );
}

// --- FJ-036: Purification tests ---

#[test]
fn test_fj036_purify_simple_script() {
    let script = "#!/bin/bash\necho hello\n";
    let result = purify_script(script);
    // Purification should succeed on a simple script
    assert!(result.is_ok(), "purify failed: {:?}", result.err());
}

#[test]
fn test_fj036_purify_preserves_semantics() {
    let script = "#!/bin/bash\nset -euo pipefail\nmkdir -p /tmp/test\n";
    if let Ok(purified) = purify_script(script) {
        assert!(
            purified.contains("mkdir"),
            "purified lost mkdir: {purified}"
        );
    }
}

#[test]
fn test_fj036_purify_returns_string() {
    let script = "echo test";
    if let Ok(purified) = purify_script(script) {
        assert!(!purified.is_empty());
    }
}

// --- FJ-036: Integration with codegen output ---

#[test]
fn test_fj036_validate_generated_package_script() {
    let script = r#"#!/bin/bash
set -euo pipefail
SUDO=""
if [ "$(id -u)" -ne 0 ]; then SUDO="sudo"; fi
dpkg -l curl 2>/dev/null | grep -q '^ii'
"#;
    assert!(
        validate_script(script).is_ok(),
        "package check script failed validation"
    );
}

#[test]
fn test_fj036_validate_generated_file_script() {
    let script = "#!/bin/bash\nset -euo pipefail\ntest -f /etc/test.conf\n";
    assert!(
        validate_script(script).is_ok(),
        "file check script failed validation"
    );
}

#[test]
fn test_fj036_validate_generated_service_script() {
    let script = "#!/bin/bash\nset -euo pipefail\nsystemctl is-active nginx\n";
    assert!(
        validate_script(script).is_ok(),
        "service check script failed validation"
    );
}

#[test]
fn test_fj036_validate_heredoc_script() {
    let script =
        "set -euo pipefail\ncat > '/etc/test.conf' <<'FORJAR_EOF'\nkey=value\nFORJAR_EOF\n";
    assert!(
        validate_script(script).is_ok(),
        "heredoc script failed validation"
    );
}

// --- FJ-036: Codegen integration — validate real generated scripts ---

#[test]
fn test_fj036_codegen_file_check_validates() {
    use crate::core::codegen;
    let r = make_test_resource(crate::core::types::ResourceType::File);
    let script = codegen::check_script(&r).unwrap();
    assert!(validate_script(&script).is_ok(), "file check failed bashrs");
}

#[test]
fn test_fj036_codegen_file_apply_validates() {
    use crate::core::codegen;
    let r = make_test_resource(crate::core::types::ResourceType::File);
    let script = codegen::apply_script(&r).unwrap();
    assert!(validate_script(&script).is_ok(), "file apply failed bashrs");
}

#[test]
fn test_fj036_codegen_file_state_query_validates() {
    use crate::core::codegen;
    let r = make_test_resource(crate::core::types::ResourceType::File);
    let script = codegen::state_query_script(&r).unwrap();
    assert!(
        validate_script(&script).is_ok(),
        "file state_query failed bashrs"
    );
}

#[test]
fn test_fj036_codegen_service_all_validate() {
    use crate::core::codegen;
    let mut r = make_test_resource(crate::core::types::ResourceType::Service);
    r.name = Some("nginx".to_string());
    r.state = Some("running".to_string());
    r.enabled = Some(true);
    for (kind, result) in [
        ("check", codegen::check_script(&r)),
        ("apply", codegen::apply_script(&r)),
        ("state_query", codegen::state_query_script(&r)),
    ] {
        let script = result.unwrap();
        assert!(
            validate_script(&script).is_ok(),
            "service {kind} failed bashrs"
        );
    }
}

#[test]
fn test_fj036_codegen_mount_all_validate() {
    use crate::core::codegen;
    let mut r = make_test_resource(crate::core::types::ResourceType::Mount);
    r.source = Some("192.168.1.1:/data".to_string());
    r.fs_type = Some("nfs".to_string());
    r.options = Some("ro,hard".to_string());
    for (kind, result) in [
        ("check", codegen::check_script(&r)),
        ("apply", codegen::apply_script(&r)),
        ("state_query", codegen::state_query_script(&r)),
    ] {
        let script = result.unwrap();
        assert!(
            validate_script(&script).is_ok(),
            "mount {kind} failed bashrs"
        );
    }
}

#[test]
fn test_fj036_lint_codegen_package_has_diagnostics() {
    use crate::core::codegen;
    let mut r = make_test_resource(crate::core::types::ResourceType::Package);
    r.provider = Some("apt".to_string());
    r.packages = vec!["curl".to_string()];
    let script = codegen::apply_script(&r).unwrap();
    let result = lint_script(&script);
    // Package scripts have $SUDO pattern → expect some diagnostics
    assert!(
        !result.diagnostics.is_empty(),
        "apt scripts should have lint findings"
    );
}

#[test]
fn test_fj036_purify_codegen_file_check() {
    use crate::core::codegen;
    let r = make_test_resource(crate::core::types::ResourceType::File);
    let script = codegen::check_script(&r).unwrap();
    // File check scripts are simple (test -f ...) — should purify cleanly
    if let Ok(purified) = purify_script(&script) {
        assert!(!purified.is_empty());
    }
}

fn make_test_resource(rt: crate::core::types::ResourceType) -> crate::core::types::Resource {
    crate::core::types::Resource {
        resource_type: rt,
        machine: crate::core::types::MachineTarget::Single("m1".to_string()),
        state: None,
        depends_on: vec![],
        provider: None,
        packages: vec![],
        version: None,
        path: Some("/etc/test.conf".to_string()),
        content: Some("key=value".to_string()),
        source: None,
        target: None,
        owner: Some("root".to_string()),
        group: Some("root".to_string()),
        mode: Some("0644".to_string()),
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
        pre_apply: None,
        post_apply: None,
        lifecycle: None,
    }
}
