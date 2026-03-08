//! FJ-52: Additional coverage tests for image generation internals.

#[cfg(test)]
mod tests {
    use crate::cli::image_cmd::*;
    use crate::core::types::Machine;
    use std::path::Path;

    fn write_config(dir: &Path, yaml: &str) -> std::path::PathBuf {
        let p = dir.join("forjar.yaml");
        std::fs::write(&p, yaml).unwrap();
        p
    }

    fn make_machine(hostname: &str, user: &str, addr: &str) -> Machine {
        Machine::ssh(hostname, addr, user)
    }

    // --- generate_user_data direct tests ---

    #[test]
    fn test_generate_user_data_basic() {
        let m = make_machine("web-01", "deploy", "10.0.0.1");
        let result = generate_user_data("web-01", &m, "auto-lvm", "en_US.UTF-8", "UTC");
        assert!(result.is_ok());
        let yaml = result.unwrap();
        assert!(yaml.starts_with("#cloud-config"));
        assert!(yaml.contains("autoinstall:"));
        assert!(yaml.contains("version: 1"));
    }

    #[test]
    fn test_generate_user_data_identity() {
        let m = make_machine("db-01", "admin", "10.0.0.2");
        let yaml = generate_user_data("db-01", &m, "auto-lvm", "C.UTF-8", "UTC").unwrap();
        assert!(yaml.contains("hostname: db-01"));
        assert!(yaml.contains("username: admin"));
    }

    #[test]
    fn test_generate_user_data_locale() {
        let m = make_machine("host", "root", "10.0.0.1");
        let yaml =
            generate_user_data("host", &m, "auto-lvm", "de_DE.UTF-8", "Europe/Berlin").unwrap();
        assert!(yaml.contains("locale: de_DE.UTF-8"));
        assert!(yaml.contains("timezone: Europe/Berlin"));
    }

    #[test]
    fn test_generate_user_data_keyboard() {
        let m = make_machine("host", "root", "10.0.0.1");
        let yaml = generate_user_data("host", &m, "auto-lvm", "en_US.UTF-8", "UTC").unwrap();
        assert!(yaml.contains("keyboard:"));
        assert!(yaml.contains("layout: us"));
    }

    #[test]
    fn test_generate_user_data_lvm_layout() {
        let m = make_machine("host", "root", "10.0.0.1");
        let yaml = generate_user_data("host", &m, "auto-lvm", "en_US.UTF-8", "UTC").unwrap();
        assert!(yaml.contains("storage:"));
        assert!(yaml.contains("name: lvm"));
    }

    #[test]
    fn test_generate_user_data_zfs_layout() {
        let m = make_machine("host", "root", "10.0.0.1");
        let yaml = generate_user_data("host", &m, "auto-zfs", "en_US.UTF-8", "UTC").unwrap();
        assert!(yaml.contains("name: zfs"));
    }

    #[test]
    fn test_generate_user_data_device_path_layout() {
        let m = make_machine("host", "root", "10.0.0.1");
        let yaml =
            generate_user_data("host", &m, "/dev/sda", "en_US.UTF-8", "UTC").unwrap();
        assert!(yaml.contains("path: /dev/sda"));
        assert!(yaml.contains("name: lvm"));
    }

    #[test]
    fn test_generate_user_data_nvme_path() {
        let m = make_machine("host", "root", "10.0.0.1");
        let yaml =
            generate_user_data("host", &m, "/dev/nvme0n1", "en_US.UTF-8", "UTC").unwrap();
        assert!(yaml.contains("/dev/nvme0n1"));
    }

    #[test]
    fn test_generate_user_data_bad_disk_layout() {
        let m = make_machine("host", "root", "10.0.0.1");
        let result = generate_user_data("host", &m, "btrfs", "en_US.UTF-8", "UTC");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown disk layout"));
    }

    #[test]
    fn test_generate_user_data_sudo_config() {
        let m = make_machine("host", "noah", "10.0.0.1");
        let yaml = generate_user_data("host", &m, "auto-lvm", "en_US.UTF-8", "UTC").unwrap();
        assert!(yaml.contains("NOPASSWD:ALL"));
        assert!(yaml.contains("sudoers.d/noah-nopasswd"));
        assert!(yaml.contains("chmod 0440"));
    }

