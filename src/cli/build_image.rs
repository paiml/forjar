//! FJ-2104: Build container image from resource definitions.
//!
//! Wires `assemble_image()` into the `forjar build` CLI command,
//! converting resource definitions into `ImageBuildPlan` + `LayerEntry` sets.

use super::build_image_base;
use super::build_image_report::{print_build_json, print_build_report, record_build_metrics};
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
    json: bool,
) -> Result<(), String> {
    // Refs #212: `--json` used to print "not yet implemented, flag ignored"
    // and then the human build log, so `forjar build --json | jq` failed while
    // the command exited 0. It now emits a real manifest — and refuses the
    // combinations whose stdout it cannot own, rather than emitting JSON with
    // `docker load` progress interleaved into it.
    if json {
        reject_json_stdout_conflicts(load, push, far, sandbox)?;
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

    // IMG-001: packages and a build command can only be honoured by running them.
    if sandbox || super::build_image_exec::plan_needs_execution(&plan) {
        return cmd_build_sandbox(
            resource,
            res,
            &plan,
            &config,
            &output_dir,
            load,
            push,
            far,
            json,
        );
    }

    let layer_entries = collect_layer_entries(&plan, &config)?;

    // IMG-001: the declared base sits under the built layers. It is resolved
    // BEFORE the cache is consulted and keyed into the input hash, so a layout
    // first built as scratch is never served as the based image. A base that
    // cannot be produced (or fails its pinned digest) fails the build.
    let base_work =
        std::env::temp_dir().join(format!("forjar-base-{}-{}", std::process::id(), resource));
    let base = build_image_base::resolve_plan_base(
        &plan,
        std::path::Path::new("state/images"),
        &base_work,
        build_image_base::detect_runtime(),
    );
    let base = match base {
        Ok(b) => b,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&base_work);
            return Err(e);
        }
    };

    // FJ-2403/E16: Check build cache — skip rebuild if inputs unchanged.
    let input_hash = build_image_base::compute_layer_input_hash(&layer_entries, base.as_ref());
    if let Some(cached) = build_image_base::check_build_cache(&output_dir, &input_hash) {
        let _ = std::fs::remove_dir_all(&base_work);
        if json {
            print_build_json(&plan.tag, &output_dir, None, true);
            return Ok(());
        }
        println!("\nBuilding {resource} ({}) — CACHED", plan.tag);
        println!("  {cached}");
        println!("  Input hash: {input_hash}");
        run_distribution(resource, &plan.tag, &output_dir, load, push, far)?;
        return Ok(());
    }

    let start = std::time::Instant::now();
    let result = crate::core::store::image_assembler::assemble_image_with_base(
        &plan,
        &layer_entries,
        &output_dir,
        &OciLayerConfig::default(),
        None, // E12: default to host architecture
        base.as_ref(),
    );
    let _ = std::fs::remove_dir_all(&base_work);
    let result = result?;
    let duration = start.elapsed();

    if !json {
        print_build_report(resource, &plan.tag, &result, &output_dir, duration);
    }

    // FJ-2403/E17: Collect and persist image build metrics.
    record_build_metrics(&plan.tag, &result, duration, &output_dir);
    build_image_base::write_build_cache(&output_dir, &input_hash);

    if json {
        print_build_json(&plan.tag, &output_dir, Some(&result), false);
        return Ok(());
    }

    run_distribution(resource, &plan.tag, &output_dir, load, push, far)
}

/// Refs #212: the flags `--json` cannot share stdout with.
///
/// Lifted out of `cmd_build` unchanged, including the order the flags are
/// tested in, which decides which one the message names.
fn reject_json_stdout_conflicts(
    load: bool,
    push: bool,
    far: bool,
    sandbox: bool,
) -> Result<(), String> {
    let blocking = [
        (load, "--load"),
        (push, "--push"),
        (far, "--far"),
        (sandbox, "--sandbox"),
    ];
    if let Some((_, flag)) = blocking.into_iter().find(|(on, _)| *on) {
        return Err(format!(
            "--json is not supported together with {flag}: {flag} streams \
             human-readable progress on stdout, which would not parse as JSON. \
             Run the build with --json first, then {flag} separately."
        ));
    }
    Ok(())
}

