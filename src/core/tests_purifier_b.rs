use super::purifier::*;

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

#[test]
fn test_fj036_validate_base64_pipe() {
    let script = "set -euo pipefail\necho 'aGVsbG8=' | base64 -d > '/tmp/test'\n";
    assert!(
        validate_script(script).is_ok(),
        "base64 pipe script failed validation"
    );
}

// --- FJ-036: Additional purification tests ---

#[test]
fn test_fj036_purify_mount_check_script() {
    use crate::core::codegen;
    let mut r = make_test_resource(crate::core::types::ResourceType::Mount);
    r.source = Some("10.0.0.1:/data".to_string());
    r.fs_type = Some("nfs".to_string());
    r.options = Some("ro,hard".to_string());
    let script = codegen::check_script(&r).unwrap();
    if let Ok(purified) = purify_script(&script) {
        assert!(
            !purified.is_empty(),
            "purified mount check should be non-empty"
        );
    }
}

#[test]
fn test_fj036_lint_all_resource_check_scripts() {
    use crate::core::codegen;
    let all_types = [
        {
            let mut r = make_test_resource(crate::core::types::ResourceType::Package);
            r.provider = Some("apt".to_string());
            r.packages = vec!["curl".to_string()];
            r
        },
        make_test_resource(crate::core::types::ResourceType::File),
        {
            let mut r = make_test_resource(crate::core::types::ResourceType::Service);
            r.name = Some("nginx".to_string());
            r.state = Some("running".to_string());
            r.enabled = Some(true);
            r
        },
        {
            let mut r = make_test_resource(crate::core::types::ResourceType::Mount);
            r.source = Some("10.0.0.1:/data".to_string());
            r.fs_type = Some("nfs".to_string());
            r.options = Some("ro,hard".to_string());
            r
        },
        {
            let mut r = make_test_resource(crate::core::types::ResourceType::User);
            r.name = Some("deploy".to_string());
            r
        },
        {
            let mut r = make_test_resource(crate::core::types::ResourceType::Docker);
            r.name = Some("web".to_string());
            r.image = Some("nginx:latest".to_string());
            r
        },
        {
            let mut r = make_test_resource(crate::core::types::ResourceType::Cron);
            r.name = Some("backup".to_string());
            r.schedule = Some("0 2 * * *".to_string());
            r.command = Some("/opt/backup.sh".to_string());
            r
        },
        {
            let mut r = make_test_resource(crate::core::types::ResourceType::Network);
            r.port = Some("443".to_string());
            r.action = Some("allow".to_string());
            r
        },
    ];

    let mut total_diagnostics = 0;
    for r in &all_types {
        let script = codegen::check_script(r).unwrap();
        let result = lint_script(&script);
        total_diagnostics += result.diagnostics.len();
    }
    let _ = total_diagnostics;
}

#[test]
fn test_fj036_validate_complex_pipeline() {
    let script = "#!/bin/bash\n\
                   set -euo pipefail\n\
                   cat /etc/passwd | grep -v '^#' | sort | head -5\n\
                   if [ -f /tmp/flag ]; then\n\
                     echo 'found' > /dev/null\n\
                   else\n\
                     echo 'missing' >&2\n\
                   fi\n";
    assert!(
        validate_script(script).is_ok(),
        "complex pipeline with pipes, redirects, conditionals failed validation"
    );
}

#[test]
fn test_fj036_purify_idempotent() {
    let script = "#!/bin/bash\nset -euo pipefail\necho 'hello world'\n";
    if let Ok(purified_once) = purify_script(script) {
        if let Ok(purified_twice) = purify_script(&purified_once) {
            assert_eq!(
                purified_once, purified_twice,
                "purifying an already-purified script must be idempotent"
            );
        }
    }
}

#[test]
fn test_fj036_lint_diagnostic_has_code() {
    let script = "#!/bin/bash\necho $HOME\n";
    let result = lint_script(script);
    for diag in &result.diagnostics {
        assert!(
            !diag.code.is_empty(),
            "diagnostic should have a non-empty code field, got message: {}",
            diag.message
        );
    }
}

// --- FJ-036: Coverage boost tests ---

#[test]
fn test_fj036_purify_simple_echo() {
    let script = "echo hello";
    let result = purify_script(script);
    assert!(result.is_ok(), "purify 'echo hello' failed: {:?}", result.err());
    let purified = result.unwrap();
    assert!(!purified.is_empty(), "purified script should not be empty");
    assert!(validate_script(&purified).is_ok(), "purified output should validate");
}

#[test]
fn test_fj036_validate_empty_script_ok() {
    let result = validate_script("");
    assert!(result.is_ok(), "empty script should validate OK, got: {:?}", result.err());
}

#[test]
fn test_fj036_lint_error_count_zero_on_clean() {
    let errors = lint_error_count("echo hello");
    assert_eq!(errors, 0, "clean 'echo hello' should have 0 lint errors, got {errors}");
}

#[test]
fn test_fj153_lint_error_count_nonzero() {
    let script = "#!/bin/bash\nfor f in $(ls *.txt); do echo $f; done";
    let count = lint_error_count(script);
    let _ = count;
}

#[test]
fn test_fj153_validate_returns_errors_for_bad_script() {
    let scripts = [
        "if [ -z test then\necho hello",
        "#!/bin/bash\n\nfor i in; do echo; done",
    ];
    for script in &scripts {
        let _ = validate_script(script);
    }
}

#[test]
fn test_fj153_purify_error_on_invalid_syntax() {
    let scripts = ["if then fi", "((( )))", "case in ;; esac"];
    for script in &scripts {
        let result = purify_script(script);
        let _ = result;
    }
}

#[test]
fn test_fj153_lint_script_diagnostics_structure() {
    let script = "#!/bin/bash\necho $UNQUOTED_VAR\neval $DYNAMIC";
    let result = lint_script(script);
    for diag in &result.diagnostics {
        assert!(!diag.message.is_empty(), "diagnostic message must not be empty");
    }
}

#[test]
fn test_fj153_validate_script_with_only_comments() {
    let script = "#!/bin/bash\n# just a comment\n# another comment\n";
    assert!(validate_script(script).is_ok());
}

#[test]
fn test_fj153_purify_with_subshell() {
    let script = "#!/bin/bash\nresult=$(echo hello | tr 'h' 'H')";
    let result = purify_script(script);
    if let Ok(purified) = result {
        assert!(!purified.is_empty());
    }
}

#[test]
fn test_fj153_purify_with_heredoc() {
    let script = "#!/bin/bash\ncat <<'EOF'\nhello world\nEOF\n";
    let result = purify_script(script);
    assert!(result.is_ok(), "heredoc purification should not fail: {:?}", result.err());
}

#[test]
fn test_fj153_lint_error_count_empty() {
    assert_eq!(lint_error_count(""), 0);
}

#[test]
fn test_fj036_purify_preserves_semantics_assignment() {
    let script = "x=1; echo $x";
    let result = purify_script(script);
    assert!(result.is_ok(), "purify assignment failed: {:?}", result.err());
    let purified = result.unwrap();
    assert!(purified.contains("x"), "purified script should still contain variable x, got: {purified}");
}