    #[test]
    fn test_generate_user_data_packages() {
        let m = make_machine("host", "root", "10.0.0.1");
        let yaml = generate_user_data("host", &m, "auto-lvm", "en_US.UTF-8", "UTC").unwrap();
        assert!(yaml.contains("packages:"));
        assert!(yaml.contains("openssh-server"));
        assert!(yaml.contains("curl"));
    }

    #[test]
    fn test_generate_user_data_forjar_embed() {
        let m = make_machine("host", "root", "10.0.0.1");
        let yaml = generate_user_data("host", &m, "auto-lvm", "en_US.UTF-8", "UTC").unwrap();
        assert!(yaml.contains("/usr/local/bin/forjar"));
        assert!(yaml.contains("/etc/forjar/forjar.yaml"));
    }

    #[test]
    fn test_generate_user_data_static_ip_comment() {
        let m = make_machine("host", "root", "192.168.1.100");
        let yaml = generate_user_data("host", &m, "auto-lvm", "en_US.UTF-8", "UTC").unwrap();
        assert!(yaml.contains("192.168.1.100"));
    }

    #[test]
    fn test_generate_user_data_localhost_no_ip_comment() {
        let m = make_machine("host", "root", "127.0.0.1");
        let yaml = generate_user_data("host", &m, "auto-lvm", "en_US.UTF-8", "UTC").unwrap();
        assert!(!yaml.contains("Static IP: 127.0.0.1"));
    }

    #[test]
    fn test_generate_user_data_container_no_ip_comment() {
        let m = make_machine("host", "root", "container");
        let yaml = generate_user_data("host", &m, "auto-lvm", "en_US.UTF-8", "UTC").unwrap();
        assert!(!yaml.contains("Static IP: container"));
    }

    // --- firstboot_service_command tests ---

    #[test]
    fn test_firstboot_service_unit() {
        let cmd = firstboot_service_command();
        assert!(cmd.contains("[Unit]"));
        assert!(cmd.contains("[Service]"));
        assert!(cmd.contains("[Install]"));
    }

    #[test]
    fn test_firstboot_service_network_after() {
        let cmd = firstboot_service_command();
        assert!(cmd.contains("After=network-online.target"));
        assert!(cmd.contains("Wants=network-online.target"));
    }

    #[test]
    fn test_firstboot_service_idempotent() {
        let cmd = firstboot_service_command();
        assert!(cmd.contains("ConditionPathExists=!/etc/forjar/.firstboot-done"));
        assert!(cmd.contains("touch /etc/forjar/.firstboot-done"));
    }

    #[test]
    fn test_firstboot_service_timeout() {
        let cmd = firstboot_service_command();
        assert!(cmd.contains("TimeoutSec=1800"));
    }

    #[test]
    fn test_firstboot_service_enable() {
        let cmd = firstboot_service_command();
        assert!(cmd.contains("systemctl enable forjar-firstboot"));
    }

    // --- read_ssh_pub_key tests ---

    #[test]
    fn test_read_ssh_pub_key_none() {
        let keys = read_ssh_pub_key(None).unwrap();
        assert!(keys.is_empty());
    }

    #[test]
    fn test_read_ssh_pub_key_missing_file() {
        let keys = read_ssh_pub_key(Some("/nonexistent/key")).unwrap();
        assert!(keys.is_empty());
    }

