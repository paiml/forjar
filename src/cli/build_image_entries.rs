//! FJ-2104: the `LayerEntry` sets of an image plan's layers (E13 file-type
//! splitting included). Split out of `build_image.rs`.

use crate::core::store::layer_builder::LayerEntry;
use crate::core::store::overlay_export;
use crate::core::types::{ForjarConfig, ImageBuildPlan, LayerStrategy};

/// Collect LayerEntry sets for each layer in the plan.
pub(super) fn collect_layer_entries(
    plan: &ImageBuildPlan,
    config: &ForjarConfig,
) -> Result<Vec<Vec<LayerEntry>>, String> {
    plan.layers
        .iter()
        .map(|strategy| collect_strategy_entries(strategy, config))
        .collect()
}

fn collect_strategy_entries(
    strategy: &LayerStrategy,
    config: &ForjarConfig,
) -> Result<Vec<LayerEntry>, String> {
    match strategy {
        LayerStrategy::Files { paths } => Ok(collect_file_entries(paths, config)),
        LayerStrategy::Packages { names } => {
            let content = names.join("\n");
            Ok(vec![LayerEntry::file(
                "var/lib/forjar/packages.list",
                content.as_bytes(),
                0o644,
            )])
        }
        LayerStrategy::Build {
            command: _,
            workdir,
        } => collect_build_entries(workdir.as_deref()),
        LayerStrategy::Derivation { store_path } => collect_derivation_entries(store_path),
    }
}

fn collect_file_entries(paths: &[String], config: &ForjarConfig) -> Vec<LayerEntry> {
    paths
        .iter()
        .map(|path| {
            if let Some(res) = config
                .resources
                .values()
                .find(|r| r.path.as_deref() == Some(path))
            {
                let content = res.content.as_deref().unwrap_or("").as_bytes();
                let mode = res
                    .mode
                    .as_deref()
                    .and_then(|m| u32::from_str_radix(m, 8).ok())
                    .unwrap_or(0o644);
                LayerEntry::file(path, content, mode)
            } else {
                LayerEntry::file(path, b"", 0o644)
            }
        })
        .collect()
}

/// FJ-2103: Scan overlay upper dir for Build layer strategy.
fn collect_build_entries(workdir: Option<&str>) -> Result<Vec<LayerEntry>, String> {
    let overlay_dir = workdir
        .map(std::path::Path::new)
        .unwrap_or_else(|| std::path::Path::new("/tmp/forjar-overlay"));
    if overlay_dir.exists() {
        let scan = overlay_export::scan_overlay_upper(overlay_dir, overlay_dir)
            .map_err(|e| format!("overlay scan: {e}"))?;
        Ok(overlay_export::merge_overlay_entries(&scan))
    } else {
        Ok(vec![])
    }
}

/// Scan derivation store path for layer entries.
fn collect_derivation_entries(store_path: &str) -> Result<Vec<LayerEntry>, String> {
    let p = std::path::Path::new(store_path);
    if p.exists() {
        let scan = overlay_export::scan_overlay_upper(p, p)
            .map_err(|e| format!("derivation scan: {e}"))?;
        Ok(scan.entries)
    } else {
        Ok(vec![])
    }
}

/// E13: Split file paths into config and app layers.
///
/// Config files (yaml, toml, json, conf, cfg, ini, env, properties)
/// go to a separate layer from application binaries. This provides
/// better cache reuse since configs change more frequently than binaries.
pub(super) fn split_paths_by_type(paths: &[String]) -> (Vec<String>, Vec<String>) {
    let config_exts = [
        ".yaml",
        ".yml",
        ".toml",
        ".json",
        ".conf",
        ".cfg",
        ".ini",
        ".env",
        ".properties",
    ];
    let mut config_paths = Vec::new();
    let mut app_paths = Vec::new();
    for path in paths {
        let lower = path.to_lowercase();
        if config_exts.iter().any(|ext| lower.ends_with(ext)) {
            config_paths.push(path.clone());
        } else {
            app_paths.push(path.clone());
        }
    }
    (config_paths, app_paths)
}
