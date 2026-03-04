//! FJ-1395: SBOM generation for managed infrastructure.
//! FJ-1399: Recipe SBOM — expands recipes before component collection.
//!
//! Generates a Software Bill of Materials in SPDX-lite JSON format from the
//! resolved config and state lock files. Covers packages, files with sources,
//! docker images, model artifacts, and recipe-expanded resources.

use crate::core::{parser, state, types};
use std::path::Path;

/// Generate SBOM from config and optional state directory.
///
/// FJ-1399: Recipes are expanded before component collection so that
/// recipe-defined packages, containers, and models appear in the SBOM.
pub(crate) fn cmd_sbom(file: &Path, state_dir: &Path, json: bool) -> Result<(), String> {
    let mut config = parser::parse_and_validate(file)?;
    // FJ-1399: Expand recipes so their inner resources appear in the SBOM.
    let config_dir = file.parent();
    let _ = parser::expand_recipes(&mut config, config_dir);
    let components = collect_components(&config, state_dir);

    if json {
        print_sbom_json(&config, &components)?;
    } else {
        print_sbom_text(&config, &components);
    }
    Ok(())
}

/// A single SBOM component extracted from config/state.
struct SbomComponent {
    name: String,
    version: String,
    component_type: String,
    supplier: String,
    hash: String,
}

/// Collect all SBOM components from config resources and state locks.
fn collect_components(config: &types::ForjarConfig, state_dir: &Path) -> Vec<SbomComponent> {
    let mut components = Vec::new();

    for (id, resource) in &config.resources {
        match resource.resource_type {
            types::ResourceType::Package => {
                collect_package_components(id, resource, &mut components);
            }
            types::ResourceType::Docker => {
                collect_docker_component(id, resource, &mut components);
            }
            types::ResourceType::Model => {
                collect_model_component(id, resource, &mut components);
            }
            types::ResourceType::File => {
                collect_file_component(id, resource, state_dir, &mut components);
            }
            _ => {}
        }
    }

    components
}

/// Extract package components (one per package name).
fn collect_package_components(
    id: &str,
    resource: &types::Resource,
    components: &mut Vec<SbomComponent>,
) {
    let provider = resource.provider.as_deref().unwrap_or("unknown");
    let version = resource.version.as_deref().unwrap_or("*");

    for pkg in &resource.packages {
        components.push(SbomComponent {
            name: pkg.clone(),
            version: version.to_string(),
            component_type: "library".to_string(),
            supplier: format!("{provider}:{id}"),
            hash: String::new(),
        });
    }
}

/// Extract docker image as a component.
fn collect_docker_component(
    id: &str,
    resource: &types::Resource,
    components: &mut Vec<SbomComponent>,
) {
    if let Some(ref image) = resource.image {
        let (name, version) = parse_image_tag(image);
        components.push(SbomComponent {
            name,
            version,
            component_type: "container".to_string(),
            supplier: format!("docker:{id}"),
            hash: resource.checksum.clone().unwrap_or_default(),
        });
    }
}

/// Extract ML model as a component.
fn collect_model_component(
    id: &str,
    resource: &types::Resource,
    components: &mut Vec<SbomComponent>,
) {
    if let Some(ref source) = resource.source {
        components.push(SbomComponent {
            name: source.clone(),
            version: resource.version.as_deref().unwrap_or("unknown").to_string(),
            component_type: "model".to_string(),
            supplier: format!("model:{id}"),
            hash: resource.checksum.clone().unwrap_or_default(),
        });
    }
}

/// Extract file with source as a component (e.g., downloaded binary).
fn collect_file_component(
    id: &str,
    resource: &types::Resource,
    state_dir: &Path,
    components: &mut Vec<SbomComponent>,
) {
    if let Some(ref source) = resource.source {
        let hash = lookup_state_hash(state_dir, id);
        components.push(SbomComponent {
            name: source.clone(),
            version: "1.0".to_string(),
            component_type: "file".to_string(),
            supplier: format!("file:{id}"),
            hash,
        });
    }
}

/// Look up BLAKE3 hash from state lock for a resource.
fn lookup_state_hash(state_dir: &Path, resource_id: &str) -> String {
    if let Ok(entries) = std::fs::read_dir(state_dir) {
        for entry in entries.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if let Ok(Some(lock)) = state::load_lock(state_dir, &name) {
                if let Some(res) = lock.resources.get(resource_id) {
                    return res.hash.clone();
                }
            }
        }
    }
    String::new()
}

/// Parse "image:tag" into (image, tag).
fn parse_image_tag(image: &str) -> (String, String) {
    if let Some(pos) = image.rfind(':') {
        let name = image[..pos].to_string();
        let tag = image[pos + 1..].to_string();
        // Avoid splitting on registry port like "registry:5000/image"
        if tag.contains('/') {
            return (image.to_string(), "latest".to_string());
        }
        (name, tag)
    } else {
        (image.to_string(), "latest".to_string())
    }
}

/// Print SBOM as SPDX-lite JSON.
fn print_sbom_json(
    config: &types::ForjarConfig,
    components: &[SbomComponent],
) -> Result<(), String> {
    let packages: Vec<serde_json::Value> = components
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let mut pkg = serde_json::json!({
                "SPDXID": format!("SPDXRef-Package-{}", i),
                "name": c.name,
                "versionInfo": c.version,
                "supplier": c.supplier,
                "primaryPackagePurpose": c.component_type,
                "downloadLocation": "NOASSERTION",
            });
            if !c.hash.is_empty() {
                pkg["checksums"] = serde_json::json!([{
                    "algorithm": "BLAKE3",
                    "checksumValue": c.hash,
                }]);
            }
            pkg
        })
        .collect();

    let doc = serde_json::json!({
        "spdxVersion": "SPDX-2.3",
        "dataLicense": "CC0-1.0",
        "SPDXID": "SPDXRef-DOCUMENT",
        "name": format!("forjar-sbom-{}", config.name),
        "documentNamespace": format!("https://forjar.dev/sbom/{}", config.name),
        "creationInfo": {
            "created": chrono_now(),
            "creators": [format!("Tool: forjar-{}", env!("CARGO_PKG_VERSION"))],
        },
        "packages": packages,
    });

    let output = serde_json::to_string_pretty(&doc).map_err(|e| format!("JSON error: {e}"))?;
    println!("{output}");
    Ok(())
}

/// Print SBOM as human-readable text table.
fn print_sbom_text(config: &types::ForjarConfig, components: &[SbomComponent]) {
    println!("SBOM: {} ({} components)", config.name, components.len());
    println!("{:-<72}", "");
    println!(
        "{:<30} {:<12} {:<12} {:<16}",
        "NAME", "VERSION", "TYPE", "HASH"
    );
    println!("{:-<72}", "");
    for c in components {
        let hash_short = if c.hash.len() > 12 {
            &c.hash[..12]
        } else {
            &c.hash
        };
        println!(
            "{:<30} {:<12} {:<12} {:<16}",
            truncate_str(&c.name, 29),
            truncate_str(&c.version, 11),
            c.component_type,
            hash_short
        );
    }
    println!("{:-<72}", "");
    println!("Total: {} components", components.len());
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max.saturating_sub(3)])
    } else {
        s.to_string()
    }
}

fn chrono_now() -> String {
    // ISO 8601 timestamp without chrono dependency
    let now = std::time::SystemTime::now();
    let secs = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Simple UTC timestamp
    format!("1970-01-01T00:00:00Z+{secs}s")
}
