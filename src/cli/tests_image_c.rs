//! FJ-52 / #731: Unit tests for autoinstall network config, grub.cfg patching, and xorriso repack argv.

#[cfg(test)]
mod tests {
    use crate::cli::image_cmd::*;
    use crate::core::types::Machine;
    use std::path::Path;

    fn make_machine(dir: &Path, addr: &str) -> Machine {
        let key_path = dir.join("id_ed25519.pub");
        std::fs::write(&key_path, "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5 test@host\n").unwrap();
        let mut m = Machine::ssh("host", addr, "noah");
        m.ssh_key = Some(key_path.display().to_string());
        m
    }

    fn parse_yaml(yaml: &str) -> serde_yaml_ng::Value {
        serde_yaml_ng::from_str(yaml)
            .unwrap_or_else(|e| panic!("invalid autoinstall YAML: {e}\n---\n{yaml}"))
    }

    // --- item 3: network autoinstall config ---

    #[test]
    fn test_generate_user_data_network_netplan_v2() {
        let dir = tempfile::tempdir().unwrap();
        let m = make_machine(dir.path(), "192.168.50.38");
        let yaml = generate_user_data("host", &m, "auto-lvm", "en_US.UTF-8", "UTC").unwrap();
        let doc = parse_yaml(&yaml);
        let net = &doc["autoinstall"]["network"];
        assert_eq!(net["version"].as_u64(), Some(2));
        let en = &net["ethernets"]["en"];
        assert_eq!(en["match"]["name"], "en*");
        assert_eq!(en["dhcp4"].as_bool(), Some(true));
        assert!(!yaml.contains("# Static IP:"));
        assert!(!yaml.contains("late-commands if needed"));
    }

    // --- item 5: patch_grub_cfg & staging ---

    #[test]
    fn test_patch_grub_cfg_appends_to_all_linux_lines() {
        let input = "\
set timeout=5
menuentry \"Try or Install Ubuntu Server\" {
\tset gfxpayload=keep
\tlinux\t/casper/vmlinuz  ---
\tinitrd\t/casper/initrd
}
menuentry \"Ubuntu Server (HWE kernel)\" {
    linux   /casper/hwe-vmlinuz nomodeset ---  
    initrd  /casper/hwe-initrd
}
# linux /casper/commented-out
menuentry \"Linux Rescue\" {
    linux /casper/vmlinuz single
}
";
        let patched = patch_grub_cfg(input);
        let expected_arg = r"autoinstall ds=nocloud\;s=/cdrom/nocloud/";
        let lines: Vec<&str> = patched.lines().collect();
        assert_eq!(lines[3], format!("\tlinux\t/casper/vmlinuz  --- {expected_arg}"));
        assert_eq!(lines[7], format!("    linux   /casper/hwe-vmlinuz nomodeset --- {expected_arg}"));
        assert_eq!(lines[10], "# linux /casper/commented-out");
        assert_eq!(lines[11], "menuentry \"Linux Rescue\" {");
        assert_eq!(lines[12], format!("    linux /casper/vmlinuz single {expected_arg}"));
        assert!(patched.ends_with('\n'));
    }

    #[test]
    fn test_patch_grub_cfg_is_idempotent() {
        let input = "\
menuentry \"Install\" {
    linux /casper/vmlinuz ---
    initrd /casper/initrd
}
menuentry \"Already Patched\" {
    linux /casper/vmlinuz --- autoinstall ds=nocloud\\;s=/cdrom/nocloud/
    initrd /casper/initrd
}
";
        let once = patch_grub_cfg(input);
        let twice = patch_grub_cfg(&once);
        assert_eq!(once, twice);
        assert_eq!(once.matches("ds=nocloud").count(), 2);
        assert_eq!(once.matches("autoinstall").count(), 2);
    }

    #[test]
    fn test_patch_grub_cfg_empty_and_no_trailing_newline() {
        assert_eq!(patch_grub_cfg(""), "");
        assert_eq!(
            patch_grub_cfg("linux /casper/vmlinuz"),
            r"linux /casper/vmlinuz autoinstall ds=nocloud\;s=/cdrom/nocloud/"
        );
    }

    #[test]
    fn test_stage_iso_payload_patches_extracted_grub_cfg() {
        let dir = tempfile::tempdir().unwrap();
        let work = dir.path().join("work");
        let grub_dir = packed_iso_dir(&work).join("boot/grub");
        std::fs::create_dir_all(&grub_dir).unwrap();
        let grub_path = grub_dir.join("grub.cfg");
        std::fs::write(&grub_path, "menuentry \"Install\" {\n    linux /casper/vmlinuz ---\n}\n")
            .unwrap();
        // Extracted ISO files from xorriso may be read-only; staging must still patch them.
        let mut perms = std::fs::metadata(&grub_path).unwrap().permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&grub_path, perms).unwrap();

        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: \"1.0\"\nname: t\n").unwrap();
        let m = make_machine(dir.path(), "10.0.0.5");
        let ud = generate_user_data("host", &m, "auto-lvm", "en_US.UTF-8", "UTC").unwrap();

        stage_iso_payload(&work, &ud, &cfg).unwrap();

        let grub_out = std::fs::read_to_string(&grub_path).unwrap();
        assert!(grub_out.contains(r"autoinstall ds=nocloud\;s=/cdrom/nocloud/"));
        let staged_ud = std::fs::read_to_string(packed_iso_dir(&work).join("nocloud/user-data"))
            .unwrap();
        let doc = parse_yaml(&staged_ud);
        assert_eq!(doc["autoinstall"]["network"]["version"].as_u64(), Some(2));
    }

    // --- item 6: repack_iso_args ---

    #[test]
    fn test_repack_iso_args_replay_flags() {
        let args = repack_iso_args(
            Path::new("/tmp/ubuntu-24.04-live-server-amd64.iso"),
            Path::new("/tmp/work/iso"),
            Path::new("/tmp/out/forjar.iso"),
        );
        assert_eq!(
            args,
            vec![
                "-indev",
                "/tmp/ubuntu-24.04-live-server-amd64.iso",
                "-outdev",
                "/tmp/out/forjar.iso",
                "-boot_image",
                "any",
                "replay",
                "-map",
                "/tmp/work/iso",
                "/",
            ]
        );
        assert!(!args.iter().any(|a| a.contains("efi.img")));
        assert!(!args.iter().any(|a| a.contains("boot_hybrid.img")));
        assert!(!args.iter().any(|a| a.contains("eltorito.img")));
        assert!(!args.iter().any(|a| a == "mkisofs"));
    }

    #[test]
    fn test_repack_iso_args_preserves_paths_with_spaces() {
        let args = repack_iso_args(
            Path::new("/tmp/my isos/base.iso"),
            Path::new("/tmp/work dir/iso"),
            Path::new("/tmp/out dir/custom.iso"),
        );
        assert_eq!(args[1], "/tmp/my isos/base.iso");
        assert_eq!(args[3], "/tmp/out dir/custom.iso");
        assert_eq!(args[8], "/tmp/work dir/iso");
        assert_eq!(args[9], "/");
    }
}