/// Hand a finished layout to the distribution flags, in the order a build
/// has always applied them: `--load`, then `--push`, then `--far`.
fn run_distribution(
    resource: &str,
    tag: &str,
    output_dir: &std::path::Path,
    load: bool,
    push: bool,
    far: bool,
) -> Result<(), String> {
    if load {
        cmd_build_load(output_dir)?;
    }
    if push {
        cmd_build_push(tag, output_dir)?;
    }
    if far {
        cmd_build_far(resource, output_dir)?;
    }
    Ok(())
}

/// FJ-2103: Build image inside container sandbox (Docker/Podman).
#[allow(clippy::too_many_arguments)]
fn cmd_build_sandbox(
    resource: &str,
    res: &Resource,
    plan: &ImageBuildPlan,
    config: &ForjarConfig,
    output_dir: &std::path::Path,
    load: bool,
    push: bool,
    far: bool,
    json: bool,
) -> Result<(), String> {
    use crate::core::store::container_build;

    // Generate apply scripts from non-image resources in the config
    let mut apply_scripts: Vec<String> = super::build_image_exec::image_build_scripts(plan, res)?;
    apply_scripts.extend(
        config
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
            .collect::<Vec<_>>(),
    );

    if !json {
        println!("\nBuilding {resource} ({}) via container sandbox", plan.tag);
        println!("  Scripts: {}", apply_scripts.len());
    }

    let file_layers = sandbox_file_layers(plan, config)?;
    let result = container_build::build_image_in_container_with_files(
        plan,
        &apply_scripts,
        &file_layers,
        output_dir,
    )?;

    if json {
        print_build_json(&plan.tag, output_dir, Some(&result.image), false);
        return Ok(());
    }
    println!("  {}", container_build::format_container_build(&result));
    println!("  Layout: {}", output_dir.display());

    run_distribution(resource, &plan.tag, output_dir, load, push, far)
}

/// IMG-001: the resource's `path:` Files layers, to sit on top of the container
/// diff. `image_build_scripts` covers only packages and command, and image
/// resources are filtered out of the apply scripts, so without these the file
/// the spec declares never lands in the image.
pub(crate) fn sandbox_file_layers(
    plan: &ImageBuildPlan,
    config: &ForjarConfig,
) -> Result<Vec<Vec<LayerEntry>>, String> {
    collect_layer_entries(&super::build_image_exec::files_only_plan(plan), config)
}

/// Refs #210: the one place an image resource's reference is decided.
///
/// `tag:` on an image resource is honoured as a FULL reference
/// (`registry/repo:tag`) when it names one; it used to be parsed and silently
/// dropped, so a resource declaring `ghcr.io/foo/bar:1.2.3` built — and
/// pushed — under a name nobody wrote. `--push` reuses this exact string, so
/// the pushed reference is by construction the reference that was built.
fn resource_image_reference(name: &str, res: &Resource) -> String {
    let declared = res.tag.as_deref().map(str::trim).filter(|t| !t.is_empty());
    if let Some(full) = declared.filter(|t| t.contains('/') || t.contains(':')) {
        return full.to_string();
    }
    let version = declared
        .or(res.version.as_deref())
        .unwrap_or(DEFAULT_IMAGE_TAG);
    let image_name = res.name.as_deref().unwrap_or(name);
    format!("{image_name}:{version}")
}

/// Tag assumed when an image resource declares no version.
const DEFAULT_IMAGE_TAG: &str = "latest";

/// Build an ImageBuildPlan from a resource definition.
fn build_plan_from_resource(
    name: &str,
    res: &Resource,
    config: &ForjarConfig,
) -> Result<ImageBuildPlan, String> {
    // GH-91: config not yet used for build plan customization
    let _ = config;
    let image_reference = resource_image_reference(name, res);

    // IMG-001: `packages` (tier 0) and a build `command` (tier 1) are layers of
    // their own; the command is then NOT an entrypoint. A resource without
    // `packages` keeps the legacy reading of `command` as the entrypoint.
    let mut layers = Vec::new();
    let has_packages = !res.packages.is_empty();
    if has_packages {
        layers.push(LayerStrategy::Packages {
            names: res.packages.clone(),
        });
    }
    let command = res.command.clone().filter(|c| !c.trim().is_empty());
    if let (true, Some(cmd)) = (has_packages, command.as_ref()) {
        layers.push(LayerStrategy::Build {
            command: cmd.clone(),
            workdir: None,
        });
    }
    let entrypoint = if has_packages {
        None
    } else {
        command.map(|e| vec![e])
    };

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
        tag: image_reference,
        base_image: res.image.clone(),
        layers,
        labels: vec![],
        entrypoint,
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
