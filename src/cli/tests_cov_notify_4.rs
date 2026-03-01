//! Coverage tests: check.rs — cmd_check with filters, cmd_test with filters.
//! Also covers doctor.rs — cmd_doctor edge cases, cmd_doctor_network edge cases.

#![allow(unused_imports)]
use std::path::Path;
use super::dispatch_notify::*;
use super::dispatch_notify_custom::*;
use super::secrets::*;
use super::check::*;
use super::doctor::*;
use super::test_fixtures::*;

#[cfg(test)]
mod tests {
    use super::*;

    // ─── check.rs — cmd_check with filters ────────────────────────

    fn write_check_config(dir: &std::path::Path) -> std::path::PathBuf {
        let file = dir.join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: check-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  pkg1:
    type: package
    machine: local
    name: coreutils
    tags: [base, system]
  cfg1:
    type: file
    machine: local
    path: /tmp/forjar-check-test.txt
    content: "hello"
    tags: [config]
"#,
        )
        .unwrap();
        file
    }

    #[test]
    fn test_cmd_check_with_tag_filter() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_check_config(dir.path());
        // Filter by tag "config" — should skip "pkg1" (base,system tags)
        let result = cmd_check(&config, None, None, Some("config"), false, false);
        // May fail because check script might not exist for file, but exercises filter code
        let _ = result;
    }

    #[test]
    fn test_cmd_check_with_tag_filter_json() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_check_config(dir.path());
        let _ = cmd_check(&config, None, None, Some("config"), true, false);
    }

    #[test]
    fn test_cmd_check_with_resource_filter() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_check_config(dir.path());
        let _ = cmd_check(&config, None, Some("pkg1"), None, false, false);
    }

    #[test]
    fn test_cmd_check_with_resource_filter_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_check_config(dir.path());
        let _ = cmd_check(&config, None, Some("nonexistent"), None, false, false);
    }

    #[test]
    fn test_cmd_check_with_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_check_config(dir.path());
        let _ = cmd_check(&config, Some("local"), None, None, false, false);
    }

    #[test]
    fn test_cmd_check_with_machine_filter_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_check_config(dir.path());
        let _ = cmd_check(&config, Some("nonexistent"), None, None, false, false);
    }

    #[test]
    fn test_cmd_check_verbose() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_check_config(dir.path());
        let _ = cmd_check(&config, None, None, None, false, true);
    }

    #[test]
    fn test_cmd_check_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_check_config(dir.path());
        let _ = cmd_check(&config, None, None, None, true, false);
    }

    #[test]
    fn test_cmd_check_combined_filters() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_check_config(dir.path());
        let _ = cmd_check(&config, Some("local"), Some("pkg1"), Some("base"), false, false);
    }

    #[test]
    fn test_cmd_check_invalid_config() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("bad.yaml");
        std::fs::write(&file, "invalid: [[[").unwrap();
        let result = cmd_check(&file, None, None, None, false, false);
        assert!(result.is_err());
    }

    // ─── check.rs — cmd_test ──────────────────────────────────────

    #[test]
    fn test_cmd_test_basic() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_check_config(dir.path());
        let _ = cmd_test(&config, None, None, None, None, false, false);
    }

    #[test]
    fn test_cmd_test_json() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_check_config(dir.path());
        let _ = cmd_test(&config, None, None, None, None, true, false);
    }

    #[test]
    fn test_cmd_test_verbose() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_check_config(dir.path());
        let _ = cmd_test(&config, None, None, None, None, false, true);
    }

    #[test]
    fn test_cmd_test_with_tag_filter() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_check_config(dir.path());
        let _ = cmd_test(&config, None, None, Some("base"), None, false, false);
    }

    #[test]
    fn test_cmd_test_with_resource_filter() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_check_config(dir.path());
        let _ = cmd_test(&config, None, Some("cfg1"), None, None, false, false);
    }

    #[test]
    fn test_cmd_test_with_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_check_config(dir.path());
        let _ = cmd_test(&config, Some("local"), None, None, None, false, false);
    }

    #[test]
    fn test_cmd_test_with_group_filter() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_check_config(dir.path());
        // No resources have resource_group set, so all skip
        let _ = cmd_test(&config, None, None, None, Some("web"), false, false);
    }

    #[test]
    fn test_cmd_test_with_nonexistent_resource_filter() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_check_config(dir.path());
        let _ = cmd_test(&config, None, Some("nonexistent"), None, None, false, false);
    }

    #[test]
    fn test_cmd_test_invalid_config() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("bad.yaml");
        std::fs::write(&file, "invalid: [[[").unwrap();
        let result = cmd_test(&file, None, None, None, None, false, false);
        assert!(result.is_err());
    }

    // ─── doctor.rs — cmd_doctor edge cases ────────────────────────

    #[test]
    fn test_cmd_doctor_with_fix_flag() {
        let result = cmd_doctor(None, false, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_doctor_json_with_fix() {
        let result = cmd_doctor(None, true, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_doctor_with_enc_markers_config() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: enc-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  secret:
    type: file
    machine: local
    path: /tmp/secret.txt
    content: "ENC[age,fakeciphertext]"
"#,
        )
        .unwrap();
        // This config has ENC markers, so doctor should check age identity
        let _ = cmd_doctor(Some(&file), false, false);
    }

    #[test]
    fn test_cmd_doctor_with_enc_markers_config_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: enc-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  secret:
    type: file
    machine: local
    path: /tmp/secret.txt
    content: "ENC[age,fakeciphertext]"
"#,
        )
        .unwrap();
        let _ = cmd_doctor(Some(&file), true, false);
    }

    #[test]
    fn test_cmd_doctor_with_container_config_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: ctr-test
