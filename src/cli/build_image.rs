//! FJ-2104: Build container image from resource definitions.
//!
//! Wires `assemble_image()` into the `forjar build` CLI command,
//! converting resource definitions into `ImageBuildPlan` + `LayerEntry` sets.

use crate::core::store::layer_builder::LayerEntry;
use crate::core::store::overlay_export;
use crate::core::types::{
    ForjarConfig, ImageBuildMetrics, ImageBuildPlan, LayerMetric, LayerStrategy, OciLayerConfig,
    Resource,
};

/// FJ-2104: Build container image from a resource definition.
#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_build(
    file: &std::path::Path,
    resource: &str,
    load: bool,
    push: bool,
    far: bool,
    sandbox: bool,
    json: bool,
) -> Result<(), String> {
    // GH-91: Warn that --json is not yet implemented for build
    if json {
        eprintln!("Warning: --json is not yet implemented for build output. Flag ignored.");
    }
    let config = super::helpers::parse_and_validate(file)?;
    let res = config
        .resources
        .get(resource)
        .ok_or_else(|| format!("resource '{resource}' not found"))?;
    if !matches!(res.resource_type, crate::core::types::ResourceType::Image) {
        return Err(format!("resource '{resource}' is not type: image"));
    }

    let plan = build_plan_from_resource(resource, res, &config)?;
    let output_dir = std::path::Path::new("state/images").join(resource);
    std::fs::create_dir_all(&output_dir).map_err(|e| format!("create output dir: {e}"))?;

    if sandbox {
        return cmd_build_sandbox(resource, &plan, &config, &output_dir, load, push, far);
    }

    let layer_entries = collect_layer_entries(&plan, &config)?;

    // FJ-2403/E16: Check build cache — skip rebuild if inputs unchanged.
    let input_hash = compute_layer_input_hash(&layer_entries);
    if let Some(cached) = check_build_cache(&output_dir, &input_hash) {
        println!("\nBuilding {resource} ({}) — CACHED", plan.tag);
        println!("  {cached}");
        println!("  Input hash: {input_hash}");
        if load {
            cmd_build_load(&output_dir)?;
        }
        if push {
            cmd_build_push(res, &output_dir)?;
        }
        if far {
            cmd_build_far(resource, &output_dir)?;
        }
        return Ok(());
    }

    let start = std::time::Instant::now();
    let result = crate::core::store::image_assembler::assemble_image(
        &plan,
        &layer_entries,
        &output_dir,
        &OciLayerConfig::default(),
        None, // E12: default to host architecture
    )?;
    let duration = start.elapsed();

    println!("\nBuilding {resource} ({})", plan.tag);
    for (i, layer) in result.layers.iter().enumerate() {
        println!(
            "  Layer {}/{}: {} files, {} -> {} bytes",
            i + 1,
            result.layers.len(),
            layer.file_count,
            layer.uncompressed_size,
            layer.compressed_size
        );
    }
    println!(
        "\n  Image: {} ({} layers, {} bytes)",
        plan.tag,
        result.layers.len(),
        result.total_size
    );
    println!("  Layout: {}", output_dir.display());
    println!("  Built in {:.1}s", duration.as_secs_f64());

    // FJ-2403/E17: Collect and persist image build metrics.
    let metrics = ImageBuildMetrics {
        tag: plan.tag.clone(),
        layer_count: result.layers.len(),
        total_size: result.total_size,
        layers: result
            .layers
            .iter()
            .map(|l| LayerMetric {
                file_count: l.file_count,
                uncompressed_size: l.uncompressed_size,
                compressed_size: l.compressed_size,
            })
            .collect(),
        duration_secs: duration.as_secs_f64(),
        built_at: crate::tripwire::eventlog::now_iso8601(),
        forjar_version: env!("CARGO_PKG_VERSION").to_string(),
        target_arch: std::env::consts::ARCH.to_string(),
    };
    if let Err(e) = metrics.write_to(&output_dir) {
        eprintln!("  warning: {e}");
    }
    write_build_cache(&output_dir, &input_hash);

    if load {
        cmd_build_load(&output_dir)?;
    }
    if push {
        cmd_build_push(res, &output_dir)?;
    }
    if far {
        cmd_build_far(resource, &output_dir)?;
    }
    Ok(())
}

