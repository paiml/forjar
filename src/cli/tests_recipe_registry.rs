//! Tests: FJ-1426 recipe registry.

#![allow(unused_imports)]
use super::recipe_registry::*;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_empty_index() {
        let dir = tempfile::tempdir().unwrap();
        let index = load_index(dir.path()).unwrap();
        assert!(index.entries.is_empty());
    }

    #[test]
    fn test_save_and_load_index() {
        let dir = tempfile::tempdir().unwrap();
        let mut index = RegistryIndex::default();
        index.entries.push(RegistryEntry {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            path: "/tmp/test.yaml".to_string(),
            blake3: "a".repeat(64),
            description: "test recipe".to_string(),
            tags: vec!["web".to_string()],
        });
        save_index(dir.path(), &index).unwrap();
        let loaded = load_index(dir.path()).unwrap();
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].name, "test");
    }

    #[test]
    fn test_register_recipe() {
        let dir = tempfile::tempdir().unwrap();
        let registry = dir.path().join("registry");
        let recipe = dir.path().join("recipe.yaml");
        std::fs::write(&recipe, "version: \"1.0\"\nname: test\n").unwrap();

        let entry = register_recipe(
            &registry,
            &recipe,
            "1.0.0",
            "test recipe",
            &["web".to_string()],
        )
        .unwrap();
        assert_eq!(entry.name, "recipe");
        assert_eq!(entry.version, "1.0.0");
        assert_eq!(entry.blake3.len(), 64);
    }

    #[test]
    fn test_search_registry() {
        let index = RegistryIndex {
            entries: vec![
                RegistryEntry {
                    name: "nginx".to_string(),
                    version: "1.0.0".to_string(),
                    path: "/tmp/nginx.yaml".to_string(),
                    blake3: "a".repeat(64),
                    description: "nginx recipe".to_string(),
                    tags: vec!["web".to_string()],
                },
                RegistryEntry {
                    name: "postgres".to_string(),
                    version: "1.0.0".to_string(),
                    path: "/tmp/pg.yaml".to_string(),
                    blake3: "b".repeat(64),
                    description: "pg recipe".to_string(),
                    tags: vec!["database".to_string()],
                },
            ],
        };
        let results = search_registry(&index, "nginx");
        assert_eq!(results.len(), 1);
        let results = search_registry(&index, "web");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_get_latest() {
        let index = RegistryIndex {
            entries: vec![
                RegistryEntry {
                    name: "app".to_string(),
                    version: "1.0.0".to_string(),
                    path: "/tmp/a.yaml".to_string(),
                    blake3: "a".repeat(64),
                    description: "v1".to_string(),
                    tags: vec![],
                },
                RegistryEntry {
                    name: "app".to_string(),
                    version: "2.0.0".to_string(),
                    path: "/tmp/b.yaml".to_string(),
                    blake3: "b".repeat(64),
                    description: "v2".to_string(),
                    tags: vec![],
                },
            ],
        };
        let latest = get_latest(&index, "app").unwrap();
        assert_eq!(latest.version, "2.0.0");
    }

    #[test]
    fn test_cmd_registry_list() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_registry_list(dir.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_registry_entry_serde() {
        let entry = RegistryEntry {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            path: "/tmp/test.yaml".to_string(),
            blake3: "a".repeat(64),
            description: "test".to_string(),
            tags: vec!["web".to_string()],
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"name\":\"test\""));
        let round: RegistryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(round.name, "test");
    }
}
