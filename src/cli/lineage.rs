//! FJ-1405: Merkle DAG configuration lineage.
//!
//! Builds a Merkle tree over the configuration DAG where each node's hash
//! incorporates its dependencies' hashes. This enables tamper-evident
//! verification of the entire dependency chain.

use super::helpers::*;
use crate::core::types;
use std::path::Path;

/// Compute Merkle hash for a node (incorporates dependency hashes).
fn merkle_hash(
    id: &str,
    resource: &types::Resource,
    dep_hashes: &std::collections::HashMap<String, String>,
) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(id.as_bytes());
    hasher.update(format!("{:?}", resource.resource_type).as_bytes());

    if let Some(ref c) = resource.content {
        hasher.update(c.as_bytes());
    }
    if let Some(ref p) = resource.path {
        hasher.update(p.as_bytes());
    }

    // Incorporate dependency hashes in sorted order (Merkle property)
    let mut deps: Vec<&String> = resource.depends_on.iter().collect();
    deps.sort();
    for dep in deps {
        if let Some(dep_hash) = dep_hashes.get(dep.as_str()) {
            hasher.update(dep_hash.as_bytes());
        }
    }

    hasher.finalize().to_hex()[..16].to_string()
}

/// Build in-degree map and forward adjacency list.
fn build_degree_map(
    config: &types::ForjarConfig,
) -> (
    std::collections::HashMap<String, usize>,
    std::collections::HashMap<String, Vec<String>>,
) {
    let mut in_deg: std::collections::HashMap<String, usize> =
        config.resources.keys().map(|k| (k.clone(), 0)).collect();
    let mut fwd: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (id, resource) in &config.resources {
        for dep in &resource.depends_on {
            if config.resources.contains_key(dep.as_str()) {
                *in_deg.entry(id.clone()).or_insert(0) += 1;
                fwd.entry(dep.clone()).or_default().push(id.clone());
            }
        }
    }
    (in_deg, fwd)
}

/// Topological sort via Kahn's algorithm.
fn topo_sort_ids(config: &types::ForjarConfig) -> Vec<String> {
    let (mut in_deg, fwd) = build_degree_map(config);
    let mut queue: std::collections::BTreeSet<String> = in_deg
        .iter()
        .filter(|(_, &d)| d == 0)
        .map(|(k, _)| k.clone())
        .collect();
    let mut order = Vec::new();
    while let Some(node) = queue.iter().next().cloned() {
        queue.remove(&node);
        order.push(node.clone());
        for child in fwd.get(&node).unwrap_or(&Vec::new()) {
            if let Some(deg) = in_deg.get_mut(child) {
                *deg -= 1;
                if *deg == 0 {
                    queue.insert(child.clone());
                }
            }
        }
    }
    order
}

/// Build Merkle DAG hashes in topological order.
fn build_merkle_dag(
    config: &types::ForjarConfig,
) -> Vec<(String, String, Vec<String>)> {
    let topo = topo_sort_ids(config);
    let mut hashes: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut result = Vec::new();
    for node in &topo {
        if let Some(resource) = config.resources.get(node) {
            let hash = merkle_hash(node, resource, &hashes);
            let deps = resource.depends_on.clone();
            hashes.insert(node.clone(), hash.clone());
            result.push((node.clone(), hash, deps));
        }
    }
    result
}

/// Compute the Merkle root hash (hash of all leaf hashes combined).
fn merkle_root(dag: &[(String, String, Vec<String>)]) -> String {
    let mut hasher = blake3::Hasher::new();
    for (_, hash, _) in dag {
        hasher.update(hash.as_bytes());
    }
    hasher.finalize().to_hex()[..16].to_string()
}

pub(crate) fn cmd_lineage(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let dag = build_merkle_dag(&config);
    let root = merkle_root(&dag);

    if json {
        print_lineage_json(&dag, &root, &config.name);
    } else {
        print_lineage_text(&dag, &root, &config.name);
    }

    Ok(())
}

fn print_lineage_json(
    dag: &[(String, String, Vec<String>)],
    root: &str,
    name: &str,
) {
    let nodes: Vec<String> = dag
        .iter()
        .map(|(id, hash, deps)| {
            let dep_list: Vec<String> = deps.iter().map(|d| format!("\"{d}\"")).collect();
            format!(
                r#"{{"id":"{}","merkle_hash":"{}","depends_on":[{}]}}"#,
                id,
                hash,
                dep_list.join(",")
            )
        })
        .collect();

    println!(
        r#"{{"name":"{}","merkle_root":"{}","nodes":[{}]}}"#,
        name,
        root,
        nodes.join(",")
    );
}

fn print_lineage_text(
    dag: &[(String, String, Vec<String>)],
    root: &str,
    name: &str,
) {
    println!("{}\n", bold("Merkle DAG Lineage"));
    println!("  Config:      {}", bold(name));
    println!("  Merkle root: {}", green(root));
    println!("  Nodes:       {}\n", dag.len());

    for (id, hash, deps) in dag {
        if deps.is_empty() {
            println!("  {} {} {}", green("*"), bold(id), dim(hash));
        } else {
            let dep_str = deps.join(", ");
            println!(
                "  {} {} {} <- [{}]",
                yellow("*"),
                bold(id),
                dim(hash),
                dep_str
            );
        }
    }

    println!(
        "\n  {} Any change to a node propagates through the Merkle tree",
        dim("Note:")
    );
}
