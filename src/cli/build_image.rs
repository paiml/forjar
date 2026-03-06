//! FJ-2104: Build container image from resource definitions.
//!
//! Wires `assemble_image()` into the `forjar build` CLI command,
//! converting resource definitions into `ImageBuildPlan` + `LayerEntry` sets.

use crate::core::store::layer_builder::LayerEntry;
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
        cmd_build_push(res)?;
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
    Ok(ImageBuildPlan {
        tag: format!("{image_name}:{tag}"),
        base_image: res.image.clone(),
        layers: vec![LayerStrategy::Files {
            paths: res.path.iter().cloned().collect(),
        }],
        labels: vec![],
        entrypoint: res.command.clone().map(|e| vec![e]),
    })
}

/// Collect LayerEntry sets for each layer in the plan.
fn collect_layer_entries(
    plan: &ImageBuildPlan, config: &ForjarConfig,
) -> Result<Vec<Vec<LayerEntry>>, String> {
    let mut all_entries = Vec::new();
    for strategy in &plan.layers {
        let entries = match strategy {
            LayerStrategy::Files { paths } => {
                let mut e = Vec::new();
                for path in paths {
                    if let Some(res) = config.resources.values().find(|r| r.path.as_deref() == Some(path)) {
                        let content = res.content.as_deref().unwrap_or("").as_bytes();
                        let mode = res.mode.as_deref().and_then(|m| u32::from_str_radix(m, 8).ok()).unwrap_or(0o644);
                        e.push(LayerEntry::file(path, content, mode));
                    } else {
                        e.push(LayerEntry::file(path, b"", 0o644));
                    }
                }
                e
            }
            LayerStrategy::Packages { names } => {
                let content = names.join("\n");
                vec![LayerEntry::file("var/lib/forjar/packages.list", content.as_bytes(), 0o644)]
            }
            _ => vec![],
        };
        all_entries.push(entries);
    }
    Ok(all_entries)
}

/// FJ-2105: Handle --push flag for registry push.
fn cmd_build_push(res: &Resource) -> Result<(), String> {
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
    println!("  check-existing: HEAD /v2/{name}/blobs/{{digest}}");
    println!("  protocol: POST uploads/ → PUT ?digest= → PUT manifests/{tag}");
    Ok(())
}