machines:
  ctr:
    hostname: ctr
    addr: container
    transport: container
    container:
      image: alpine:3.18
      runtime: podman
resources:
  f:
    type: file
    machine: ctr
    path: /tmp/test
    content: "x"
"#,
        )
        .unwrap();
        // Tests container runtime check (podman) and JSON output
        let _ = cmd_doctor(Some(&file), true, false);
    }

    #[test]
    fn test_cmd_doctor_with_container_default_runtime() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: ctr-test
machines:
  ctr:
    hostname: ctr
    addr: container
    transport: container
resources:
  f:
    type: file
    machine: ctr
    path: /tmp/test
    content: "x"
"#,
        )
        .unwrap();
        // No container.runtime specified => defaults to "docker"
        let _ = cmd_doctor(Some(&file), false, false);
    }

    // ─── doctor.rs — cmd_doctor_network edge cases ────────────────

    #[test]
    fn test_cmd_doctor_network_no_file_default_path() {
        // None uses default "forjar.yaml" which likely doesn't exist in test cwd
        let result = cmd_doctor_network(None, false);
        // Will likely fail because forjar.yaml doesn't exist in the test dir
        let _ = result;
    }

    #[test]
    fn test_cmd_doctor_network_local_only() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: net-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: local
    path: /tmp/test
    content: "x"
"#,
        )
        .unwrap();
        let result = cmd_doctor_network(Some(&file), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_doctor_network_local_only_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: net-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: local
    path: /tmp/test
    content: "x"
"#,
        )
        .unwrap();
        let result = cmd_doctor_network(Some(&file), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_doctor_network_with_ssh_key() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: net-test
machines:
  remote:
    hostname: remote
    addr: 10.0.0.99
    user: deploy
    ssh_key: /nonexistent/key.pem
resources:
  f:
    type: file
    machine: remote
    path: /tmp/test
    content: "x"
"#,
        )
        .unwrap();
        // SSH will fail because host is unreachable, but exercises the ssh_key code path
        let result = cmd_doctor_network(Some(&file), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_doctor_network_with_ssh_key_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: net-test
machines:
  remote:
    hostname: remote
    addr: 10.0.0.99
    user: deploy
    ssh_key: /nonexistent/key.pem
resources:
  f:
    type: file
    machine: remote
    path: /tmp/test
    content: "x"
"#,
        )
        .unwrap();
        let result = cmd_doctor_network(Some(&file), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_doctor_network_localhost_alias() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: net-test
machines:
  lo:
    hostname: lo
    addr: localhost
resources:
  f:
    type: file
    machine: lo
    path: /tmp/test
    content: "x"
"#,
        )
        .unwrap();
        let result = cmd_doctor_network(Some(&file), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_doctor_network_invalid_config() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(&file, "invalid: [[[").unwrap();
        let result = cmd_doctor_network(Some(&file), false);
        assert!(result.is_err());
    }

    // ─── check.rs — cmd_test with combined filters ────────────────

    #[test]
    fn test_cmd_test_combined_machine_and_tag_filter() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_check_config(dir.path());
        let _ = cmd_test(&config, Some("local"), None, Some("system"), None, false, false);
    }

    #[test]
    fn test_cmd_test_json_with_group_filter() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_check_config(dir.path());
        let _ = cmd_test(&config, None, None, None, Some("nonexistent"), true, false);
    }

    #[test]
    fn test_cmd_test_verbose_json() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_check_config(dir.path());
        let _ = cmd_test(&config, None, None, None, None, true, true);
    }
}
