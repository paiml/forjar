//! FJ-1307: Input closure tracking.
//!
//! Computes the transitive input closure for a resource — the full set of
//! inputs that contribute to its store hash. Identical closures produce
//! identical store paths (the determinism invariant).

use crate::tripwire::hasher::composite_hash;
use std::collections::{BTreeMap, BTreeSet};

/// A single resource's direct inputs (before transitive closure).
#[derive(Debug, Clone, Default)]
pub struct ResourceInputs {
    /// BLAKE3 hashes of this resource's direct inputs.
    pub input_hashes: Vec<String>,
    /// Names of resources this depends on (via `depends_on`).
    pub depends_on: Vec<String>,
}

/// Compute the transitive input closure for a named resource.
///
/// Walks the dependency graph (via `depends_on`) and collects all
/// input hashes reachable from the resource. Returns them sorted.
pub fn input_closure(resource: &str, graph: &BTreeMap<String, ResourceInputs>) -> Vec<String> {
    let mut visited = BTreeSet::new();
    let mut all_hashes = BTreeSet::new();
    collect_closure(resource, graph, &mut visited, &mut all_hashes);
    all_hashes.into_iter().collect()
}

/// Compute the closure hash — a single BLAKE3 hash over all transitive inputs.
///
/// This is the identity of the closure: identical closures → identical hashes.
///
/// Empty closures are hashed via a fixed sentinel component ("empty-closure-v1")
/// to satisfy the STRONG `composite_hash` precondition (`parts.len() > 0` AND no
/// empty parts) from `aprender-contracts blake3-state-v1`. The sentinel is
/// deterministic and distinct from any real closure, preserving identity.
pub fn closure_hash(closure: &[String]) -> String {
    if closure.is_empty() {
        // Sentinel for the empty-closure case: uphold STRONG blake3-state-v1
        // precondition (`parts.len() > 0` AND `!parts.iter().any(|p| p.is_empty())`).
        // Using a fixed marker ensures `closure_hash(&[])` is deterministic and
        // collision-free with real closures (no real hash starts with "sentinel:").
        return composite_hash(&["sentinel:empty-closure-v1"]);
    }
    let refs: Vec<&str> = closure.iter().map(|s| s.as_str()).collect();
    composite_hash(&refs)
}

/// Compute closures for all resources in a graph.
///
/// Returns a map from resource name to sorted closure hashes.
pub fn all_closures(graph: &BTreeMap<String, ResourceInputs>) -> BTreeMap<String, Vec<String>> {
    graph
        .keys()
        .map(|name| (name.clone(), input_closure(name, graph)))
        .collect()
}

fn collect_closure(
    name: &str,
    graph: &BTreeMap<String, ResourceInputs>,
    visited: &mut BTreeSet<String>,
    hashes: &mut BTreeSet<String>,
) {
    if !visited.insert(name.to_string()) {
        return; // Already visited (cycle protection)
    }
    if let Some(inputs) = graph.get(name) {
        for h in &inputs.input_hashes {
            hashes.insert(h.clone());
        }
        for dep in &inputs.depends_on {
            collect_closure(dep, graph, visited, hashes);
        }
    }
}
