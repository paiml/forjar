//! Tests: FJ-1427 service catalog.

#![allow(unused_imports)]
use super::service_catalog::*;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_empty_catalog() {
        let dir = tempfile::tempdir().unwrap();
        let catalog = load_catalog(dir.path()).unwrap();
        assert!(catalog.entries.is_empty());
    }

    #[test]
    fn test_save_and_load_catalog() {
        let dir = tempfile::tempdir().unwrap();
        let mut catalog = Catalog::default();
        add_entry(
            &mut catalog,
            CatalogEntry {
                name: "web-app".to_string(),
                description: "Standard web application".to_string(),
                category: "web".to_string(),
                parameters: vec![CatalogParam {
                    name: "port".to_string(),
                    description: "HTTP port".to_string(),
                    param_type: "integer".to_string(),
                    default: Some("8080".to_string()),
                    required: false,
                }],
                template_path: None,
                tags: vec!["web".to_string()],
                approved: true,
            },
        );
        save_catalog(dir.path(), &catalog).unwrap();
        let loaded = load_catalog(dir.path()).unwrap();
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].name, "web-app");
    }

    #[test]
    fn test_search_catalog() {
        let catalog = Catalog {
            entries: vec![
                CatalogEntry {
                    name: "nginx".to_string(),
                    description: "nginx web server".to_string(),
                    category: "web".to_string(),
                    parameters: vec![],
                    template_path: None,
                    tags: vec!["web".to_string(), "proxy".to_string()],
                    approved: true,
                },
                CatalogEntry {
                    name: "postgres".to_string(),
                    description: "PostgreSQL database".to_string(),
                    category: "database".to_string(),
                    parameters: vec![],
                    template_path: None,
                    tags: vec!["db".to_string()],
                    approved: true,
                },
            ],
        };
        assert_eq!(search_catalog(&catalog, "nginx").len(), 1);
        assert_eq!(search_catalog(&catalog, "proxy").len(), 1);
        assert_eq!(search_catalog(&catalog, "database").len(), 1);
    }

    #[test]
    fn test_cmd_catalog_list() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_catalog_list(dir.path(), None, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_catalog_list_filtered() {
        let dir = tempfile::tempdir().unwrap();
        let mut catalog = Catalog::default();
        add_entry(
            &mut catalog,
            CatalogEntry {
                name: "app".to_string(),
                description: "app".to_string(),
                category: "web".to_string(),
                parameters: vec![],
                template_path: None,
                tags: vec![],
                approved: true,
            },
        );
        save_catalog(dir.path(), &catalog).unwrap();
        let result = cmd_catalog_list(dir.path(), Some("web"), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_catalog_entry_serde() {
        let entry = CatalogEntry {
            name: "test".to_string(),
            description: "test".to_string(),
            category: "web".to_string(),
            parameters: vec![],
            template_path: None,
            tags: vec![],
            approved: true,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"approved\":true"));
        let round: CatalogEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(round.name, "test");
    }

    #[test]
    fn test_catalog_param_serde() {
        let param = CatalogParam {
            name: "port".to_string(),
            description: "HTTP port".to_string(),
            param_type: "integer".to_string(),
            default: Some("8080".to_string()),
            required: true,
        };
        let json = serde_json::to_string(&param).unwrap();
        assert!(json.contains("\"required\":true"));
    }
}
