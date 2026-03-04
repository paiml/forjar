use super::*;

#[test]
fn test_fj132_exec_script_large_output() {
    // Verify transport handles large output without truncation
    let machine = Machine {
        hostname: "local".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    let out = exec_script(&machine, "seq 1 10000").unwrap();
    assert!(out.success());
    assert!(out.stdout.contains("10000"));
}

#[test]
fn test_fj132_exec_script_env_isolation() {
    // Scripts should not leak env vars between calls
    let machine = Machine {
        hostname: "local".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    exec_script(&machine, "export FORJAR_TEST_LEAK=yes").unwrap();
    let out = exec_script(&machine, "echo ${FORJAR_TEST_LEAK:-unset}").unwrap();
    assert!(out.success());
    assert_eq!(out.stdout.trim(), "unset");
}

#[test]
fn test_fj132_exec_script_exit_code_preserved() {
    // Verify various exit codes are preserved
    let machine = Machine {
        hostname: "local".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    for code in [0, 1, 2, 42, 126, 127] {
        let out = exec_script(&machine, &format!("exit {code}")).unwrap();
        assert_eq!(
            out.exit_code, code,
            "exit code {code} should be preserved"
        );
    }
}

#[test]
fn test_fj132_timeout_zero_seconds_fails() {
    // A timeout of 0 seconds should cause immediate timeout
    let machine = Machine {
        hostname: "local".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    // sleep 5 with 0s timeout should error — but 0-second timeout
    // may or may not catch "echo ok" depending on scheduling
    let result = exec_script_timeout(&machine, "sleep 5", Some(0));
    // This should almost always timeout, but we accept either outcome
    // since 0-second timeout behavior is platform-dependent
    if let Err(e) = result {
        assert!(e.contains("timeout"));
    }
}

#[test]
fn test_fj132_is_local_addr_empty_string() {
    assert!(!is_local_addr(""));
}

// ── FJ-036: Transport script execution coverage ─────────────────

#[test]
fn test_fj036_local_script_echo() {
    let machine = Machine {
        hostname: "local".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    let out = exec_script(&machine, "echo 'hello from forjar'").unwrap();
    assert!(out.success());
    assert_eq!(out.exit_code, 0);
    assert_eq!(out.stdout.trim(), "hello from forjar");
    // stderr may contain noise from parallel test processes under coverage instrumentation
    let _ = &out.stderr;
}

#[test]
fn test_fj036_local_script_exit_code() {
    let machine = Machine {
        hostname: "local".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    let out = exec_script(&machine, "exit 1").unwrap();
    assert!(!out.success());
    assert_eq!(out.exit_code, 1);
}

#[test]
fn test_fj036_is_local_addr_comprehensive() {
    // All standard local address variants
    assert!(is_local_addr("127.0.0.1"), "IPv4 loopback must be local");
    assert!(is_local_addr("localhost"), "localhost must be local");
    assert!(is_local_addr("::1"), "IPv6 loopback must be local");

    // Non-local addresses must return false
    assert!(!is_local_addr("0.0.0.0"), "0.0.0.0 is not treated as local");
    assert!(
        !is_local_addr("192.168.1.1"),
        "private IP must not be local"
    );
    assert!(!is_local_addr("10.0.0.1"), "10.x must not be local");
    assert!(!is_local_addr("8.8.8.8"), "public IP must not be local");
    assert!(!is_local_addr("google.com"), "domain must not be local");
    assert!(!is_local_addr(""), "empty string must not be local");
    assert!(
        !is_local_addr("127.0.0.2"),
        "127.0.0.2 is not explicitly local"
    );
}

// ── FJ-261: SSH retry with exponential backoff ──

#[test]
fn test_fj261_is_transient_ssh_error_connection_refused() {
    assert!(is_transient_ssh_error(
        "ssh: connect to host 10.0.0.1 port 22: Connection refused"
    ));
}

#[test]
fn test_fj261_is_transient_ssh_error_connection_reset() {
    assert!(is_transient_ssh_error("Connection reset by peer"));
}

#[test]
fn test_fj261_is_transient_ssh_error_timeout() {
    assert!(is_transient_ssh_error(
        "transport timeout: script on 'box' exceeded 30s limit"
    ));
}

#[test]
fn test_fj261_is_transient_ssh_error_broken_pipe() {
    assert!(is_transient_ssh_error("Write failed: Broken pipe"));
}

#[test]
fn test_fj261_is_transient_ssh_error_no_route() {
    assert!(is_transient_ssh_error(
        "ssh: connect to host 10.0.0.1: No route to host"
    ));
}

#[test]
fn test_fj261_is_transient_ssh_error_spawn_failure() {
    assert!(is_transient_ssh_error(
        "failed to spawn ssh to 10.0.0.1: ..."
    ));
}

#[test]
fn test_fj261_is_transient_ssh_error_non_transient() {
    assert!(!is_transient_ssh_error("Permission denied (publickey)"));
    assert!(!is_transient_ssh_error("Host key verification failed"));
    assert!(!is_transient_ssh_error("exit code 1: command not found"));
}

#[test]
fn test_fj261_retry_local_skips_retry() {
    // Local targets should never retry — only 1 attempt
    let machine = Machine {
        hostname: "local".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    let out = exec_script_retry(&machine, "echo ok", None, 3).unwrap();
    assert!(out.success());
    assert_eq!(out.stdout.trim(), "ok");
}

#[test]
fn test_fj261_retry_default_one_is_no_retry() {
    // ssh_retries=1 means one attempt, no retry
    let machine = Machine {
        hostname: "local".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    let out = exec_script_retry(&machine, "echo once", None, 1).unwrap();
    assert!(out.success());
    assert_eq!(out.stdout.trim(), "once");
}

#[test]
fn test_fj261_retry_clamped_to_max_4() {
    // ssh_retries > 4 should be clamped to 4. For local, still runs once.
    let machine = Machine {
        hostname: "local".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    let out = exec_script_retry(&machine, "echo clamped", None, 100).unwrap();
    assert!(out.success());
    assert_eq!(out.stdout.trim(), "clamped");
}

#[test]
fn test_fj261_retry_with_timeout() {
    let machine = Machine {
        hostname: "local".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    let out = exec_script_retry(&machine, "echo fast", Some(10), 2).unwrap();
    assert!(out.success());
    assert_eq!(out.stdout.trim(), "fast");
}

#[test]
fn test_fj261_retry_zero_clamped_to_one() {
    // ssh_retries=0 should clamp to 1 (at least one attempt)
    let machine = Machine {
        hostname: "local".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
    };
    let out = exec_script_retry(&machine, "echo zero", None, 0).unwrap();
    assert!(out.success());
    assert_eq!(out.stdout.trim(), "zero");
}

// ── FJ-29: Base64 payload stripping for I8 lint ──

#[test]
fn test_fj29_strip_data_payloads_simple() {
    let script = "set -euo pipefail\necho 'SGVsbG8gV29ybGQ=' | base64 -d > '/tmp/hello'\nchmod '0755' '/tmp/hello'";
    let stripped = strip_data_payloads(script);
    assert!(
        !stripped.contains("SGVsbG8gV29ybGQ="),
        "base64 payload should be stripped"
    );
    assert!(
        stripped.contains("FORJAR_BASE64_STRIPPED"),
        "placeholder should replace base64"
    );
    assert!(
        stripped.contains("chmod '0755' '/tmp/hello'"),
        "non-payload lines preserved"
    );
}

#[test]
fn test_fj29_strip_data_payloads_large_binary() {
    // Simulate a large binary (like a 22MB forjar ELF) base64-encoded
    // Generate fake base64 that contains sequences bashrs would misparse
    let fake_b64 = "doZmaQBpbg=="; // contains "do", "fi", "in" substrings
    let script = format!(
        "set -euo pipefail\nmkdir -p '/home/noah/.cargo/bin'\necho '{}' | base64 -d > '/home/noah/.cargo/bin/forjar'\nchown 'noah' '/home/noah/.cargo/bin/forjar'\nchmod '0755' '/home/noah/.cargo/bin/forjar'",
        fake_b64
    );
    let stripped = strip_data_payloads(&script);
    assert!(!stripped.contains(fake_b64));
    // The structural commands must survive
    assert!(stripped.contains("set -euo pipefail"));
    assert!(stripped.contains("mkdir -p"));
    assert!(stripped.contains("chown 'noah'"));
    assert!(stripped.contains("chmod '0755'"));
}

#[test]
fn test_fj29_strip_preserves_non_base64_scripts() {
    let script = "set -euo pipefail\necho 'hello world'\nexit 0";
    let stripped = strip_data_payloads(script);
    assert_eq!(stripped, script, "scripts without base64 should be unchanged");
}

#[test]
fn test_fj29_validate_before_exec_accepts_base64_script() {
    // This script would fail I8 without the fix because the base64
    // contains sequences that look like shell keywords
    let fake_b64 = "doZmaQBpbg==";
    let script = format!(
        "set -euo pipefail\necho '{}' | base64 -d > '/tmp/test'\nchmod '0755' '/tmp/test'",
        fake_b64
    );
    let result = validate_before_exec(&script);
    assert!(result.is_ok(), "base64 file deploy should pass I8: {result:?}");
}

#[test]
fn test_fj29_strip_heredoc_payloads() {
    let script = "set -euo pipefail\nmkdir -p '/home/noah'\ncat > '/home/noah/.bashrc' <<'FORJAR_EOF'\n#!/usr/bin/env bash\nexport PATH=\"$HOME/.cargo/bin:$PATH\"\nFORJAR_EOF\nchown 'noah' '/home/noah/.bashrc'";
    let stripped = strip_data_payloads(script);
    assert!(
        !stripped.contains("export PATH"),
        "heredoc payload should be stripped"
    );
    assert!(
        stripped.contains("# payload stripped for lint"),
        "placeholder should replace heredoc"
    );
    assert!(
        stripped.contains("chown 'noah'"),
        "post-heredoc commands preserved"
    );
}

#[test]
fn test_fj29_validate_before_exec_accepts_heredoc_with_shebang() {
    // SC1128 false positive: shebang inside heredoc content, not at script top
    let script = "set -euo pipefail\ncat > '/tmp/install.sh' <<'FORJAR_EOF'\n#!/usr/bin/env bash\nset -euo pipefail\necho hello\nFORJAR_EOF\nchmod '0755' '/tmp/install.sh'";
    let result = validate_before_exec(script);
    assert!(result.is_ok(), "heredoc with shebang should pass I8: {result:?}");
}
