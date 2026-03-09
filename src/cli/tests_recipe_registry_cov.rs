//! Coverage tests for recipe_registry.rs — registry operations.

use super::recipe_registry::*;

// ── load_index ──────────────────────────────────────────────────────

#[test]
fn load_index_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let index = load_index(dir.path()).unwrap();
    assert!(index.entries.is_empty());
}

#[test]
fn load_index_with_entries() {
    let dir = tempfile::tempdir().unwrap();
    let index = RegistryIndex {
        entries: vec![RegistryEntry {
            name: "test-recipe".to_string(),
            version: "1.0".to_string(),
            path: "/recipes/test.yaml".to_string(),
            blake3: "abc123".to_string(),
            description: "A test recipe".to_string(),
            tags: vec!["web".to_string()],
        }],
    };
    save_index(dir.path(), &index).unwrap();
    let loaded = load_index(dir.path()).unwrap();
    assert_eq!(loaded.entries.len(), 1);
    assert_eq!(loaded.entries[0].name, "test-recipe");
}

#[test]
fn load_index_corrupt_json() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("index.json"), "{{{invalid").unwrap();
    let result = load_index(dir.path());
    assert!(result.is_err());
}

// ── save_index ──────────────────────────────────────────────────────

#[test]
fn save_index_creates_dir() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("nested").join("registry");
    let index = RegistryIndex {
        entries: vec![],
    };
    save_index(&sub, &index).unwrap();
    assert!(sub.join("index.json").exists());
}

// ── register_recipe ─────────────────────────────────────────────────

#[test]
fn register_recipe_creates_entry() {
    let dir = tempfile::tempdir().unwrap();
    let recipe = dir.path().join("my-recipe.yaml");
    std::fs::write(&recipe, "name: test\nversion: 1.0\n").unwrap();

    let registry_dir = dir.path().join("registry");
    let entry = register_recipe(
        &registry_dir,
        &recipe,
        "1.0.0",
        "A test recipe",
        &["web".to_string(), "nginx".to_string()],
    )
    .unwrap();

    assert_eq!(entry.name, "my-recipe");
    assert_eq!(entry.version, "1.0.0");
    assert!(!entry.blake3.is_empty());

    let index = load_index(&registry_dir).unwrap();
    assert_eq!(index.entries.len(), 1);
}

#[test]
fn register_recipe_nonexistent_file() {
    let dir = tempfile::tempdir().unwrap();
    let result = register_recipe(
        dir.path(),
        std::path::Path::new("/nonexistent/recipe.yaml"),
        "1.0",
        "desc",
        &[],
    );
    assert!(result.is_err());
}

// ── search_registry ─────────────────────────────────────────────────

#[test]
fn search_by_name() {
    let index = RegistryIndex {
        entries: vec![
            RegistryEntry {
                name: "nginx-config".to_string(),
                version: "1.0".to_string(),
                path: "/r/nginx".to_string(),
                blake3: "aaa".to_string(),
                description: "Nginx setup".to_string(),
                tags: vec!["web".to_string()],
            },
            RegistryEntry {
                name: "postgres-config".to_string(),
                version: "1.0".to_string(),
                path: "/r/pg".to_string(),
                blake3: "bbb".to_string(),
                description: "PG setup".to_string(),
                tags: vec!["database".to_string()],
            },
        ],
    };
    let results = search_registry(&index, "nginx");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "nginx-config");
}

#[test]
fn search_by_tag() {
    let index = RegistryIndex {
        entries: vec![RegistryEntry {
            name: "app".to_string(),
            version: "1.0".to_string(),
            path: "/r/app".to_string(),
            blake3: "ccc".to_string(),
            description: "App".to_string(),
            tags: vec!["production".to_string()],
        }],
    };
    let results = search_registry(&index, "production");
    assert_eq!(results.len(), 1);
}

#[test]
fn search_no_match() {
    let index = RegistryIndex {
        entries: vec![RegistryEntry {
            name: "app".to_string(),
            version: "1.0".to_string(),
            path: "/r/app".to_string(),
            blake3: "ddd".to_string(),
            description: "App".to_string(),
            tags: vec![],
        }],
    };
    let results = search_registry(&index, "nonexistent");
    assert!(results.is_empty());
}

#[test]
fn search_case_insensitive() {
    let index = RegistryIndex {
        entries: vec![RegistryEntry {
            name: "NginX-Config".to_string(),
            version: "1.0".to_string(),
            path: "/r/nginx".to_string(),
            blake3: "eee".to_string(),
            description: "Nginx".to_string(),
            tags: vec![],
        }],
    };
    let results = search_registry(&index, "nginx");
    assert_eq!(results.len(), 1);
}

// ── get_latest ──────────────────────────────────────────────────────

#[test]
fn get_latest_single_version() {
    let index = RegistryIndex {
        entries: vec![RegistryEntry {
            name: "app".to_string(),
            version: "1.0".to_string(),
            path: "/r/app".to_string(),
            blake3: "fff".to_string(),
            description: "App".to_string(),
            tags: vec![],
        }],
    };
    let latest = get_latest(&index, "app").unwrap();
    assert_eq!(latest.version, "1.0");
}

#[test]
fn get_latest_multiple_versions() {
    let index = RegistryIndex {
        entries: vec![
            RegistryEntry {
                name: "app".to_string(),
                version: "1.0".to_string(),
                path: "/r/app/1.0".to_string(),
                blake3: "a".to_string(),
                description: "App v1".to_string(),
                tags: vec![],
            },
            RegistryEntry {
                name: "app".to_string(),
                version: "2.0".to_string(),
                path: "/r/app/2.0".to_string(),
                blake3: "b".to_string(),
                description: "App v2".to_string(),
                tags: vec![],
            },
        ],
    };
    let latest = get_latest(&index, "app").unwrap();
    assert_eq!(latest.version, "2.0");
}

#[test]
fn get_latest_not_found() {
    let index = RegistryIndex { entries: vec![] };
    assert!(get_latest(&index, "missing").is_none());
}

// ── cmd_registry_list ───────────────────────────────────────────────

#[test]
fn registry_list_empty() {
    let dir = tempfile::tempdir().unwrap();
    let result = cmd_registry_list(dir.path(), false);
    assert!(result.is_ok());
}

#[test]
fn registry_list_json() {
    let dir = tempfile::tempdir().unwrap();
    let result = cmd_registry_list(dir.path(), true);
    assert!(result.is_ok());
}

#[test]
fn registry_list_with_entries() {
    let dir = tempfile::tempdir().unwrap();
    let recipe = dir.path().join("test.yaml");
    std::fs::write(&recipe, "name: test\n").unwrap();
    let registry_dir = dir.path().join("registry");
    register_recipe(&registry_dir, &recipe, "1.0", "Test", &[]).unwrap();
    let result = cmd_registry_list(&registry_dir, false);
    assert!(result.is_ok());
}

#[test]
fn registry_list_json_with_entries() {
    let dir = tempfile::tempdir().unwrap();
    let recipe = dir.path().join("test.yaml");
    std::fs::write(&recipe, "name: test\n").unwrap();
    let registry_dir = dir.path().join("registry");
    register_recipe(&registry_dir, &recipe, "1.0", "Test", &[]).unwrap();
    let result = cmd_registry_list(&registry_dir, true);
    assert!(result.is_ok());
}

// ── default_registry_dir ────────────────────────────────────────────

#[test]
fn default_registry_dir_returns_path() {
    let dir = default_registry_dir();
    assert!(dir.to_string_lossy().contains("registry"));
}