/// FJ-2103: Build image inside container sandbox (Docker/Podman).
#[allow(clippy::too_many_arguments)]
fn cmd_build_sandbox(
    resource: &str,
    plan: &ImageBuildPlan,
    config: &ForjarConfig,
    output_dir: &std::path::Path,
    load: bool,
    push: bool,
    far: bool,
) -> Result<(), String> {
    use crate::core::store::container_build;

    // Generate apply scripts from non-image resources in the config
    let apply_scripts: Vec<String> = config
        .resources
        .iter()
        .filter(|(_, r)| !matches!(r.resource_type, crate::core::types::ResourceType::Image))
        .filter_map(|(_, r)| {
            let resolved = crate::core::resolver::resolve_resource_templates(
                r,
                &config.params,
                &config.machines,
            )
            .ok()?;
            crate::core::codegen::apply_script(&resolved).ok()
        })
        .collect();

    println!("\nBuilding {resource} ({}) via container sandbox", plan.tag);
    println!("  Scripts: {}", apply_scripts.len());

    let result = container_build::build_image_in_container(plan, &apply_scripts, output_dir)?;

    println!("  {}", container_build::format_container_build(&result));
    println!("  Layout: {}", output_dir.display());

    let res = config.resources.get(resource);
    if load {
        cmd_build_load(output_dir)?;
    }
    if push {
        if let Some(r) = res {
            cmd_build_push(r, output_dir)?;
        }
    }
    if far {
        cmd_build_far(resource, output_dir)?;
    }
    Ok(())
}

/// Build an ImageBuildPlan from a resource definition.
fn build_plan_from_resource(
    name: &str,
    res: &Resource,
    config: &ForjarConfig,
) -> Result<ImageBuildPlan, String> {
    // GH-91: config not yet used for build plan customization
    let _ = config;
    let tag = res.version.as_deref().unwrap_or("latest");
    let image_name = res.name.as_deref().unwrap_or(name);

    // Check for base image layers
    let mut layers = Vec::new();
    if let Some(ref base) = res.image {
        let base_dir = std::path::Path::new("state/images").join(base.replace([':', '/'], "_"));
        if base_dir.exists() {
            if let Ok(base_layers) = crate::core::store::base_image::extract_base_layers(&base_dir)
            {
                println!(
                    "  {}",
                    crate::core::store::base_image::format_base_info(base, &base_layers)
                );
            }
        }
    }

    // E13: Automatic layer splitting by file type.
    // Config files go to a separate layer for better cache reuse.
    let all_paths: Vec<String> = res.path.iter().cloned().collect();
    let (config_paths, app_paths) = split_paths_by_type(&all_paths);

    if !config_paths.is_empty() && !app_paths.is_empty() {
        // Two layers: app binaries first (changes less), config on top (changes more)
        layers.push(LayerStrategy::Files { paths: app_paths });
        layers.push(LayerStrategy::Files {
            paths: config_paths,
        });
    } else {
        layers.push(LayerStrategy::Files { paths: all_paths });
    }

    Ok(ImageBuildPlan {
        tag: format!("{image_name}:{tag}"),
        base_image: res.image.clone(),
        layers,
        labels: vec![],
        entrypoint: res.command.clone().map(|e| vec![e]),
    })
}

/// Collect LayerEntry sets for each layer in the plan.
fn collect_layer_entries(
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
fn split_paths_by_type(paths: &[String]) -> (Vec<String>, Vec<String>) {
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

/// FJ-2403/E16: Compute a BLAKE3 hash of all layer input content.
fn compute_layer_input_hash(layer_entries: &[Vec<LayerEntry>]) -> String {
    let mut hasher = blake3::Hasher::new();
    for entries in layer_entries {
        for entry in entries {
            hasher.update(entry.path.as_bytes());
            hasher.update(&entry.content);
            hasher.update(&entry.mode.to_le_bytes());
        }
    }
    hasher.finalize().to_hex().to_string()
}

/// FJ-2403/E16: Check if a cached build with the same input hash exists.
/// Returns a cache-hit message if found, None otherwise.
fn check_build_cache(output_dir: &std::path::Path, input_hash: &str) -> Option<String> {
    let cache_path = output_dir.join("build-cache.hash");
    let cached_hash = std::fs::read_to_string(&cache_path).ok()?;
    if cached_hash.trim() == input_hash {
        let metrics_path = output_dir.join("build-metrics.json");
        if metrics_path.exists() {
            return Some(format!(
                "Layer inputs unchanged (hash: {:.16}…), skipping rebuild",
                input_hash
            ));
        }
    }
    None
}

/// FJ-2403/E16: Write the input hash for cache checking on next build.
fn write_build_cache(output_dir: &std::path::Path, input_hash: &str) {
    let cache_path = output_dir.join("build-cache.hash");
    let _ = std::fs::write(cache_path, input_hash);
}

/// Exposed for testing.
#[cfg(test)]
pub(crate) fn test_build_plan_from_resource(
    name: &str,
    res: &Resource,
    config: &ForjarConfig,
) -> Result<ImageBuildPlan, String> {
    build_plan_from_resource(name, res, config)
}

/// Exposed for testing.
#[cfg(test)]
pub(crate) fn test_collect_layer_entries(
    plan: &ImageBuildPlan,
    config: &ForjarConfig,
) -> Result<Vec<Vec<LayerEntry>>, String> {
    collect_layer_entries(plan, config)
}

/// Exposed for testing.
#[cfg(test)]
pub(crate) fn test_split_paths_by_type(paths: &[String]) -> (Vec<String>, Vec<String>) {
    split_paths_by_type(paths)
}

// Distribution functions (load/push/far) extracted to build_distribution.rs.
use super::build_distribution::{cmd_build_far, cmd_build_load, cmd_build_push};
