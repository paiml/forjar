//! FJ-52: Tests for autoinstall image generation.

#[cfg(test)]
mod tests {
    use crate::cli::image_cmd::*;
    use crate::core::parser::parse_config;
    use std::path::Path;

    fn write_config(dir: &Path, yaml: &str) -> std::path::PathBuf {
        let p = dir.join("forjar.yaml");
        std::fs::write(&p, yaml).unwrap();
        p
    }

    const SINGLE_MACHINE_YAML: &str = r#"
version: "1.0"
name: test-image
machines:
  yoga:
    hostname: yoga
    addr: 192.168.50.38
    user: noah
resources:
  pkg-curl:
    type: package
    machine: yoga
    provider: apt
    packages: [curl]
"#;

    const MULTI_MACHINE_YAML: &str = r#"
version: "1.0"
name: test-multi
machines:
  web:
    hostname: web-01
    addr: 10.0.0.1
    user: deploy
  db:
    hostname: db-01
    addr: 10.0.0.2
    user: deploy
resources:
  pkg-curl:
    type: package
    machine: web
    provider: apt
    packages: [curl]
"#;

    #[test]
    fn test_fj52_user_data_to_stdout() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), SINGLE_MACHINE_YAML);
        let result = cmd_image_user_data(
            &p,
            Some("yoga"),
            "auto-lvm",
            "en_US.UTF-8",
            "UTC",
            None,
            false,
        );
        assert!(result.is_ok(), "user_data must succeed: {result:?}");
    }

    #[test]
    fn test_fj52_user_data_to_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), SINGLE_MACHINE_YAML);
        let out = dir.path().join("user-data");
        let result = cmd_image_user_data(
            &p,
            Some("yoga"),
            "auto-lvm",
            "en_US.UTF-8",
            "UTC",
            Some(&out),
            false,
        );
        assert!(result.is_ok());
        assert!(out.exists(), "user-data file must exist");
        let content = std::fs::read_to_string(&out).unwrap();
        assert!(content.contains("#cloud-config"), "must have cloud-config header");
        assert!(content.contains("autoinstall:"), "must have autoinstall section");
    }

    #[test]
    fn test_fj52_user_data_json() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), SINGLE_MACHINE_YAML);
        let out = dir.path().join("user-data.yaml");
        let result = cmd_image_user_data(
            &p,
            Some("yoga"),
            "auto-lvm",
            "en_US.UTF-8",
            "UTC",
            Some(&out),
            true,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj52_user_data_hostname() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), SINGLE_MACHINE_YAML);
        let out = dir.path().join("user-data");
        cmd_image_user_data(&p, Some("yoga"), "auto-lvm", "en_US.UTF-8", "UTC", Some(&out), false)
            .unwrap();
        let content = std::fs::read_to_string(&out).unwrap();
        assert!(content.contains("hostname: yoga"), "must set hostname: {content}");
    }

    #[test]
    fn test_fj52_user_data_username() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), SINGLE_MACHINE_YAML);
        let out = dir.path().join("user-data");
        cmd_image_user_data(&p, Some("yoga"), "auto-lvm", "en_US.UTF-8", "UTC", Some(&out), false)
            .unwrap();
        let content = std::fs::read_to_string(&out).unwrap();
        assert!(content.contains("username: noah"), "must set username: {content}");
    }

    #[test]
    fn test_fj52_user_data_locale_timezone() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), SINGLE_MACHINE_YAML);
        let out = dir.path().join("user-data");
        cmd_image_user_data(
            &p,
            Some("yoga"),
            "auto-lvm",
            "en_US.UTF-8",
            "America/New_York",
            Some(&out),
            false,
        )
        .unwrap();
        let content = std::fs::read_to_string(&out).unwrap();
        assert!(content.contains("locale: en_US.UTF-8"), "must set locale");
        assert!(
            content.contains("timezone: America/New_York"),
            "must set timezone"
        );
    }

    #[test]
    fn test_fj52_user_data_lvm_storage() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), SINGLE_MACHINE_YAML);
        let out = dir.path().join("user-data");
        cmd_image_user_data(&p, Some("yoga"), "auto-lvm", "en_US.UTF-8", "UTC", Some(&out), false)
            .unwrap();
        let content = std::fs::read_to_string(&out).unwrap();
        assert!(content.contains("name: lvm"), "auto-lvm must produce lvm layout");
    }

    #[test]
    fn test_fj52_user_data_zfs_storage() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), SINGLE_MACHINE_YAML);
        let out = dir.path().join("user-data");
        cmd_image_user_data(&p, Some("yoga"), "auto-zfs", "en_US.UTF-8", "UTC", Some(&out), false)
            .unwrap();
        let content = std::fs::read_to_string(&out).unwrap();
        assert!(content.contains("name: zfs"), "auto-zfs must produce zfs layout");
    }

    #[test]
    fn test_fj52_user_data_device_path() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), SINGLE_MACHINE_YAML);
        let out = dir.path().join("user-data");
        cmd_image_user_data(
            &p,
            Some("yoga"),
            "/dev/nvme0n1",
            "en_US.UTF-8",
            "UTC",
            Some(&out),
            false,
        )
        .unwrap();
        let content = std::fs::read_to_string(&out).unwrap();
        assert!(
            content.contains("/dev/nvme0n1"),
            "device path must appear: {content}"
        );
    }

    #[test]
    fn test_fj52_user_data_bad_disk() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), SINGLE_MACHINE_YAML);
        let result = cmd_image_user_data(
            &p,
            Some("yoga"),
            "garbage",
            "en_US.UTF-8",
            "UTC",
            None,
            false,
        );
        assert!(result.is_err(), "bad disk layout must fail");
        assert!(result.unwrap_err().contains("unknown disk layout"));
    }

    #[test]
    fn test_fj52_user_data_sudo() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), SINGLE_MACHINE_YAML);
        let out = dir.path().join("user-data");
        cmd_image_user_data(&p, Some("yoga"), "auto-lvm", "en_US.UTF-8", "UTC", Some(&out), false)
            .unwrap();
        let content = std::fs::read_to_string(&out).unwrap();
        assert!(
            content.contains("NOPASSWD:ALL"),
            "must configure passwordless sudo"
        );
        assert!(
            content.contains("sudoers.d/noah"),
            "must use username in sudoers path"
        );
    }

    #[test]
    fn test_fj52_user_data_firstboot_service() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), SINGLE_MACHINE_YAML);
        let out = dir.path().join("user-data");
        cmd_image_user_data(&p, Some("yoga"), "auto-lvm", "en_US.UTF-8", "UTC", Some(&out), false)
            .unwrap();
        let content = std::fs::read_to_string(&out).unwrap();
        assert!(
            content.contains("forjar-firstboot.service"),
            "must install firstboot service"
        );
        assert!(
            content.contains("forjar apply --yes"),
            "firstboot must run forjar apply"
        );
        assert!(
            content.contains("ConditionPathExists"),
            "must be idempotent"
        );
    }

    #[test]
    fn test_fj52_user_data_ssh_server() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), SINGLE_MACHINE_YAML);
        let out = dir.path().join("user-data");
        cmd_image_user_data(&p, Some("yoga"), "auto-lvm", "en_US.UTF-8", "UTC", Some(&out), false)
            .unwrap();
        let content = std::fs::read_to_string(&out).unwrap();
        assert!(
            content.contains("install-server: true"),
            "must install SSH server"
        );
        assert!(
            content.contains("openssh-server"),
            "must include openssh-server package"
        );
    }

    #[test]
    fn test_fj52_single_machine_auto_resolve() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), SINGLE_MACHINE_YAML);
        // No --machine specified, should auto-resolve single machine
        let result = cmd_image_user_data(&p, None, "auto-lvm", "en_US.UTF-8", "UTC", None, false);
        assert!(result.is_ok(), "single machine must auto-resolve");
    }

    #[test]
    fn test_fj52_multi_machine_requires_flag() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), MULTI_MACHINE_YAML);
        let result = cmd_image_user_data(&p, None, "auto-lvm", "en_US.UTF-8", "UTC", None, false);
        assert!(result.is_err(), "multi-machine without --machine must fail");
        let err = result.unwrap_err();
        assert!(
            err.contains("--machine"),
            "error must mention --machine: {err}"
        );
    }

    #[test]
    fn test_fj52_multi_machine_with_flag() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), MULTI_MACHINE_YAML);
        let result =
            cmd_image_user_data(&p, Some("web"), "auto-lvm", "en_US.UTF-8", "UTC", None, false);
        assert!(result.is_ok(), "selecting a machine must succeed");
    }

    #[test]
    fn test_fj52_missing_machine() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), SINGLE_MACHINE_YAML);
        let result = cmd_image_user_data(
            &p,
            Some("nonexistent"),
            "auto-lvm",
            "en_US.UTF-8",
            "UTC",
            None,
            false,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_fj52_iso_missing_base() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), SINGLE_MACHINE_YAML);
        let result = cmd_image_iso(
            &p,
            Some("yoga"),
            Path::new("/nonexistent/base.iso"),
            Path::new("/tmp/out.iso"),
            "auto-lvm",
            "en_US.UTF-8",
            "UTC",
            false,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("base ISO not found"));
    }

    #[test]
    fn test_fj52_user_data_forjar_binary_copy() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), SINGLE_MACHINE_YAML);
        let out = dir.path().join("user-data");
        cmd_image_user_data(&p, Some("yoga"), "auto-lvm", "en_US.UTF-8", "UTC", Some(&out), false)
            .unwrap();
        let content = std::fs::read_to_string(&out).unwrap();
        assert!(
            content.contains("/usr/local/bin/forjar"),
            "must copy forjar binary to target"
        );
    }

    #[test]
    fn test_fj52_user_data_config_copy() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), SINGLE_MACHINE_YAML);
        let out = dir.path().join("user-data");
        cmd_image_user_data(&p, Some("yoga"), "auto-lvm", "en_US.UTF-8", "UTC", Some(&out), false)
            .unwrap();
        let content = std::fs::read_to_string(&out).unwrap();
        assert!(
            content.contains("/etc/forjar/forjar.yaml"),
            "must copy forjar config to target"
        );
    }

    #[test]
    fn test_fj52_expand_tilde() {
        let home = std::env::var("HOME").unwrap_or_default();
        let result = super::tests::expand_tilde_wrapper("~/foo/bar");
        if !home.is_empty() {
            assert_eq!(result, format!("{home}/foo/bar"));
        }
        // No tilde
        assert_eq!(
            super::tests::expand_tilde_wrapper("/absolute/path"),
            "/absolute/path"
        );
    }

    // Wrapper to call private fn
    pub(super) fn expand_tilde_wrapper(path: &str) -> String {
        crate::cli::image_cmd::expand_tilde(path)
    }

    #[test]
    fn test_fj52_user_data_static_ip_comment() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), SINGLE_MACHINE_YAML);
        let out = dir.path().join("user-data");
        cmd_image_user_data(&p, Some("yoga"), "auto-lvm", "en_US.UTF-8", "UTC", Some(&out), false)
            .unwrap();
        let content = std::fs::read_to_string(&out).unwrap();
        assert!(
            content.contains("192.168.50.38"),
            "must include static IP comment"
        );
    }

    #[test]
    fn test_fj52_user_data_root_user() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = r#"
version: "1.0"
name: test
machines:
  srv:
    hostname: server
    addr: 10.0.0.5
    user: root
resources:
  pkg:
    type: package
    machine: srv
    provider: apt
    packages: [vim]
"#;
        let p = write_config(dir.path(), yaml);
        let out = dir.path().join("user-data");
        cmd_image_user_data(&p, Some("srv"), "auto-lvm", "en_US.UTF-8", "UTC", Some(&out), false)
            .unwrap();
        let content = std::fs::read_to_string(&out).unwrap();
        assert!(content.contains("username: root"), "must use root username");
    }
}
