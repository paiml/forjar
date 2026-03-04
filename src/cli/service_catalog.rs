//! FJ-1427: Service catalog / self-service provisioning.
//!
//! Pre-approved infrastructure blueprints for non-IaC-expert consumers.
//! Catalogs are YAML files describing available services with parameters.

use std::path::Path;

/// A catalog entry describing an available service blueprint.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CatalogEntry {
    pub name: String,
    pub description: String,
    pub category: String,
    pub parameters: Vec<CatalogParam>,
    pub template_path: Option<String>,
    pub tags: Vec<String>,
    pub approved: bool,
}

/// A parameter for a catalog entry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CatalogParam {
    pub name: String,
    pub description: String,
    pub param_type: String,
    pub default: Option<String>,
    pub required: bool,
}

/// Service catalog.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Catalog {
    pub entries: Vec<CatalogEntry>,
}

/// Catalog report.
#[derive(Debug, serde::Serialize)]
pub struct CatalogReport {
    pub entries: Vec<CatalogEntry>,
    pub total: usize,
    pub approved: usize,
    pub categories: Vec<String>,
}

/// Load catalog from directory.
pub fn load_catalog(catalog_dir: &Path) -> Result<Catalog, String> {
    let catalog_file = catalog_dir.join("catalog.json");
    if !catalog_file.exists() {
        return Ok(Catalog::default());
    }
    let data = std::fs::read_to_string(&catalog_file)
        .map_err(|e| format!("read catalog: {e}"))?;
    serde_json::from_str(&data).map_err(|e| format!("parse catalog: {e}"))
}

/// Save catalog to directory.
#[allow(dead_code)]
pub fn save_catalog(catalog_dir: &Path, catalog: &Catalog) -> Result<(), String> {
    std::fs::create_dir_all(catalog_dir).map_err(|e| format!("mkdir: {e}"))?;
    let data = serde_json::to_string_pretty(catalog)
        .map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(catalog_dir.join("catalog.json"), data)
        .map_err(|e| format!("write catalog: {e}"))
}

/// Add a blueprint to the catalog.
#[allow(dead_code)]
pub fn add_entry(catalog: &mut Catalog, entry: CatalogEntry) {
    catalog.entries.push(entry);
}

/// List catalog entries.
pub fn cmd_catalog_list(
    catalog_dir: &Path,
    category: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let catalog = load_catalog(catalog_dir)?;
    let filtered = filter_entries(&catalog, category);
    let report = build_catalog_report(&filtered);

    if json {
        let out =
            serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {e}"))?;
        println!("{out}");
    } else {
        print_catalog_report(&report);
    }
    Ok(())
}

fn filter_entries(catalog: &Catalog, category: Option<&str>) -> Vec<CatalogEntry> {
    match category {
        Some(cat) => catalog
            .entries
            .iter()
            .filter(|e| e.category == cat)
            .cloned()
            .collect(),
        None => catalog.entries.clone(),
    }
}

fn build_catalog_report(entries: &[CatalogEntry]) -> CatalogReport {
    let mut categories: Vec<String> = entries
        .iter()
        .map(|e| e.category.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    categories.sort();

    CatalogReport {
        total: entries.len(),
        approved: entries.iter().filter(|e| e.approved).count(),
        categories,
        entries: entries.to_vec(),
    }
}

fn print_catalog_report(report: &CatalogReport) {
    println!("Service Catalog");
    println!("===============");
    println!(
        "Total: {} | Approved: {} | Categories: {}",
        report.total,
        report.approved,
        report.categories.join(", ")
    );
    println!();
    for e in &report.entries {
        let status = if e.approved { "approved" } else { "pending" };
        println!("  [{}] {} — {} ({})", status, e.name, e.description, e.category);
    }
}

/// Search catalog by name or tag.
#[allow(dead_code)]
pub fn search_catalog<'a>(catalog: &'a Catalog, query: &str) -> Vec<&'a CatalogEntry> {
    let q = query.to_lowercase();
    catalog
        .entries
        .iter()
        .filter(|e| {
            e.name.to_lowercase().contains(&q)
                || e.description.to_lowercase().contains(&q)
                || e.tags.iter().any(|t| t.to_lowercase().contains(&q))
        })
        .collect()
}