    #[test]
    fn test_read_ssh_pub_key_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let key_path = dir.path().join("id_test.pub");
        std::fs::write(&key_path, "ssh-ed25519 AAAAC3... test@host\n").unwrap();
        let keys = read_ssh_pub_key(Some(key_path.to_str().unwrap())).unwrap();
        assert_eq!(keys.len(), 1);
        assert!(keys[0].starts_with("ssh-ed25519"));
    }

    #[test]
    fn test_read_ssh_pub_key_adds_pub_suffix() {
        let dir = tempfile::tempdir().unwrap();
        let key_path = dir.path().join("id_test.pub");
        std::fs::write(&key_path, "ssh-rsa AAAAB3... test@host\n").unwrap();
        // Pass without .pub — should try appending .pub
        let private_path = dir.path().join("id_test");
        std::fs::write(&private_path, "PRIVATE KEY").unwrap();
        let keys = read_ssh_pub_key(Some(private_path.to_str().unwrap())).unwrap();
        assert_eq!(keys.len(), 1);
        assert!(keys[0].starts_with("ssh-rsa"));
    }

    #[test]
    fn test_read_ssh_pub_key_multi_line() {
        let dir = tempfile::tempdir().unwrap();
        let key_path = dir.path().join("authorized_keys.pub");
        std::fs::write(&key_path, "ssh-ed25519 AAA... user1\nssh-rsa BBB... user2\n").unwrap();
        let keys = read_ssh_pub_key(Some(key_path.to_str().unwrap())).unwrap();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_read_ssh_pub_key_skips_empty_lines() {
        let dir = tempfile::tempdir().unwrap();
        let key_path = dir.path().join("key.pub");
        std::fs::write(&key_path, "ssh-ed25519 AAA...\n\n\nssh-rsa BBB...\n").unwrap();
        let keys = read_ssh_pub_key(Some(key_path.to_str().unwrap())).unwrap();
        assert_eq!(keys.len(), 2);
    }

    // --- expand_tilde tests ---

    #[test]
    fn test_expand_tilde_home() {
        let result = expand_tilde("~/Documents/key");
        if let Ok(home) = std::env::var("HOME") {
            assert_eq!(result, format!("{home}/Documents/key"));
        }
    }

    #[test]
    fn test_expand_tilde_no_tilde() {
        assert_eq!(expand_tilde("/absolute/path"), "/absolute/path");
        assert_eq!(expand_tilde("relative/path"), "relative/path");
    }

    #[test]
    fn test_expand_tilde_just_tilde() {
        // "~" alone should not expand (only "~/" prefix)
        assert_eq!(expand_tilde("~"), "~");
    }

    // --- resolve_machine tests ---

    #[test]
    fn test_resolve_machine_by_name() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(
            dir.path(),
            r#"
version: "1.0"
name: test
machines:
  a:
    hostname: a
    addr: 10.0.0.1
  b:
    hostname: b
    addr: 10.0.0.2
resources:
  pkg:
    type: package
    machine: a
    provider: apt
    packages: [vim]
"#,
        );
        let config = crate::core::parser::parse_and_validate(&p).unwrap();
        let (name, m) = resolve_machine(&config, Some("a")).unwrap();
        assert_eq!(name, "a");
        assert_eq!(m.hostname, "a");
    }

    #[test]
    fn test_resolve_machine_no_machines() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(
            dir.path(),
            r#"
version: "1.0"
name: test
resources:
  pkg:
    type: package
    provider: apt
    packages: [vim]
"#,
        );
        let config = crate::core::parser::parse_and_validate(&p).unwrap();
        let result = resolve_machine(&config, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no machines"));
    }

    // --- user-data with SSH key ---

    #[test]
    fn test_user_data_with_ssh_key() {
        let dir = tempfile::tempdir().unwrap();
        let key_path = dir.path().join("id_ed25519.pub");
        std::fs::write(&key_path, "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5 test@host\n").unwrap();

        let mut m = make_machine("host", "noah", "10.0.0.1");
        m.ssh_key = Some(key_path.to_str().unwrap().to_string());
        let yaml = generate_user_data("host", &m, "auto-lvm", "en_US.UTF-8", "UTC").unwrap();
        assert!(yaml.contains("authorized-keys:"));
        assert!(yaml.contains("ssh-ed25519"));
    }

    #[test]
    fn test_user_data_no_ssh_key() {
        let m = make_machine("host", "noah", "10.0.0.1");
        let yaml = generate_user_data("host", &m, "auto-lvm", "en_US.UTF-8", "UTC").unwrap();
        assert!(!yaml.contains("authorized-keys:"));
    }

    // --- ISO error paths ---

    #[test]
    fn test_iso_missing_base_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(
            dir.path(),
            r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 10.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [vim]
"#,
        );
        let result = cmd_image_iso(
            &p,
            Some("m"),
            Path::new("/does/not/exist.iso"),
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
    fn test_iso_json_mode_missing_base() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(
            dir.path(),
            r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 10.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [vim]
"#,
        );
        let result = cmd_image_iso(
            &p,
            Some("m"),
            Path::new("/nonexistent.iso"),
            Path::new("/tmp/out.iso"),
            "auto-lvm",
            "en_US.UTF-8",
            "UTC",
            true,
        );
        assert!(result.is_err());
    }
}
