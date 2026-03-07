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
    file: &std::path::Path,
    resource: &str,
    load: bool,
    push: bool,
    far: bool,
    sandbox: bool,
    _json: bool,
) -> Result<(), String> {
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
    let start = std::time::Instant::now();
    let result = crate::core::store::image_assembler::assemble_image(
        &plan,
        &layer_entries,
        &output_dir,
        &OciLayerConfig::default(),
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
    _config: &ForjarConfig,
) -> Result<ImageBuildPlan, String> {
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

/// FJ-2106: Handle --load flag — tar OCI layout and pipe to docker/podman load.
fn cmd_build_load(oci_dir: &std::path::Path) -> Result<(), String> {
    let runtime = if super::dispatch_misc_b::which_runtime("docker") {
        "docker"
    } else if super::dispatch_misc_b::which_runtime("podman") {
        "podman"
    } else {
        return Err("--load requires docker or podman on PATH".into());
    };

    println!("\n--load: piping OCI tarball to `{runtime} load`...");
    let tar_output = std::process::Command::new("tar")
        .arg("-cf")
        .arg("-")
        .arg("-C")
        .arg(oci_dir)
        .arg(".")
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn tar: {e}"))?;

    let status = std::process::Command::new(runtime)
        .arg("load")
        .stdin(tar_output.stdout.ok_or("tar stdout unavailable")?)
        .status()
        .map_err(|e| format!("{runtime} load: {e}"))?;

    if status.success() {
        println!("  loaded into {runtime}");
        Ok(())
    } else {
        Err(format!("{runtime} load exited with {status}"))
    }
}

/// FJ-2107: Handle --far flag — wrap OCI layout in a FAR archive.
fn cmd_build_far(resource: &str, oci_dir: &std::path::Path) -> Result<(), String> {
    use crate::core::store::far::{encode_far, FarManifest, FarProvenance};

    let mut files = Vec::new();
    let mut chunks = Vec::new();
    let mut total_size: u64 = 0;

    collect_far_files(oci_dir, oci_dir, &mut files, &mut chunks, &mut total_size)?;

    let tree_hash = if chunks.is_empty() {
        blake3::hash(b"empty").to_hex().to_string()
    } else {
        let mut hasher = blake3::Hasher::new();
        for (h, _) in &chunks {
            hasher.update(h);
        }
        hasher.finalize().to_hex().to_string()
    };

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let manifest = FarManifest {
        name: resource.to_string(),
        version: "1.0.0".to_string(),
        arch: std::env::consts::ARCH.to_string(),
        store_hash: tree_hash.clone(),
        tree_hash,
        file_count: files.len() as u64,
        total_size,
        files,
        provenance: FarProvenance {
            origin_provider: "forjar-build".to_string(),
            origin_ref: None,
            origin_hash: None,
            created_at: format!("{ts}"),
            generator: format!("forjar {}", env!("CARGO_PKG_VERSION")),
        },
        kernel_contracts: None,
    };

    let far_path = oci_dir.with_extension("far");
    let file = std::fs::File::create(&far_path).map_err(|e| format!("create FAR: {e}"))?;
    let writer = std::io::BufWriter::new(file);
    encode_far(&manifest, &chunks, writer)?;

    let far_size = std::fs::metadata(&far_path).map(|m| m.len()).unwrap_or(0);
    println!("\n--far: {}", far_path.display());
    println!(
        "  {} files, {} bytes -> {} bytes FAR",
        manifest.file_count, total_size, far_size
    );
    Ok(())
}

/// Recursively collect files from OCI dir into FAR entries and chunks.
fn collect_far_files(
    base: &std::path::Path,
    dir: &std::path::Path,
    files: &mut Vec<crate::core::store::far::FarFileEntry>,
    chunks: &mut Vec<([u8; 32], Vec<u8>)>,
    total_size: &mut u64,
) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("read dir: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("dir entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_far_files(base, &path, files, chunks, total_size)?;
        } else {
            let data = std::fs::read(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
            let hash = blake3::hash(&data);
            let rel = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            *total_size += data.len() as u64;
            files.push(crate::core::store::far::FarFileEntry {
                path: rel,
                size: data.len() as u64,
                blake3: hash.to_hex().to_string(),
            });
            chunks.push((*hash.as_bytes(), data));
        }
    }
    Ok(())
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
