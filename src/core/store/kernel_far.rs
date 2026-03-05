//! FJ-1353: Kernel contract FAR packaging and model onboarding pipeline.
//!
//! Packages verified kernel contracts into a FAR archive and orchestrates
//! the full onboard workflow: config parse → kernel mapping → coverage check
//! → scaffold missing → FAR encode.

use super::chunker::{chunk_directory, tree_hash};
use super::contract_coverage::{
    coverage_report, read_binding_registry, scan_contracts_dir, ContractStatus, CoverageReport,
};
use super::contract_scaffold::{scaffold_contracts, write_stubs};
use super::far::{encode_far, FarManifest, FarProvenance, KernelContractInfo};
use super::hf_config::{parse_hf_config, required_kernels, HfModelConfig};
use crate::tripwire::eventlog::now_iso8601;
use crate::tripwire::hasher::hash_directory;
use std::path::Path;

/// Result of the full onboarding pipeline.
#[derive(Debug)]
pub struct OnboardResult {
    /// Parsed HuggingFace model configuration.
    pub config: HfModelConfig,
    /// Kernel contract coverage report.
    pub coverage: CoverageReport,
    /// Filenames of newly scaffolded contract stubs.
    pub scaffolded: Vec<String>,
    /// FAR archive manifest.
    pub far_manifest: FarManifest,
}

/// Package a contracts directory into a FAR archive with kernel metadata.
///
/// Follows the `conda_to_far` pattern: hash_directory → chunk_directory →
/// tree_hash → build FarManifest → encode_far.
pub fn contracts_to_far(
    contracts_dir: &Path,
    config: &HfModelConfig,
    coverage: &CoverageReport,
    far_output: &Path,
) -> Result<FarManifest, String> {
    let store_hash = hash_directory(contracts_dir)?;
    let (chunks, file_entries) = chunk_directory(contracts_dir)?;
    let th = tree_hash(&chunks);
    let tree_hash_str = format!(
        "blake3:{}",
        th.iter().map(|b| format!("{b:02x}")).collect::<String>()
    );

    let total_size: u64 = file_entries.iter().map(|f| f.size).sum();
    let required_ops: Vec<String> = coverage.contracts.keys().cloned().collect();

    let manifest = FarManifest {
        name: format!("{}-kernel-contracts", config.model_type),
        version: "0.1.0".to_string(),
        arch: "noarch".to_string(),
        store_hash,
        tree_hash: tree_hash_str,
        file_count: file_entries.len() as u64,
        total_size,
        files: file_entries,
        provenance: FarProvenance {
            origin_provider: "kernel-contracts".to_string(),
            origin_ref: Some(config.model_type.clone()),
            origin_hash: None,
            created_at: now_iso8601(),
            generator: format!("forjar {}", env!("CARGO_PKG_VERSION")),
        },
        kernel_contracts: Some(KernelContractInfo {
            model_type: config.model_type.clone(),
            required_ops,
            coverage_pct: coverage.coverage_pct,
        }),
    };

    let chunk_pairs: Vec<([u8; 32], Vec<u8>)> =
        chunks.into_iter().map(|c| (c.hash, c.data)).collect();
    let file = std::fs::File::create(far_output)
        .map_err(|e| format!("create {}: {e}", far_output.display()))?;
    let writer = std::io::BufWriter::new(file);
    encode_far(&manifest, &chunk_pairs, writer)?;

    Ok(manifest)
}

/// Full model onboarding pipeline.
///
/// 1. Parse HuggingFace config.json
/// 2. Derive required kernel contracts
/// 3. Read binding registry + scan contracts directory
/// 4. Compute coverage report
/// 5. Scaffold missing contracts
/// 6. Package into FAR archive
pub fn onboard_model(
    config_path: &Path,
    contracts_dir: &Path,
    binding_path: &Path,
    output_dir: &Path,
) -> Result<OnboardResult, String> {
    // 1. Parse config
    let config = parse_hf_config(config_path)?;

    // 2. Required kernels
    let required = required_kernels(&config);

    // 3. Read bindings + scan contracts
    let registry = read_binding_registry(binding_path)?;
    let available = scan_contracts_dir(contracts_dir)?;

    // 4. Coverage report
    let coverage = coverage_report(&config.model_type, &required, &registry, &available);

    // 5. Scaffold missing contracts
    let missing: Vec<_> = required
        .iter()
        .filter(|r| {
            coverage
                .contracts
                .get(&r.contract)
                .is_none_or(|s| *s != ContractStatus::Implemented)
        })
        .cloned()
        .collect();
    let stubs = scaffold_contracts(&missing, "forjar-onboard");
    let scaffolded = write_stubs(&stubs, contracts_dir)?;

    // 6. Package FAR
    std::fs::create_dir_all(output_dir).map_err(|e| format!("create output dir: {e}"))?;
    let far_path = output_dir.join(format!("{}-kernel-contracts.far", config.model_type));

    // Re-scan after scaffolding for updated coverage
    let available_after = scan_contracts_dir(contracts_dir)?;
    let coverage_after =
        coverage_report(&config.model_type, &required, &registry, &available_after);

    let far_manifest = contracts_to_far(contracts_dir, &config, &coverage_after, &far_path)?;

    Ok(OnboardResult {
        config,
        coverage: coverage_after,
        scaffolded,
        far_manifest,
    })
}
