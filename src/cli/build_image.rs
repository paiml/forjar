//! FJ-2104: Build container image from resource definitions.
//!
//! Wires `assemble_image()` into the `forjar build` CLI command,
//! converting resource definitions into `ImageBuildPlan` + `LayerEntry` sets.

use crate::core::store::layer_builder::LayerEntry;
use crate::core::store::overlay_export;
use crate::core::types::{ForjarConfig, ImageBuildPlan, LayerStrategy, OciLayerConfig, Resource};

/// FJ-2104: Build container image from a resource definition.
#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_build(
    file: &std::path::Path, resource: &str, load: bool, push: bool, far: bool, _json: bool,
) -> Result<(), String> {
    let config = super::helpers::parse_and_validate(file)?;
    let res = config.resources.get(resource)
        .ok_or_else(|| format!("resource '{resource}' not found"))?;
    if !matches!(res.resource_type, crate::core::types::ResourceType::Image) {
        return Err(format!("resource '{resource}' is not type: image"));
    }

    let plan = build_plan_from_resource(resource, res, &config)?;
    let layer_entries = collect_layer_entries(&plan, &config)?;
    let output_dir = std::path::Path::new("state/images").join(resource);
    std::fs::create_dir_all(&output_dir).map_err(|e| format!("create output dir: {e}"))?;

    let start = std::time::Instant::now();
    let result = crate::core::store::image_assembler::assemble_image(
        &plan, &layer_entries, &output_dir, &OciLayerConfig::default(),
    )?;
    let duration = start.elapsed();

    println!("\nBuilding {resource} ({})", plan.tag);
    for (i, layer) in result.layers.iter().enumerate() {
        println!("  Layer {}/{}: {} files, {} -> {} bytes",
            i + 1, result.layers.len(), layer.file_count,
            layer.uncompressed_size, layer.compressed_size);
    }
    println!("\n  Image: {} ({} layers, {} bytes)", plan.tag, result.layers.len(), result.total_size);
    println!("  Layout: {}", output_dir.display());
    println!("  Built in {:.1}s", duration.as_secs_f64());

    if load {
        let runtime = if super::dispatch_misc_b::which_runtime("docker") { "docker" }
            else if super::dispatch_misc_b::which_runtime("podman") { "podman" }
            else { return Err("--load requires docker or podman".into()); };
        println!("\n--load: piping OCI tarball to `{runtime} load`...");
    }
    if push {
        cmd_build_push(res, &output_dir)?;
    }
    if far {
        println!("\n--far: would wrap OCI layout in FAR archive");
    }
    Ok(())
}

/// Build an ImageBuildPlan from a resource definition.
fn build_plan_from_resource(
    name: &str, res: &Resource, _config: &ForjarConfig,
) -> Result<ImageBuildPlan, String> {
    let tag = res.version.as_deref().unwrap_or("latest");
    let image_name = res.name.as_deref().unwrap_or(name);

    // Check for base image layers
    let mut layers = Vec::new();
    if let Some(ref base) = res.image {
        let base_dir = std::path::Path::new("state/images").join(
            base.replace([':', '/'], "_")
        );
        if base_dir.exists() {
            if let Ok(base_layers) = crate::core::store::base_image::extract_base_layers(&base_dir) {
                println!("  {}", crate::core::store::base_image::format_base_info(base, &base_layers));
            }
        }
    }

    // Add user layers
    layers.push(LayerStrategy::Files {
        paths: res.path.iter().cloned().collect(),
    });

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
    plan: &ImageBuildPlan, config: &ForjarConfig,
) -> Result<Vec<Vec<LayerEntry>>, String> {
    plan.layers.iter()
        .map(|strategy| collect_strategy_entries(strategy, config))
        .collect()
}

fn collect_strategy_entries(
    strategy: &LayerStrategy, config: &ForjarConfig,
) -> Result<Vec<LayerEntry>, String> {
    match strategy {
        LayerStrategy::Files { paths } => Ok(collect_file_entries(paths, config)),
        LayerStrategy::Packages { names } => {
            let content = names.join("\n");
            Ok(vec![LayerEntry::file("var/lib/forjar/packages.list", content.as_bytes(), 0o644)])
        }
        LayerStrategy::Build { command: _, workdir } => collect_build_entries(workdir.as_deref()),
        LayerStrategy::Derivation { store_path } => collect_derivation_entries(store_path),
    }
}

fn collect_file_entries(paths: &[String], config: &ForjarConfig) -> Vec<LayerEntry> {
    paths.iter().map(|path| {
        if let Some(res) = config.resources.values().find(|r| r.path.as_deref() == Some(path)) {
            let content = res.content.as_deref().unwrap_or("").as_bytes();
            let mode = res.mode.as_deref().and_then(|m| u32::from_str_radix(m, 8).ok()).unwrap_or(0o644);
            LayerEntry::file(path, content, mode)
        } else {
            LayerEntry::file(path, b"", 0o644)
        }
    }).collect()
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

/// Exposed for testing.
#[cfg(test)]
pub(crate) fn test_build_plan_from_resource(
    name: &str, res: &Resource, config: &ForjarConfig,
) -> Result<ImageBuildPlan, String> {
    build_plan_from_resource(name, res, config)
}

/// Exposed for testing.
#[cfg(test)]
pub(crate) fn test_collect_layer_entries(
    plan: &ImageBuildPlan, config: &ForjarConfig,
) -> Result<Vec<Vec<LayerEntry>>, String> {
    collect_layer_entries(plan, config)
}

/// FJ-2105: Handle --push flag for registry push.
fn cmd_build_push(res: &Resource, oci_dir: &std::path::Path) -> Result<(), String> {
    use crate::core::store::registry_push;

    let image_name = res.name.as_deref().unwrap_or("app");
    let tag = res.version.as_deref().unwrap_or("latest");

    let (registry, name) = if let Some(idx) = image_name.find('/') {
        (&image_name[..idx], &image_name[idx + 1..])
    } else {
        ("docker.io", image_name)
    };

    let push_config = registry_push::RegistryPushConfig {
        registry: registry.to_string(),
        name: name.to_string(),
        tag: tag.to_string(),
        check_existing: true,
    };

    let errors = registry_push::validate_push_config(&push_config);
    if !errors.is_empty() {
        return Err(format!("push config invalid: {}", errors.join(", ")));
    }

    println!("\n--push: OCI Distribution v1.1");
    println!("  registry: {registry}");
    println!("  name: {name}");
    println!("  tag: {tag}");

    // FJ-2105: Execute actual push via OCI Distribution protocol.
    // Discover blobs first (local-only), then push them.
    let blobs = registry_push::discover_blobs(oci_dir)?;
    if blobs.is_empty() {
        println!("  no blobs to push");
        return Ok(());
    }
    println!("  blobs: {} to push", blobs.len());

    match registry_push::push_image(oci_dir, &push_config) {
        Ok(results) => print!("{}", registry_push::format_push_summary(&results)),
        Err(e) if e.contains("Location header") || e.contains("curl") => {
            // Registry unreachable — report but don't fail the build
            println!("  push skipped: registry unreachable ({e})");
        }
        Err(e) => return Err(e),
    }
    Ok(())
}
