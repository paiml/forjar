//! #731: `forjar image` must produce an autoinstall that subiquity can run.
//!
//! The FJ-52 tests asserted that the YAML *contained strings*. These parse
//! the YAML the way subiquity reads it, and inspect a built ISO. Every test here
//! is red on the pre-#731 code.

#[cfg(test)]
mod tests {
    use crate::cli::image_cmd::*;
    use crate::core::types::Machine;
    use serde_yaml_ng::Value;
    use std::path::{Path, PathBuf};

    const KEY: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIFakeFakeFakeFakeFakeFakeFakeFakeFakeFakeFake op@example";

    fn keyed_machine(dir: &Path) -> Machine {
        let key = dir.join("id_test");
        std::fs::write(key.with_extension("pub"), format!("{KEY}\n")).unwrap();
        let mut m = Machine::ssh("box-01", "192.0.2.10", "op");
        m.ssh_key = Some(key.display().to_string());
        m
    }

    fn autoinstall(yaml: &str) -> Value {
        let doc: Value = serde_yaml_ng::from_str(yaml).expect("user-data must be valid YAML");
        doc["autoinstall"].clone()
    }

    // --- storage: the pin must be one subiquity honours ---

    #[test]
    fn test_731_serial_pin_is_a_layout_match_and_no_config() {
        let d = tempfile::tempdir().unwrap();
        let m = keyed_machine(d.path());
        let y =
            generate_user_data("box-01", &m, "serial:*S58SNMFX200010W", "C.UTF-8", "UTC").unwrap();
        let storage = &autoinstall(&y)["storage"];
        assert_eq!(storage["layout"]["name"], Value::from("lvm"));
        assert_eq!(
            storage["layout"]["match"]["serial"],
            Value::from("*S58SNMFX200010W")
        );
        // subiquity ignores storage.config when storage.layout is set: emitting
        // both is how the pin was silently dropped.
        assert!(
            storage.get("config").is_none(),
            "layout + config: config is ignored"
        );
    }

    #[test]
    fn test_731_kernel_path_pin_is_a_layout_match() {
        let d = tempfile::tempdir().unwrap();
        let m = keyed_machine(d.path());
        let y = generate_user_data("box-01", &m, "/dev/nvme0n1", "C.UTF-8", "UTC").unwrap();
        let storage = &autoinstall(&y)["storage"];
        assert_eq!(
            storage["layout"]["match"]["path"],
            Value::from("/dev/nvme0n1")
        );
        assert!(storage.get("config").is_none());
    }

    #[test]
    fn test_731_by_id_path_is_refused() {
        // subiquity matches kernel paths; a by-id link would match no disk.
        let d = tempfile::tempdir().unwrap();
        let m = keyed_machine(d.path());
        let err = generate_user_data(
            "box-01",
            &m,
            "/dev/disk/by-id/nvme-Some_SSD_S58SNMFX200010W",
            "C.UTF-8",
            "UTC",
        )
        .unwrap_err();
        assert!(
            err.contains("serial:"),
            "error must point at serial: — got {err}"
        );
    }

    // --- reachability: the installed box must be loggable-into ---

    #[test]
    fn test_731_no_ssh_key_is_an_error() {
        let m = Machine::ssh("box-01", "192.0.2.10", "op");
        let err = generate_user_data("box-01", &m, "auto-lvm", "C.UTF-8", "UTC").unwrap_err();
        assert!(err.contains("unreachable"), "got {err}");
    }

    #[test]
    fn test_731_missing_key_file_is_an_error() {
        let mut m = Machine::ssh("box-01", "192.0.2.10", "op");
        m.ssh_key = Some("/nonexistent/forjar-731/id_none".into());
        let err = read_ssh_pub_key(m.ssh_key.as_deref()).unwrap_err();
        assert!(
            err.contains("/nonexistent/forjar-731/id_none.pub"),
            "got {err}"
        );
    }

    #[test]
    fn test_731_identity_ssh_and_network_are_what_subiquity_needs() {
        let d = tempfile::tempdir().unwrap();
        let m = keyed_machine(d.path());
        let ai =
            autoinstall(&generate_user_data("box-01", &m, "auto-lvm", "C.UTF-8", "UTC").unwrap());
        // identity.password is required by subiquity's schema; locked, not empty.
        let pw = ai["identity"]["password"]
            .as_str()
            .expect("identity.password");
        assert!(
            pw.starts_with('!') || pw == "*",
            "password must be locked, got {pw}"
        );
        assert_eq!(ai["ssh"]["install-server"], Value::from(true));
        assert_eq!(ai["ssh"]["allow-pw"], Value::from(false));
        assert_eq!(ai["ssh"]["authorized-keys"][0], Value::from(KEY));
        let eth = &ai["network"]["ethernets"];
        let wired = eth
            .as_mapping()
            .expect("network.ethernets")
            .values()
            .next()
            .unwrap();
        assert_eq!(wired["match"]["name"], Value::from("en*"));
        assert_eq!(wired["dhcp4"], Value::from(true));
    }

