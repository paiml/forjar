//! Refs #208 partition P4 (artifact surfaces): image / plugin / dist / build.
//!
//! One test per confirmed defect, plus the non-regression guard that stops
//! "fixed" from meaning "does nothing". Each assertion marked RED fails on the
//! published 1.12.3 binary.

// ── #212: image --user-data must emit parseable cloud-init YAML ───────

mod user_data {
    use crate::cli::image_cmd::{firstboot_service_command, generate_user_data, indent_block};
    use crate::core::types::Machine;

    fn machine() -> Machine {
        Machine::ssh("sandbox-local", "127.0.0.1", "nobody")
    }

    #[test]
    fn generated_user_data_parses_as_yaml() {
        let ud = generate_user_data("local", &machine(), "auto-lvm", "en_US.UTF-8", "UTC").unwrap();
        // RED on 1.12.3: the systemd unit body sat at column 0 inside a `|`
        // block scalar, so the loader hit "could not find expected ':'".
        let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_str(&ud)
            .unwrap_or_else(|e| panic!("user-data is not valid YAML: {e}\n---\n{ud}"));
        assert!(parsed.get("autoinstall").is_some(), "no autoinstall key");
    }

    #[test]
    fn every_disk_layout_still_parses() {
        for disk in ["auto-lvm", "auto-zfs", "/dev/nvme0n1"] {
            let ud = generate_user_data("local", &machine(), disk, "en_US.UTF-8", "UTC").unwrap();
            serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&ud)
                .unwrap_or_else(|e| panic!("{disk}: {e}\n---\n{ud}"));
        }
    }

    #[test]
    fn heredoc_terminator_is_at_column_zero_after_yaml_strips_the_block() {
        let ud = generate_user_data("local", &machine(), "auto-lvm", "en_US.UTF-8", "UTC").unwrap();
        let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(&ud).unwrap();
        let cmds = doc["autoinstall"]["late-commands"]
            .as_sequence()
            .expect("late-commands");
        let script = cmds
            .iter()
            .filter_map(|c| c.as_str())
            .find(|c| c.contains("<<'UNIT'"))
            .expect("the firstboot heredoc command");

        // A `<<'UNIT'` heredoc only terminates on a line that is exactly UNIT.
        assert!(
            script.lines().any(|l| l == "UNIT"),
            "heredoc never terminates — the shell would swallow the rest:\n{script}"
        );
        assert!(
            script.lines().any(|l| l == "[Unit]"),
            "unit body lost its column-0 position:\n{script}"
        );
    }

    #[test]
    fn firstboot_body_is_indented_into_the_block_scalar() {
        let cmd = firstboot_service_command();
        for line in cmd.lines().filter(|l| !l.is_empty()) {
            assert!(
                line.starts_with("    "),
                "line escapes the YAML block: {line:?}"
            );
        }
    }

    #[test]
    fn indent_block_leaves_blank_lines_blank() {
        assert_eq!(indent_block("a\n\nb", 2), "  a\n\n  b");
        assert_eq!(indent_block("", 4), "");
    }

    // Non-regression: the generator still generates the machine's real values.
    #[test]
    fn hostname_and_user_still_reach_the_output() {
        let ud = generate_user_data("local", &machine(), "auto-lvm", "en_US.UTF-8", "UTC").unwrap();
        let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(&ud).unwrap();
        assert_eq!(doc["autoinstall"]["identity"]["hostname"], "sandbox-local");
        assert_eq!(doc["autoinstall"]["identity"]["username"], "nobody");
    }
}

// ── #211: dist -o must reach every generator ──────────────────────────

mod dist_output {
    use crate::cli::dist_output::resolve_dist_output;
    use std::path::Path;

    #[test]
    fn single_artifact_uses_output_as_the_file() {
        let t = resolve_dist_output(Some(Path::new("/x/MYNAME")), None, &[true, false]).unwrap();
        // RED on 1.12.3: only the installer branch ever saw `output`.
        assert_eq!(t.single_file(), Some(Path::new("/x/MYNAME")));
    }

    #[test]
    fn several_artifacts_make_output_a_directory() {
        let t = resolve_dist_output(Some(Path::new("/x/odir")), None, &[true, true, true]).unwrap();
        // RED on 1.12.3: `--all -o ./odir` wrote the INSTALLER to ./odir and
        // put the other six in ./dist — or exited 1 if ./odir was a directory.
        assert_eq!(t.single_file(), None);
        assert_eq!(t.dir(), Path::new("/x/odir"));
    }

    #[test]
    fn output_dir_still_wins_on_its_own() {
        let t = resolve_dist_output(None, Some(Path::new("/x/od2")), &[true, true]).unwrap();
        assert_eq!(t.dir(), Path::new("/x/od2"));
    }

    #[test]
    fn default_is_still_dist() {
        let t = resolve_dist_output(None, None, &[true]).unwrap();
        assert_eq!(t.dir(), Path::new("dist"));
    }

    #[test]
    fn output_and_output_dir_together_are_refused() {
        let err = resolve_dist_output(Some(Path::new("a")), Some(Path::new("b")), &[true])
            .unwrap_err();
        assert!(err.contains("--output-dir"), "unexpected error: {err}");
    }
}

// ── #213 / #211: plugin name and --output ─────────────────────────────

mod plugin_scaffold {
    use crate::cli::plugin::validate_plugin_name;

    #[test]
    fn empty_name_is_refused() {
        // RED on 1.12.3: `plugin init ""` wrote plugins/plugin.yaml, which
        // `plugin list` could not see but `plugin run ""` could run.
        assert!(validate_plugin_name("").is_err());
    }

    #[test]
    fn path_like_names_are_refused() {
        for bad in ["a/b", "..", ".", "a\\b"] {
            assert!(validate_plugin_name(bad).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn ordinary_names_are_accepted() {
        for good in ["good", "my-plugin", "plugin_2"] {
            assert!(validate_plugin_name(good).is_ok(), "rejected {good:?}");
        }
    }
}
