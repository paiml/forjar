//! FJ-1341–FJ-1344: Store derivation model.
//!
//! A derivation takes store entries as inputs, applies a transformation
//! inside a pepita sandbox, and produces a new store entry. This is the
//! "import once, own forever" model — the universal adapter.

use super::purity::PurityLevel;
use super::sandbox::SandboxConfig;
use crate::tripwire::hasher::composite_hash;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A derivation input — either a store hash or a reference to another resource.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DerivationInput {
    /// Direct store hash reference
    Store {
        /// Content-addressed store hash.
        store: String,
    },
    /// Reference to another resource's output
    Resource {
        /// Resource identifier to resolve.
        resource: String,
    },
}

/// A store derivation definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Derivation {
    /// Named inputs (available as $inputs/<name> inside sandbox)
    pub inputs: BTreeMap<String, DerivationInput>,

    /// Build script (bashrs-purified POSIX shell)
    pub script: String,

    /// Sandbox configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<SandboxConfig>,

    /// Target architecture
    #[serde(default = "default_arch")]
    pub arch: String,

    /// Output path variable (default: $out)
    #[serde(default = "default_out_var")]
    pub out_var: String,
}

fn default_arch() -> String {
    "x86_64".to_string()
}

fn default_out_var() -> String {
    "$out".to_string()
}

/// Result of evaluating a derivation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DerivationResult {
    /// Store hash of the produced artifact
    pub store_hash: String,

    /// Store path for the output
    pub store_path: String,

    /// All input store hashes that contributed
    pub input_closure: Vec<String>,

    /// Closure hash (composite of all inputs + script + arch)
    pub closure_hash: String,

    /// Derivation depth (max of input depths + 1)
    pub derivation_depth: u32,
}

/// Compute the closure hash for a derivation.
///
/// Hash = composite_hash(sorted input hashes + script hash + arch)
pub fn derivation_closure_hash(
    derivation: &Derivation,
    input_hashes: &BTreeMap<String, String>,
) -> String {
    let script_hash = format!(
        "script:{}",
        blake3::hash(derivation.script.as_bytes()).to_hex()
    );
    let mut components: Vec<&str> = input_hashes.values().map(|s| s.as_str()).collect();
    components.sort();
    components.push(&script_hash);
    components.push(&derivation.arch);
    composite_hash(&components)
}

/// Collect all store hashes from derivation inputs.
pub fn collect_input_hashes(
    derivation: &Derivation,
    resolved_resources: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, String> {
    let mut result = BTreeMap::new();

    for (name, input) in &derivation.inputs {
        let hash = match input {
            DerivationInput::Store { store } => store.clone(),
            DerivationInput::Resource { resource } => resolved_resources
                .get(resource)
                .cloned()
                .ok_or_else(|| format!("unresolved resource input: {resource}"))?,
        };
        result.insert(name.clone(), hash);
    }

    Ok(result)
}

/// Validate a derivation definition.
pub fn validate_derivation(derivation: &Derivation) -> Vec<String> {
    let mut errors = Vec::new();

    if derivation.inputs.is_empty() {
        errors.push("derivation must have at least one input".to_string());
    }
    if derivation.script.trim().is_empty() {
        errors.push("derivation script cannot be empty".to_string());
    }
    if derivation.arch.is_empty() {
        errors.push("derivation arch cannot be empty".to_string());
    }

    for (name, input) in &derivation.inputs {
        match input {
            DerivationInput::Store { store } => {
                if store.is_empty() {
                    errors.push(format!("input '{name}': store hash cannot be empty"));
                }
            }
            DerivationInput::Resource { resource } => {
                if resource.is_empty() {
                    errors.push(format!("input '{name}': resource name cannot be empty"));
                }
            }
        }
    }

    errors
}

/// Validate a derivation DAG (no cycles).
///
/// Each entry maps a derivation name to its dependency names.
pub fn validate_dag(graph: &BTreeMap<String, Vec<String>>) -> Result<Vec<String>, String> {
    let mut visited = BTreeSet::new();
    let mut in_stack = BTreeSet::new();
    let mut order = Vec::new();

    for name in graph.keys() {
        if !visited.contains(name) {
            dag_dfs(name, graph, &mut visited, &mut in_stack, &mut order)?;
        }
    }

    Ok(order)
}

fn dag_dfs(
    node: &str,
    graph: &BTreeMap<String, Vec<String>>,
    visited: &mut BTreeSet<String>,
    in_stack: &mut BTreeSet<String>,
    order: &mut Vec<String>,
) -> Result<(), String> {
    if in_stack.contains(node) {
        return Err(format!("cycle detected at derivation: {node}"));
    }
    if visited.contains(node) {
        return Ok(());
    }

    in_stack.insert(node.to_string());

    if let Some(deps) = graph.get(node) {
        for dep in deps {
            dag_dfs(dep, graph, visited, in_stack, order)?;
        }
    }

    in_stack.remove(node);
    visited.insert(node.to_string());
    order.push(node.to_string());
    Ok(())
}

/// Classify a derivation's purity level from its sandbox config.
pub fn derivation_purity(derivation: &Derivation) -> PurityLevel {
    match &derivation.sandbox {
        Some(cfg) => match cfg.level {
            super::sandbox::SandboxLevel::Full => PurityLevel::Pure,
            super::sandbox::SandboxLevel::NetworkOnly => PurityLevel::Pinned,
            super::sandbox::SandboxLevel::Minimal => PurityLevel::Constrained,
            super::sandbox::SandboxLevel::None => PurityLevel::Impure,
        },
        None => PurityLevel::Impure,
    }
}

/// Parse a derivation from YAML string.
pub fn parse_derivation(yaml: &str) -> Result<Derivation, String> {
    serde_yaml_ng::from_str(yaml).map_err(|e| format!("invalid derivation: {e}"))
}

/// Compute derivation depth from input depths.
pub fn compute_depth(input_depths: &[u32]) -> u32 {
    input_depths.iter().max().copied().unwrap_or(0) + 1
}
