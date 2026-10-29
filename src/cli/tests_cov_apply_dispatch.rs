//! Coverage tests for dispatch_apply.rs — pre-checks and helpers.

use super::dispatch_apply::*;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_script_check_success() {
        let result = run_script_check("true");
        assert!(result.is_ok());
    }

    #[test]
    fn run_script_check_failure() {
        let result = run_script_check("false");
        assert!(result.is_err());
    }

    #[test]
    fn run_script_check_output() {
        let result = run_script_check("echo hello && exit 0");
        assert!(result.is_ok());
    }

    #[test]
    fn run_script_check_stderr() {
        let result = run_script_check("echo 'error msg' >&2 && exit 1");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("error msg"));
    }

    #[test]
    fn maybe_encrypt_state_no_encrypt() {
        let dir = tempfile::tempdir().unwrap();
        // encrypt=false: should do nothing
        maybe_encrypt_state(false, &Ok(()), dir.path());
    }

    #[test]
    fn maybe_encrypt_state_on_error() {
        let dir = tempfile::tempdir().unwrap();
        // encrypt=true but result is Err: should not encrypt
        maybe_encrypt_state(true, &Err("fail".to_string()), dir.path());
    }

    #[test]
    fn maybe_encrypt_state_on_success() {
        let dir = tempfile::tempdir().unwrap();
        // encrypt=true and result Ok: attempts encryption (empty dir, warns)
        maybe_encrypt_state(true, &Ok(()), dir.path());
    }

    #[test]
    fn run_pre_script_success() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("pre.sh");
        std::fs::write(&script, "#!/bin/bash\nexit 0\n").unwrap();
        let result = run_pre_script(&script);
        assert!(result.is_ok());
    }

    #[test]
    fn run_pre_script_failure() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("pre.sh");
        std::fs::write(&script, "#!/bin/bash\nexit 1\n").unwrap();
        let result = run_pre_script(&script);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Pre-script"));
    }

    #[test]
    fn run_pre_script_nonexistent() {
        let result = run_pre_script(Path::new("/nonexistent/script.sh"));
        assert!(result.is_err());
    }

    #[test]
    fn run_post_flight_success() {
        run_post_flight("true");
    }

    #[test]
    fn run_post_flight_failure() {
        // Should warn but not panic
        run_post_flight("exit 1");
    }

    #[test]
    fn check_cost_limit_ok() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            "version: \"1.0\"\nname: test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  pkg:\n    type: package\n    machine: m\n    provider: apt\n    packages: [curl]\n",
        )
        .unwrap();
        // High limit: should pass
        let result = check_cost_limit(&file, &state_dir, None, None, 100);
        assert!(result.is_ok());
    }

    #[test]
    fn check_cost_limit_exceeded() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            "version: \"1.0\"\nname: test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n",
        )
        .unwrap();
        // Limit 0: should fail (at least 2 changes planned)
        let result = check_cost_limit(&file, &state_dir, None, None, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cost limit exceeded"));
    }

    #[test]
    fn check_cost_limit_respects_resource_selector() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            "version: \"1.0\"\nname: test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n",
        )
        .unwrap();
        // Unselected: 2 changes planned -> fails limit 1
        let unoped = check_cost_limit(&file, &state_dir, None, None, 1);
        assert!(unoped.is_err());
        assert!(unoped.unwrap_err().contains("2 changes planned, limit is 1"));

        // With `-r a`: only 1 change planned -> passes limit 1, fails limit 0
        let sel_a = crate::cli::apply_selection::Selectors {
            resource: Some("a"),
            ..Default::default()
        };
        let ok_one = check_cost_limit_selected(&file, &state_dir, None, None, &sel_a, 1);
        assert!(ok_one.is_ok(), "expected -r a to count 1 change: {ok_one:?}");
        let fail_zero = check_cost_limit_selected(&file, &state_dir, None, None, &sel_a, 0);
        assert!(fail_zero.is_err());
        assert!(
            fail_zero
                .unwrap_err()
                .contains("1 changes planned, limit is 0")
        );
    }

    #[test]
    fn check_operator_auth_no_restrictions() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            "version: \"1.0\"\nname: test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n",
        )
        .unwrap();
        // No operator restrictions — should succeed
        let result = check_operator_auth(&file, None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn send_webhook_before_invalid_url() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        // Best-effort: should not panic
        send_webhook_before("http://localhost:99999/nonexistent", &file);
    }
}
