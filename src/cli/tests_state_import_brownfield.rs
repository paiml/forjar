//! Tests: FJ-1423 brownfield state import.

#![allow(unused_imports)]
use super::helpers::*;
use super::state_import_brownfield::*;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dpkg_packages() {
        let content =
            "Package: curl\nStatus: install ok installed\n\nPackage: vim\nStatus: install ok installed\n";
        let pkgs = parse_dpkg_packages(content);
        assert_eq!(pkgs, vec!["curl", "vim"]);
    }

    #[test]
    fn test_parse_dpkg_empty() {
        let pkgs = parse_dpkg_packages("");
        assert!(pkgs.is_empty());
    }

    #[test]
    fn test_generate_config_basic() {
        let resources = vec![DiscoveredResource {
            id: "pkg-curl".to_string(),
            resource_type: "package".to_string(),
            properties: {
                let mut m = std::collections::BTreeMap::new();
                m.insert("provider".to_string(), "apt".to_string());
                m
            },
            source: "dpkg".to_string(),
        }];
        let config = generate_config("local", &resources);
        assert!(config.contains("version: \"1.0\""));
        assert!(config.contains("pkg-curl:"));
        assert!(config.contains("type: package"));
        assert!(config.contains("packages: [curl]"));
    }

    #[test]
    fn test_generate_config_empty() {
        let config = generate_config("test-host", &[]);
        assert!(config.contains("name: imported-test-host"));
        assert!(config.contains("resources:"));
    }

    #[test]
    fn test_parse_resource_type() {
        use crate::core::types::ResourceType;
        assert!(matches!(
            parse_resource_type("package"),
            Some(ResourceType::Package)
        ));
        assert!(matches!(
            parse_resource_type("pkg"),
            Some(ResourceType::Package)
        ));
        assert!(matches!(
            parse_resource_type("service"),
            Some(ResourceType::Service)
        ));
        assert!(matches!(
            parse_resource_type("file"),
            Some(ResourceType::File)
        ));
        assert!(matches!(
            parse_resource_type("docker"),
            Some(ResourceType::Docker)
        ));
        assert!(parse_resource_type("unknown").is_none());
    }

    #[test]
    fn test_import_brownfield_basic() {
        let result = cmd_import_brownfield("localhost", &[], None, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_import_brownfield_json() {
        let result = cmd_import_brownfield("test-host", &["package".to_string()], None, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_import_brownfield_with_output() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("imported.yaml");
        let result = cmd_import_brownfield("local", &[], Some(&output), false);
        assert!(result.is_ok());
        assert!(output.exists());
    }

    #[test]
    fn test_discovered_resource_serde() {
        let r = DiscoveredResource {
            id: "pkg-curl".to_string(),
            resource_type: "package".to_string(),
            properties: std::collections::BTreeMap::new(),
            source: "dpkg".to_string(),
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("\"source\":\"dpkg\""));
    }
}
