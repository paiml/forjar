//! FJ-1351: Contract coverage checker for kernel onboarding.
//!
//! Reads a `binding.yaml` registry and scans a contracts directory to determine
//! which kernel contracts are implemented, partial, or missing for a given
//! model architecture.

use super::hf_config::KernelRequirement;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// A single binding entry from `binding.yaml`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct BindingEntry {
    /// Contract name.
    pub contract: String,
    /// Mathematical equation reference.
    pub equation: String,
    /// Implementation status (implemented, partial, missing).
    pub status: String,
}

/// Top-level structure of a `binding.yaml` file.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct BindingRegistry {
    /// Registry schema version.
    pub version: String,
    /// Target crate for the bindings.
    pub target_crate: String,
    /// Binding entries.
    pub bindings: Vec<BindingEntry>,
}

/// Verification status of a kernel contract.
#[derive(Debug, Clone, PartialEq)]
pub enum ContractStatus {
    /// Contract fully implemented and available.
    Implemented,
    /// Contract partially implemented.
    Partial,
    /// Contract not yet implemented.
    Missing,
}

/// Coverage report for a model's kernel requirements.
#[derive(Debug, Clone)]
pub struct CoverageReport {
    /// Model architecture type.
    pub model_type: String,
    /// Total required contracts.
    pub total_required: usize,
    /// Number of covered contracts.
    pub covered: usize,
    /// Number of missing contracts.
    pub missing: usize,
    /// Coverage percentage (0-100).
    pub coverage_pct: f64,
    /// Per-contract status map.
    pub contracts: BTreeMap<String, ContractStatus>,
}

/// Parse a `binding.yaml` registry from a file path.
pub fn read_binding_registry(path: &Path) -> Result<BindingRegistry, String> {
    let data =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    serde_yaml_ng::from_str(&data).map_err(|e| format!("parse binding.yaml: {e}"))
}

/// Scan a contracts directory and return the list of contract names (stem of .yaml files).
pub fn scan_contracts_dir(dir: &Path) -> Result<Vec<String>, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("read dir {}: {e}", dir.display()))?;
    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let path = e.path();
            if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect();
    names.sort();
    Ok(names)
}

/// Compute a coverage report for a model's kernel requirements.
///
/// A contract is `Implemented` if listed in the binding registry with status
/// "implemented" AND a `.yaml` file exists in the contracts directory.
/// It is `Partial` if listed with status "partial". Otherwise `Missing`.
pub fn coverage_report(
    model_type: &str,
    required: &[KernelRequirement],
    registry: &BindingRegistry,
    available: &[String],
) -> CoverageReport {
    let binding_map: BTreeMap<&str, &str> = registry
        .bindings
        .iter()
        .map(|b| (b.contract.as_str(), b.status.as_str()))
        .collect();

    let mut contracts = BTreeMap::new();
    let mut covered = 0usize;

    for req in required {
        let status = match binding_map.get(req.contract.as_str()) {
            Some(&"implemented") if available.contains(&req.contract) => {
                covered += 1;
                ContractStatus::Implemented
            }
            Some(&"partial") => ContractStatus::Partial,
            _ => ContractStatus::Missing,
        };
        contracts.insert(req.contract.clone(), status);
    }

    let total = contracts.len();
    let missing = total - covered;
    let coverage_pct = if total == 0 {
        100.0
    } else {
        (covered as f64 / total as f64) * 100.0
    };

    CoverageReport {
        model_type: model_type.to_string(),
        total_required: total,
        covered,
        missing,
        coverage_pct,
        contracts,
    }
}
