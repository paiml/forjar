//! Tests: FJ-1394 sudo elevation in codegen dispatch.

// Refs #390-E and PMAT-158. The wrapper hands the script to `sudo bash` as a
// private temp file: not on stdin (#390-E — a stdin-reading command ate the
// rest of its own script) and not on descriptor 3 (PMAT-158 — sudo closes
// every descriptor >= 3 before exec, so `bash /dev/fd/3` found nothing and
// every `sudo: true` resource exited 127 for a non-root user). These assert
// the TEXT of the form. The text of the fd-3 form was asserted here too, and
// was green for the whole life of the defect, so the form is also EXECUTED
// under real sudo in `tests/falsification_sudo_transport_survives_closefrom.rs`.
#[cfg(test)]
mod tests {
    use crate::core::codegen;
    use crate::core::types::{Resource, ResourceType};

    /// The line that crosses the privilege boundary: a PATH argument, which
    /// survives sudo's `closefrom`, not a descriptor, which does not.
    const SUDO_LINE: &str = "sudo bash \"$forjar_sudo_script\"";

    fn file_resource(sudo: bool) -> Resource {
        Resource {
            resource_type: ResourceType::File,
            path: Some("/etc/test.conf".to_string()),
            content: Some("hello".to_string()),
            sudo,
            ..Default::default()
        }
    }

    /// Every textual property of the sudo form, in one place.
    fn assert_sudo_form(script: &str) {
        assert!(script.contains("if [ \"$(id -u)\" -eq 0 ]"), "{script}");
        assert!(script.contains(SUDO_LINE), "{script}");
        assert!(script.contains("mktemp"), "{script}");
        assert!(script.contains("<<'FORJAR_SUDO'"), "{script}");
        assert!(
            script.contains("rm -f \"$forjar_sudo_script\""),
            "the temp file must be removed:\n{script}"
        );
        assert!(
            !script.contains("/dev/fd/"),
            "sudo closes every fd >= 3 (closefrom); a /dev/fd transport runs nothing:\n{script}"
        );
    }

    #[test]
    fn test_fj1394_sudo_false_no_wrap() {
        let r = file_resource(false);
        let script = codegen::apply_script(&r).unwrap();
        assert!(!script.contains("sudo bash"));
    }

    #[test]
    fn test_fj1394_sudo_true_wraps_script() {
        let r = file_resource(true);
        let script = codegen::apply_script(&r).unwrap();
        assert_sudo_form(&script);
    }

    /// PMAT-158: the sudo transport is a temp file, on all three entry points.
    /// `sudo bash /dev/fd/3 3<<'D'` was the #390-E form; sudo's `closefrom`
    /// closed the descriptor and the elevated bash exited 127 without running
    /// a line — for apply, check and state_query alike.
    #[test]
    fn test_pmat158_sudo_transport_is_a_temp_file_not_fd3() {
        let r = file_resource(true);
        for (name, script) in [
            ("apply", codegen::apply_script(&r).unwrap()),
            ("check", codegen::check_script(&r).unwrap()),
            ("state_query", codegen::state_query_script(&r).unwrap()),
        ] {
            assert!(!script.contains("/dev/fd/"), "{name}:\n{script}");
            assert!(script.contains(SUDO_LINE), "{name}:\n{script}");
            assert!(
                script.contains("cat >\"$forjar_sudo_script\" <<'FORJAR_SUDO'"),
                "{name}: the script must be written into the temp file by a quoted heredoc:\n{script}"
            );
        }
    }

    #[test]
    fn test_fj1394_sudo_package_resource() {
        let r = Resource {
            resource_type: ResourceType::Package,
            provider: Some("apt".to_string()),
            packages: vec!["nginx".to_string()],
            sudo: true,
            ..Default::default()
        };
        let script = codegen::apply_script(&r).unwrap();
        assert_sudo_form(&script);
    }

    #[test]
    fn test_fj1394_sudo_service_resource() {
        let r = Resource {
            resource_type: ResourceType::Service,
            name: Some("nginx".to_string()),
            state: Some("running".to_string()),
            sudo: true,
            ..Default::default()
        };
        let script = codegen::apply_script(&r).unwrap();
        assert_sudo_form(&script);
    }

    #[test]
    fn test_fj1394_sudo_default_is_false() {
        let r = Resource::default();
        assert!(!r.sudo);
    }

    /// #349: the check ran unelevated while the apply ran as root, so a
    /// root-only path reported `missing:` for a file forjar had just written.
    #[test]
    fn test_fj1394_sudo_true_wraps_check_script() {
        let r = file_resource(true);
        let script = codegen::check_script(&r).unwrap();
        assert_sudo_form(&script);
    }

    /// #349: the state query is the half that writes `live_hash`/`observed`.
    #[test]
    fn test_fj1394_sudo_true_wraps_state_query_script() {
        let r = file_resource(true);
        let script = codegen::state_query_script(&r).unwrap();
        assert_sudo_form(&script);
    }

    /// The over-wrap guard: elevating unconditionally would demand sudo for
    /// every check on every host.
    #[test]
    fn test_fj1394_sudo_false_leaves_check_and_state_query_plain() {
        let r = file_resource(false);
        assert!(!codegen::check_script(&r).unwrap().contains("sudo bash"));
        assert!(!codegen::state_query_script(&r)
            .unwrap()
            .contains("sudo bash"));
    }

    #[test]
    fn test_fj1394_sudo_preserves_script_when_root() {
        let r = file_resource(true);
        let script = codegen::apply_script(&r).unwrap();
        // When root (id -u == 0), the original script runs without sudo
        assert!(script.contains("if [ \"$(id -u)\" -eq 0 ]; then"));
        assert!(script.contains("else"));
        assert!(script.contains("fi"));
    }
}
