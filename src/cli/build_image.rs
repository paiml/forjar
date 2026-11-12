//! FJ-2104: Build container image from resource definitions.
//!
//! Wires `assemble_image()` into the `forjar build` CLI command,
//! converting resource definitions into `ImageBuildPlan` + `LayerEntry` sets.

use super::build_image_base::{load_declared_base, print_base_line};
use super::build_image_cache::{check_build_cache, compute_layer_input_hash, write_build_cache};
use super::build_image_entries::collect_layer_entries;
use super::build_image_entries::split_paths_by_type;
use super::build_image_exec::{exec_decl, exec_summary, run_exec, ExecDecl, ExecReport};
use super::build_image_report::{print_build_json, print_build_report, record_build_metrics};
use crate::core::store::build_runtime::{BuildRuntime, ContainerCli};
use crate::core::store::image_assembler::BaseLayout;
use crate::core::store::layer_builder::LayerEntry;
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
    cmd_build_in(
        std::path::Path::new(STATE_IMAGES_DIR),
        file,
        resource,
        load,
        push,
        far,
        sandbox,
        json,
    )
}

/// Where layouts live, relative to the working directory.
const STATE_IMAGES_DIR: &str = "state/images";

/// `cmd_build` with the image state directory passed in, so tests can use a
/// private directory instead of the shared, cwd-relative `state/images`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_build_in(
    state_dir: &std::path::Path,
    file: &std::path::Path,
    resource: &str,
    load: bool,
    push: bool,
    far: bool,
    sandbox: bool,
    json: bool,
) -> Result<(), String> {
    let real = || ContainerCli::detect().map(|c| Box::new(c) as Box<dyn BuildRuntime>);
    cmd_build_rt(
        &real, state_dir, file, resource, load, push, far, sandbox, json,
    )
}

/// A source of the container runtime, called only when steps must run.
pub(crate) type RuntimeProvider<'a> = &'a dyn Fn() -> Result<Box<dyn BuildRuntime>, String>;

/// `cmd_build_in` with the container runtime injected, so tests can drive the
/// whole command with a fake runtime and no docker.
#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_build_rt(
    runtime: RuntimeProvider,
    state_dir: &std::path::Path,
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
    let output_dir = state_dir.join(resource);

    if sandbox {
        // PMAT-989 phase 1 leaves the sandbox path alone (it builds in a container).
        std::fs::create_dir_all(&output_dir).map_err(|e| format!("create output dir: {e}"))?;
        return cmd_build_sandbox(resource, &plan, &config, &output_dir, load, push, far);
    }

    let job = Job {
        resource,
        res,
        config: &config,
        state_dir,
        output_dir,
        dist: Dist { load, push, far },
        json,
    };
    build_layered(runtime, plan, &job)
}

/// What one `forjar build` invocation is building, and what to do with it after.
struct Job<'a> {
    resource: &'a str,
    res: &'a Resource,
    config: &'a ForjarConfig,
    state_dir: &'a std::path::Path,
    output_dir: std::path::PathBuf,
    dist: Dist,
    json: bool,
}

/// The distribution flags (`--load`, `--push`, `--far`).
struct Dist {
    load: bool,
    push: bool,
    far: bool,
}

impl Job<'_> {
    fn distribute(&self, tag: &str) -> Result<(), String> {
        let d = &self.dist;
        run_distribution(self.resource, tag, &self.output_dir, d.load, d.push, d.far)
    }
}

/// PMAT-989: steps run IN the declared base; with no base there is nothing to run in.
fn require_base_for_steps(job: &Job, decl: Option<&ExecDecl>) -> Result<(), String> {
    if decl.is_some() && job.res.image.is_none() {
        return Err(format!(
            "image '{}' declares `packages:`/`command:` but no `image:` base: the steps \
             run in a container started from the base, so declare one (e.g. image: \
             debian:bookworm-slim). `command:` is a build step now, not the entrypoint; use \
             `entrypoint: [..]` for the process the image starts.",
            job.resource
        ));
    }
    Ok(())
}