    // --- the ISO: the seed must be inside it and the kernel told to use it ---

    fn have(cmd: &str) -> bool {
        std::process::Command::new(cmd)
            .arg("-version")
            .output()
            .is_ok()
    }

    fn fixture_iso(dir: &Path) -> PathBuf {
        let tree = dir.join("tree");
        std::fs::create_dir_all(tree.join("boot/grub")).unwrap();
        std::fs::create_dir_all(tree.join(".disk")).unwrap();
        std::fs::write(tree.join(".disk/info"), "fixture\n").unwrap();
        let cfg = "menuentry \"Try or Install\" {\n\tlinux\t/casper/vmlinuz  ---\n\tinitrd\t/casper/initrd\n}\n\
                   menuentry \"HWE\" {\n\tlinux\t/casper/hwe-vmlinuz  ---\n\tinitrd\t/casper/hwe-initrd\n}\n";
        std::fs::write(tree.join("boot/grub/grub.cfg"), cfg).unwrap();
        std::fs::write(tree.join("boot/grub/loopback.cfg"), cfg).unwrap();
        let iso = dir.join("base.iso");
        let ok = std::process::Command::new("xorriso")
            .args([
                "-outdev",
                iso.to_str().unwrap(),
                "-map",
                tree.to_str().unwrap(),
                "/",
            ])
            .output()
            .unwrap()
            .status
            .success();
        assert!(ok, "build fixture ISO");
        iso
    }

    fn iso_file(iso: &Path, path: &str, out: &Path) -> String {
        let dst = out.join(path.trim_start_matches('/').replace('/', "_"));
        let st = std::process::Command::new("xorriso")
            .args([
                "-osirrox",
                "on",
                "-indev",
                iso.to_str().unwrap(),
                "-extract",
                path,
            ])
            .arg(&dst)
            .output()
            .unwrap();
        assert!(st.status.success(), "{path} must be in the ISO");
        std::fs::read_to_string(dst).unwrap()
    }

    #[test]
    fn test_731_iso_carries_seed_and_autoinstall_cmdline() {
        if !have("xorriso") {
            eprintln!(
                "SKIP test_731_iso_carries_seed_and_autoinstall_cmdline: xorriso not installed"
            );
            return;
        }
        let d = tempfile::tempdir().unwrap();
        let base = fixture_iso(d.path());
        let key = d.path().join("id_test");
        std::fs::write(key.with_extension("pub"), format!("{KEY}\n")).unwrap();
        let cfg = d.path().join("forjar.yaml");
        std::fs::write(
            &cfg,
            format!(
                "version: \"1.0\"\nname: t\nmachines:\n  box-01:\n    hostname: box-01\n    addr: 192.0.2.10\n    user: op\n    ssh_key: {}\nresources: {{}}\n",
                key.display()
            ),
        )
        .unwrap();
        let out = d.path().join("out.iso");
        cmd_image_iso(
            &cfg,
            Some("box-01"),
            &base,
            &out,
            "serial:*X1",
            "C.UTF-8",
            "UTC",
            false,
        )
        .unwrap();

        let x = d.path().join("x");
        std::fs::create_dir_all(&x).unwrap();
        let ud = iso_file(&out, "/nocloud/user-data", &x);
        assert_eq!(
            autoinstall(&ud)["storage"]["layout"]["match"]["serial"],
            Value::from("*X1")
        );
        iso_file(&out, "/nocloud/meta-data", &x);
        assert!(iso_file(&out, "/forjar/forjar.yaml", &x).contains("box-01"));
        for cfgfile in ["/boot/grub/grub.cfg", "/boot/grub/loopback.cfg"] {
            let grub = iso_file(&out, cfgfile, &x);
            let linux: Vec<&str> = grub
                .lines()
                .filter(|l| l.trim_start().starts_with("linux"))
                .collect();
            assert_eq!(linux.len(), 2, "{cfgfile}: {grub}");
            for l in linux {
                // grub reads ';' as a command separator: the ds= value must be quoted.
                assert!(
                    l.contains(" autoinstall 'ds=nocloud;s=/cdrom/nocloud/'"),
                    "{cfgfile} linux line lacks the autoinstall cmdline: {l}"
                );
            }
        }
    }
}
