//! FJ-1413: Dataset versioning and lineage tracking.
//!
//! Content-addressed dataset snapshots in store; lineage graph tracking
//! which transforms produced which outputs.

use super::helpers::*;
use crate::core::types;
use std::path::Path;

pub(crate) fn cmd_dataset_lineage(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let config_dir = file.parent().unwrap_or(Path::new("."));

    let mut datasets = Vec::new();
    let mut edges = Vec::new();

    // Collect data-related resources and their artifacts
    for (id, resource) in &config.resources {
        if !is_data_resource(resource) {
            continue;
        }

        let mut artifacts = Vec::new();

        // Source files
        if let Some(ref src) = resource.source {
            let src_path = config_dir.join(src);
            let hash = hash_file(&src_path);
            artifacts.push(ArtifactInfo {
                path: src.clone(),
                hash,
                artifact_type: "source".to_string(),
            });
        }

        // Output artifacts
        for art in &resource.output_artifacts {
            let art_path = config_dir.join(art);
            let hash = hash_file(&art_path);
            artifacts.push(ArtifactInfo {
                path: art.clone(),
                hash,
                artifact_type: "output".to_string(),
            });
        }

        datasets.push(DatasetNode {
            id: id.clone(),
            resource_type: format!("{:?}", resource.resource_type),
            tags: resource.tags.clone(),
            artifacts,
        });

        // Build dependency edges
        for dep in &resource.depends_on {
            edges.push((dep.clone(), id.clone()));
        }
    }

    // Compute lineage hash (Merkle-style)
    let lineage_hash = compute_lineage_hash(&datasets, &edges);

    if json {
        print_lineage_json(&datasets, &edges, &lineage_hash);
    } else {
        print_lineage_text(&datasets, &edges, &lineage_hash);
    }

    Ok(())
}

fn is_data_resource(resource: &types::Resource) -> bool {
    resource.tags.iter().any(|t| {
        t.contains("data")
            || t.contains("dataset")
            || t.contains("pipeline")
            || t.contains("transform")
            || t.contains("ml")
    }) || resource.resource_group.as_deref().map(|g| {
        g.contains("data") || g.contains("pipeline")
    }).unwrap_or(false)
        || !resource.output_artifacts.is_empty()
}

fn hash_file(path: &Path) -> Option<String> {
    std::fs::read(path)
        .ok()
        .map(|bytes| blake3::hash(&bytes).to_hex()[..16].to_string())
}

struct DatasetNode {
    id: String,
    resource_type: String,
    tags: Vec<String>,
    artifacts: Vec<ArtifactInfo>,
}

struct ArtifactInfo {
    path: String,
    hash: Option<String>,
    artifact_type: String,
}

fn compute_lineage_hash(datasets: &[DatasetNode], edges: &[(String, String)]) -> String {
    let mut hasher = blake3::Hasher::new();
    for ds in datasets {
        hasher.update(ds.id.as_bytes());
        for art in &ds.artifacts {
            hasher.update(art.path.as_bytes());
            if let Some(ref h) = art.hash {
                hasher.update(h.as_bytes());
            }
        }
    }
    for (from, to) in edges {
        hasher.update(from.as_bytes());
        hasher.update(to.as_bytes());
    }
    hasher.finalize().to_hex()[..16].to_string()
}

fn print_lineage_json(datasets: &[DatasetNode], edges: &[(String, String)], hash: &str) {
    let nodes: Vec<String> = datasets
        .iter()
        .map(|ds| {
            let arts: Vec<String> = ds
                .artifacts
                .iter()
                .map(|a| {
                    let h = a.hash.as_deref().unwrap_or("null");
                    format!(
                        r#"{{"path":"{p}","type":"{t}","hash":"{h}"}}"#,
                        p = a.path,
                        t = a.artifact_type,
                    )
                })
                .collect();
            let tags: Vec<String> = ds.tags.iter().map(|t| format!(r#""{t}""#)).collect();
            format!(
                r#"{{"id":"{id}","type":"{rt}","tags":[{tags}],"artifacts":[{arts}]}}"#,
                id = ds.id,
                rt = ds.resource_type,
                tags = tags.join(","),
                arts = arts.join(","),
            )
        })
        .collect();

    let edge_strs: Vec<String> = edges
        .iter()
        .map(|(f, t)| format!(r#"{{"from":"{f}","to":"{t}"}}"#))
        .collect();

    println!(
        r#"{{"lineage_hash":"{hash}","datasets":[{nodes}],"edges":[{edges}]}}"#,
        nodes = nodes.join(","),
        edges = edge_strs.join(","),
    );
}

fn print_lineage_text(datasets: &[DatasetNode], edges: &[(String, String)], hash: &str) {
    println!("{}\n", bold("Dataset Lineage"));
    println!("  Lineage hash: {}", green(hash));
    println!("  Datasets:     {}", datasets.len());
    println!("  Edges:        {}\n", edges.len());

    for ds in datasets {
        println!("  {} {} ({})", green("*"), bold(&ds.id), ds.resource_type);
        for art in &ds.artifacts {
            let h = art.hash.as_deref().unwrap_or("n/a");
            let icon = match art.artifact_type.as_str() {
                "source" => dim("S"),
                "output" => yellow("O"),
                _ => dim("?"),
            };
            println!("    {icon} {}: {} {}", art.artifact_type, art.path, dim(h));
        }
    }

    if !edges.is_empty() {
        println!("\n  Dependency edges:");
        for (from, to) in edges {
            println!("    {from} → {to}");
        }
    }

    if datasets.is_empty() {
        println!("  {} No data resources found (tag resources with 'data' or 'pipeline')", dim("(empty)"));
    }
}
