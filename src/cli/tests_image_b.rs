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

    fn write_pub_key(dir: &Path) -> String {
        let key_path = dir.join("id_ed25519.pub");
        std::fs::write(&key_path, "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5 test@host\n").unwrap();
        key_path.to_str().unwrap().to_string()
    }

    fn make_machine(dir: &Path, hostname: &str, user: &str, addr: &str) -> Machine {
        let mut m = Machine::ssh(hostname, addr, user);
        m.ssh_key = Some(write_pub_key(dir));
        m
    }

    fn parse_user_data(yaml: &str) -> serde_yaml_ng::Value {
        serde_yaml_ng::from_str(yaml)
            .unwrap_or_else(|e| panic!("invalid autoinstall YAML: {e}\n---\n{yaml}"))
    }

    // --- generate_user_data direct tests ---

    #[test]
    fn test_generate_user_data_basic() {
        let dir = tempfile::tempdir().unwrap();
        let m = make_machine(dir.path(), "web-01", "deploy", "10.0.0.1");
        let yaml = generate_user_data("web-01", &m, "auto-lvm", "en_US.UTF-8", "UTC").unwrap();
        assert!(yaml.starts_with("#cloud-config"));
        let doc = parse_user_data(&yaml);
        assert_eq!(doc["autoinstall"]["version"].as_u64(), Some(1));
    }

    #[test]
    fn test_generate_user_data_identity_and_ssh_lockdown() {
        let dir = tempfile::tempdir().unwrap();
        let m = make_machine(dir.path(), "db-01", "admin", "10.0.0.2");
        let yaml = generate_user_data("db-01", &m, "auto-lvm", "C.UTF-8", "UTC").unwrap();
        let doc = parse_user_data(&yaml);
        assert_eq!(doc["autoinstall"]["identity"]["hostname"], "db-01");
        assert_eq!(doc["autoinstall"]["identity"]["username"], "admin");
        assert_eq!(doc["autoinstall"]["identity"]["password"], "!");
        assert_eq!(doc["autoinstall"]["ssh"]["install-server"].as_bool(), Some(true));
        assert_eq!(doc["autoinstall"]["ssh"]["allow-pw"].as_bool(), Some(false));
        let keys = doc["autoinstall"]["ssh"]["authorized-keys"].as_sequence().unwrap();
        assert_eq!(keys.len(), 1);
        assert!(keys[0].as_str().unwrap().starts_with("ssh-ed25519"));
    }

    #[test]
    fn test_generate_user_data_locale_and_keyboard() {
        let dir = tempfile::tempdir().unwrap();
        let m = make_machine(dir.path(), "host", "root", "10.0.0.1");
        let yaml =
            generate_user_data("host", &m, "auto-lvm", "de_DE.UTF-8", "Europe/Berlin").unwrap();
        let doc = parse_user_data(&yaml);
        assert_eq!(doc["autoinstall"]["locale"], "de_DE.UTF-8");
        assert_eq!(doc["autoinstall"]["timezone"], "Europe/Berlin");
        assert_eq!(doc["autoinstall"]["keyboard"]["layout"], "us");
    }

    #[test]
    fn test_generate_user_data_lvm_and_zfs_layouts() {
        let dir = tempfile::tempdir().unwrap();
        let m = make_machine(dir.path(), "host", "root", "10.0.0.1");
        for (disk, expected) in [("auto-lvm", "lvm"), ("auto-zfs", "zfs")] {
            let yaml = generate_user_data("host", &m, disk, "en_US.UTF-8", "UTC").unwrap();
            let doc = parse_user_data(&yaml);
            let storage = &doc["autoinstall"]["storage"];
            assert_eq!(storage["layout"]["name"], expected);
            assert!(storage.get("config").is_none());
            assert!(storage["layout"].get("match").is_none());
        }
    }

    #[test]
    fn test_generate_user_data_serial_glob_layout() {
        let dir = tempfile::tempdir().unwrap();
        let m = make_machine(dir.path(), "host", "root", "10.0.0.1");
        for glob in ["WD-WCC123*", "*NVME*"] {
            let disk = format!("serial:{glob}");
            let yaml = generate_user_data("host", &m, &disk, "en_US.UTF-8", "UTC").unwrap();
            let doc = parse_user_data(&yaml);
            let storage = &doc["autoinstall"]["storage"];
            assert_eq!(storage["layout"]["name"], "lvm");
            assert_eq!(storage["layout"]["match"]["serial"], glob);
            assert!(storage.get("config").is_none());
        }
    }

    #[test]
    fn test_generate_user_data_device_path_layout() {
        let dir = tempfile::tempdir().unwrap();
        let m = make_machine(dir.path(), "host", "root", "10.0.0.1");
        for dev in ["/dev/sda", "/dev/nvme0n1"] {
            let yaml = generate_user_data("host", &m, dev, "en_US.UTF-8", "UTC").unwrap();
            let doc = parse_user_data(&yaml);
            let storage = &doc["autoinstall"]["storage"];
            assert_eq!(storage["layout"]["name"], "lvm");
            assert_eq!(storage["layout"]["match"]["path"], dev);
            assert!(storage.get("config").is_none());
        }
    }

    #[test]
    fn test_generate_user_data_refuses_by_id_and_bad_disk() {
        let dir = tempfile::tempdir().unwrap();
        let m = make_machine(dir.path(), "host", "root", "10.0.0.1");
        for bad in [
            "/dev/disk/by-id/nvme-Samsung_SSD_980",
            "/dev/disk/by-uuid/1234-5678",
            "/dev/disk/by-path/pci-0000:00:1f.2",
            "serial:",
            "/dev/",
            "/dev/sub/sda",
            "btrfs",
        ] {
            assert!(
                generate_user_data("host", &m, bad, "en_US.UTF-8", "UTC").is_err(),
                "expected error for disk '{bad}'"
            );
        }
    }

    #[test]
    fn test_generate_user_data_sudo_packages_and_embed() {
        let dir = tempfile::tempdir().unwrap();
        let m = make_machine(dir.path(), "host", "noah", "10.0.0.1");
        let yaml = generate_user_data("host", &m, "auto-lvm", "en_US.UTF-8", "UTC").unwrap();
        let doc = parse_user_data(&yaml);
        let pkgs = doc["autoinstall"]["packages"].as_sequence().unwrap();
        assert!(pkgs.iter().any(|p| p == "openssh-server"));
        assert!(pkgs.iter().any(|p| p == "curl"));
        let cmds: Vec<&str> = doc["autoinstall"]["late-commands"]
            .as_sequence()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert!(cmds.iter().any(|c| c.contains("NOPASSWD:ALL") && c.contains("sudoers.d/noah-nopasswd")));
        assert!(cmds.iter().any(|c| c.contains("chmod 0440")));
        assert!(cmds.iter().any(|c| c.contains("/usr/local/bin/forjar")));
        assert!(cmds.iter().any(|c| c.contains("/etc/forjar/forjar.yaml")));
    }

    #[test]
    fn test_generate_user_data_ip_comments() {
        let dir = tempfile::tempdir().unwrap();
        for addr in ["192.168.1.100", "127.0.0.1", "container"] {
            let m = make_machine(dir.path(), "host", "root", addr);
            let y = generate_user_data("host", &m, "auto-lvm", "en_US.UTF-8", "UTC").unwrap();
            let doc = parse_user_data(&y);
            let net = &doc["autoinstall"]["network"];
            assert_eq!(net["version"].as_u64(), Some(2));
            assert_eq!(net["ethernets"]["en"]["match"]["name"], "en*");
            assert_eq!(net["ethernets"]["en"]["dhcp4"].as_bool(), Some(true));
            assert!(!y.contains(&format!("Static IP: {addr}")));
        }
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
    fn test_read_ssh_pub_key_none_is_error() {
        let err = read_ssh_pub_key(None).unwrap_err();
        assert!(err.message().contains("no ssh_key configured"));
    }

    #[test]
    fn test_read_ssh_pub_key_missing_file_is_error() {
        let err = read_ssh_pub_key(Some("/nonexistent/key")).unwrap_err();
        assert!(err.message().contains("read SSH public key"));
    }

    #[test]
    fn test_read_ssh_pub_key_empty_file_is_error() {
        let dir = tempfile::tempdir().unwrap();
        let key_path = dir.path().join("empty.pub");
        std::fs::write(&key_path, "\n   \n# comment\n").unwrap();
        let err = read_ssh_pub_key(Some(key_path.to_str().unwrap())).unwrap_err();
        assert!(err.message().contains("no SSH public keys found"));
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
            "version: \"1.0\"\nname: test\nmachines:\n  a: { hostname: a, addr: 10.0.0.1 }\n  b: { hostname: b, addr: 10.0.0.2 }\nresources:\n  pkg: { type: package, machine: a, provider: apt, packages: [vim] }\n",
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
            "version: \"1.0\"\nname: test\nresources:\n  pkg: { type: package, provider: apt, packages: [vim] }\n",
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
        let m = make_machine(dir.path(), "host", "noah", "10.0.0.1");
        let yaml = generate_user_data("host", &m, "auto-lvm", "en_US.UTF-8", "UTC").unwrap();
        let doc = parse_user_data(&yaml);
        assert_eq!(doc["autoinstall"]["ssh"]["allow-pw"].as_bool(), Some(false));
        assert_eq!(doc["autoinstall"]["identity"]["password"], "!");
        let keys = doc["autoinstall"]["ssh"]["authorized-keys"].as_sequence().unwrap();
        assert!(keys[0].as_str().unwrap().starts_with("ssh-ed25519"));
    }

    #[test]
    fn test_user_data_no_ssh_key_is_error() {
        let m = Machine::ssh("host", "10.0.0.1", "noah");
        let err = generate_user_data("host", &m, "auto-lvm", "en_US.UTF-8", "UTC").unwrap_err();
        assert!(err.contains("no ssh_key configured"));
    }

    // --- stage_iso_payload (item 4) ---

    #[test]
    fn test_stage_iso_payload_writes_inside_packed_dir() {
        let dir = tempfile::tempdir().unwrap();
        let work = dir.path().join("work");
        std::fs::create_dir_all(&work).unwrap();
        let cfg = write_config(dir.path(), "version: \"1.0\"\nname: t\n");
        let m = make_machine(dir.path(), "host", "noah", "10.0.0.1");
        let ud = generate_user_data("host", &m, "serial:NVME-*", "en_US.UTF-8", "UTC").unwrap();

        stage_iso_payload(&work, &ud, &cfg).unwrap();

        let packed = packed_iso_dir(&work);
        assert_eq!(packed, work.join("iso"));
        assert!(packed.join("nocloud/user-data").is_file());
        assert!(packed.join("nocloud/meta-data").is_file());
        assert!(packed.join("forjar/forjar.yaml").is_file());
        assert!(packed.join("forjar/bin/forjar").is_file());
        assert!(!work.join("nocloud").exists());
        assert!(!work.join("forjar").exists());

        let staged_ud = std::fs::read_to_string(packed.join("nocloud/user-data")).unwrap();
        let doc = parse_user_data(&staged_ud);
        assert_eq!(doc["autoinstall"]["storage"]["layout"]["match"]["serial"], "NVME-*");
    }

    // --- ISO error paths ---

    const MIN_MACHINE_CFG: &str = "version: \"1.0\"\nname: test\nmachines:\n  m: { hostname: m, addr: 10.0.0.1 }\nresources:\n  pkg: { type: package, machine: m, provider: apt, packages: [vim] }\n";

    #[test]
    fn test_iso_missing_base_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), MIN_MACHINE_CFG);
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
        let p = write_config(dir.path(), MIN_MACHINE_CFG);
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

    // --- typed ForjarError at the image_cmd boundary (forjar#1074) ---

    #[test]
    fn test_storage_layout_unknown_disk_is_typed_error() {
        let err = storage_layout_yaml("btrfs").unwrap_err();
        assert_eq!(err.class(), crate::core::error::ErrorClass::Other);
        assert_eq!(err.exit_code(), 1);
        assert_eq!(
            err.message(),
            "unknown disk layout: btrfs (use auto-lvm, auto-zfs, serial:<glob>, or /dev/<kernel-name>)"
        );
    }

    #[test]
    fn test_extract_iso_missing_input_is_typed_error() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("absent.iso");
        let err = extract_iso(&missing, dir.path()).unwrap_err();
        assert_eq!(err.class(), crate::core::error::ErrorClass::Other);
        assert!(
            err.message().starts_with("run xorriso extract:")
                || err.message() == "xorriso extraction failed",
            "unexpected message: {}",
            err.message()
        );
    }

    #[test]
    fn test_embed_forjar_binary_unwritable_is_typed_error() {
        let dir = tempfile::tempdir().unwrap();
        // A file where the work dir should be makes create_dir_all fail.
        let blocker = dir.path().join("blocker");
        std::fs::write(&blocker, "x").unwrap();
        let err = embed_forjar_binary(&blocker).unwrap_err();
        assert_eq!(err.class(), crate::core::error::ErrorClass::Other);
        assert!(err.message().starts_with("create bin dir:"));
    }

    #[test]
    fn test_ensure_xorriso_error_is_typed_when_missing() {
        // Either xorriso is installed (Ok) or the typed message is exact.
        if let Err(err) = ensure_xorriso() {
            assert_eq!(err.class(), crate::core::error::ErrorClass::Other);
            assert_eq!(
                err.message(),
                "xorriso not found — install with: sudo apt install xorriso"
            );
        }
    }
}