/// Layer the declared base (and, when steps are declared, their executed
/// layer) under the Files layers, or serve the build cache.
fn build_layered(runtime: RuntimeProvider, plan: ImageBuildPlan, job: &Job) -> Result<(), String> {
    let decl = exec_decl(job.res);
    require_base_for_steps(job, decl.as_ref())?;

    // PMAT-989: a declared base is layered or the build fails — before anything is written.
    let base = job
        .res
        .image
        .as_deref()
        .map(|r| load_declared_base(job.resource, job.state_dir, r))
        .transpose()?;
    std::fs::create_dir_all(&job.output_dir).map_err(|e| format!("create output dir: {e}"))?;

    let layer_entries = collect_layer_entries(&plan, job.config)?;
    let report = decl.as_ref().map(ExecReport::new);

    // FJ-2403/E16: Check build cache — skip rebuild if inputs unchanged.
    let input_hash = compute_layer_input_hash(
        &layer_entries,
        base.as_ref(),
        decl.as_ref(),
        plan.entrypoint.as_deref().unwrap_or(&[]),
    );
    if let Some(cached) = check_build_cache(&job.output_dir, &input_hash) {
        return finish_cached(
            job,
            &plan,
            base.as_ref(),
            report.as_ref(),
            &cached,
            &input_hash,
        );
    }

    let mut report = report;
    let start = std::time::Instant::now();
    let (plan, layer_entries) = with_executed_layer(
        runtime,
        plan,
        layer_entries,
        decl.as_ref(),
        job.res.image.as_deref().zip(base.as_ref()),
        &mut report,
    )?;
    let result = crate::core::store::image_assembler::assemble_image_with_base(
        &plan,
        &layer_entries,
        &job.output_dir,
        &OciLayerConfig::default(),
        None, // E12: default to host architecture
        base.as_ref(),
    )?;
    let duration = start.elapsed();

    // FJ-2403/E17: Collect and persist image build metrics.
    record_build_metrics(&plan.tag, &result, duration, &job.output_dir);
    write_build_cache(&job.output_dir, &input_hash);

    let base_info = base_summary(job.res, base.as_ref());
    if job.json {
        print_build_json(
            &plan.tag,
            &job.output_dir,
            Some(&result),
            false,
            base_info,
            report.as_ref(),
        );
        return Ok(());
    }
    print_build_report(job.resource, &plan.tag, &result, &job.output_dir, duration);
    print_base_line(job.res.image.as_deref(), base.as_ref());
    if let Some(r) = &report {
        println!("  {}", r.line());
    }
    job.distribute(&plan.tag)
}

/// The declared base reference and its layer count, when one is layered.
fn base_summary<'a>(res: &'a Resource, base: Option<&BaseLayout>) -> Option<(&'a str, usize)> {
    res.image
        .as_deref()
        .zip(base)
        .map(|(r, b)| (r, b.layers.layers.len()))
}

/// The build-cache hit path: report it, run the distribution flags, build nothing.
fn finish_cached(
    job: &Job,
    plan: &ImageBuildPlan,
    base: Option<&BaseLayout>,
    report: Option<&ExecReport>,
    cached: &str,
    input_hash: &str,
) -> Result<(), String> {
    if job.json {
        let info = base_summary(job.res, base);
        print_build_json(&plan.tag, &job.output_dir, None, true, info, report);
        return Ok(());
    }
    println!("\nBuilding {} ({}) — CACHED", job.resource, plan.tag);
    print_base_line(job.res.image.as_deref(), base);
    if let Some(r) = report {
        println!("  {}", r.line());
    }
    println!("  {cached}");
    println!("  Input hash: {input_hash}");
    job.distribute(&plan.tag)
}

/// PMAT-989: run the declared steps in the base and put the captured layer
/// directly above it, below the Files layers. No steps: the inputs come back
/// untouched and no runtime is asked for.
fn with_executed_layer(
    runtime: RuntimeProvider,
    mut plan: ImageBuildPlan,
    mut entries: Vec<Vec<LayerEntry>>,
    decl: Option<&ExecDecl>,
    base: Option<(&str, &BaseLayout)>,
    report: &mut Option<ExecReport>,
) -> Result<(ImageBuildPlan, Vec<Vec<LayerEntry>>), String> {
    let (Some(decl), Some((base_ref, base))) = (decl, base) else {
        return Ok((plan, entries));
    };
    let rt = runtime()?;
    let layer = run_exec(rt.as_ref(), decl, base_ref, base)?;
    plan.layers.insert(
        0,
        LayerStrategy::Build {
            command: exec_summary(decl),
            workdir: None,
        },
    );
    *report = report.take().map(|r| r.captured(&layer));
    entries.insert(0, layer.entries);
    Ok((plan, entries))
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

    run_distribution(resource, &plan.tag, output_dir, load, push, far)
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

    // The declared base is resolved and layered by `cmd_build_in` (PMAT-989).
    let mut layers = Vec::new();

    // E13: Automatic layer splitting by file type.
    // Config files go to a separate layer for better cache reuse.
    let all_paths: Vec<String> = res.path.iter().cloned().collect();
    let (config_paths, app_paths) = split_paths_by_type(&all_paths);

    if all_paths.is_empty() && exec_decl(res).is_some() {
        // PMAT-989: the executed layer is the content; no empty Files layer beside it.
    } else if !config_paths.is_empty() && !app_paths.is_empty() {
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
        // PMAT-989: `command:` is a build step now; the entrypoint is its own field.
        entrypoint: (!res.entrypoint.is_empty()).then(|| res.entrypoint.clone()),
    })
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
