//! Tests: FJ-1394 sudo elevation in codegen dispatch.

#[cfg(test)]
mod tests {
    use crate::core::codegen;
    use crate::core::types::{Resource, ResourceType};

    fn file_resource(sudo: bool) -> Resource {
        Resource {
            resource_type: ResourceType::File,
            path: Some("/etc/test.conf".to_string()),
            content: Some("hello".to_string()),
            sudo,
            ..Default::default()
        }
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
        assert!(script.contains("sudo bash <<'FORJAR_SUDO'"));
        assert!(script.contains("if [ \"$(id -u)\" -eq 0 ]"));
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
        assert!(script.contains("sudo bash <<'FORJAR_SUDO'"));
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
        assert!(script.contains("sudo bash <<'FORJAR_SUDO'"));
    }

    #[test]
    fn test_fj1394_sudo_default_is_false() {
        let r = Resource::default();
        assert!(!r.sudo);
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
