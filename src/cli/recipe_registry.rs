//! FJ-1426: Versioned recipe registry.
//!
//! Local recipe registry for discovery, versioning, and dependency resolution.
//! Recipes are indexed by name + version with BLAKE3 integrity verification.

use std::path::{Path, PathBuf};

/// A registered recipe entry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RegistryEntry {
    pub name: String,
    pub version: String,
    pub path: String,
    pub blake3: String,
    pub description: String,
    pub tags: Vec<String>,
}

/// Registry index.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RegistryIndex {
    pub entries: Vec<RegistryEntry>,
}

/// Registry report.
#[derive(Debug, serde::Serialize)]
pub struct RegistryReport {
    pub registry_dir: String,
    pub entries: Vec<RegistryEntry>,
    pub total: usize,
}

/// Initialize or load a registry index.
pub fn load_index(registry_dir: &Path) -> Result<RegistryIndex, String> {
    let index_path = registry_dir.join("index.json");
    if !index_path.exists() {
        return Ok(RegistryIndex::default());
    }
    let data =
        std::fs::read_to_string(&index_path).map_err(|e| format!("read index: {e}"))?;
    serde_json::from_str(&data).map_err(|e| format!("parse index: {e}"))
}

/// Save registry index.
pub fn save_index(registry_dir: &Path, index: &RegistryIndex) -> Result<(), String> {
    std::fs::create_dir_all(registry_dir)
        .map_err(|e| format!("mkdir registry: {e}"))?;
    let data =
        serde_json::to_string_pretty(index).map_err(|e| format!("serialize index: {e}"))?;
    std::fs::write(registry_dir.join("index.json"), data)
        .map_err(|e| format!("write index: {e}"))
}

/// Register a recipe file into the registry.
pub fn register_recipe(
    registry_dir: &Path,
    recipe_path: &Path,
    version: &str,
    description: &str,
    tags: &[String],
) -> Result<RegistryEntry, String> {
    let content =
        std::fs::read(recipe_path).map_err(|e| format!("read recipe: {e}"))?;
    let blake3 = hash_blake3(&content);
    let name = recipe_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let dest_dir = registry_dir.join(&name).join(version);
    std::fs::create_dir_all(&dest_dir).map_err(|e| format!("mkdir: {e}"))?;
    let dest = dest_dir.join(recipe_path.file_name().unwrap_or_default());
    std::fs::copy(recipe_path, &dest).map_err(|e| format!("copy recipe: {e}"))?;

    let entry = RegistryEntry {
        name,
        version: version.to_string(),
        path: dest.display().to_string(),
        blake3,
        description: description.to_string(),
        tags: tags.to_vec(),
    };

    let mut index = load_index(registry_dir)?;
    index.entries.push(entry.clone());
    save_index(registry_dir, &index)?;

    Ok(entry)
}

fn hash_blake3(data: &[u8]) -> String {
    let hash = blake3::hash(data);
    hash.to_hex().to_string()
}

/// List registry contents.
pub fn cmd_registry_list(
    registry_dir: &Path,
    json: bool,
) -> Result<(), String> {
    let index = load_index(registry_dir)?;

    let report = RegistryReport {
        registry_dir: registry_dir.display().to_string(),
        entries: index.entries.clone(),
        total: index.entries.len(),
    };

    if json {
        let out =
            serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {e}"))?;
        println!("{out}");
    } else {
        print_registry_report(&report);
    }
    Ok(())
}

fn print_registry_report(report: &RegistryReport) {
    println!("Recipe Registry: {}", report.registry_dir);
    println!("Entries ({}):", report.total);
    for e in &report.entries {
        println!("  {} v{} [{}]", e.name, e.version, e.blake3.get(..8).unwrap_or(""));
    }
}

/// Search registry by name pattern.
pub fn search_registry<'a>(index: &'a RegistryIndex, pattern: &str) -> Vec<&'a RegistryEntry> {
    let pat = pattern.to_lowercase();
    index
        .entries
        .iter()
        .filter(|e| e.name.to_lowercase().contains(&pat) || e.tags.iter().any(|t| t.to_lowercase().contains(&pat)))
        .collect()
}

/// Get the latest version of a recipe by name.
pub fn get_latest<'a>(index: &'a RegistryIndex, name: &str) -> Option<&'a RegistryEntry> {
    index
        .entries
        .iter()
        .filter(|e| e.name == name)
        .max_by(|a, b| a.version.cmp(&b.version))
}

/// Get the default registry directory.
pub fn default_registry_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/var/lib"))
        .join(".forjar")
        .join("registry")
}
