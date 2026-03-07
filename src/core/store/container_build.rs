//! FJ-2103: Container-based OCI image building.
//!
//! Uses Docker/Podman to build OCI images from apply scripts:
//! 1. Start ephemeral container from base image
//! 2. Execute apply scripts inside container
//! 3. Extract filesystem changes via `docker diff` + `docker cp`
//! 4. Feed through overlay_export → image_assembler pipeline
//!
//! This eliminates the pepita dependency for OCI builds — any system
//! with Docker or Podman can build deterministic container images.

use super::convergence_container::detect_container_runtime;
use super::image_assembler::{assemble_image, AssembledImage};
use super::overlay_export;
use crate::core::types::{ImageBuildPlan, OciLayerConfig};
use std::path::Path;
use std::process::Command;

/// Result of a container-based build.
#[derive(Debug)]
pub struct ContainerBuildResult {
    /// Assembled OCI image.
    pub image: AssembledImage,
    /// Container runtime used (docker/podman).
    pub runtime: String,
    /// Number of changed files extracted.
    pub changed_files: usize,
    /// Build duration in milliseconds.
    pub duration_ms: u64,
}

/// Build an OCI image by running apply scripts inside a container.
///
/// Algorithm:
/// 1. Detect container runtime (docker/podman)
/// 2. Start ephemeral container from base image
/// 3. Execute each apply script inside the container
/// 4. Extract changed files to a temp directory
/// 5. Scan extracted files through overlay_export
/// 6. Assemble OCI image via image_assembler
/// 7. Clean up container
pub fn build_image_in_container(
    plan: &ImageBuildPlan,
    apply_scripts: &[String],
    output_dir: &Path,
) -> Result<ContainerBuildResult, String> {
    let start = std::time::Instant::now();

    let runtime = detect_container_runtime()
        .ok_or_else(|| "no container runtime (docker/podman) available".to_string())?;

    let base_image = plan.base_image.as_deref().unwrap_or("debian:bookworm-slim");
    let container_name = format!("forjar-build-{}", plan.tag.replace([':', '/'], "-"));

    // Step 1: Start container from base image
    start_container(&runtime, &container_name, base_image)?;

    // Step 2: Execute apply scripts
    let exec_result = execute_scripts(&runtime, &container_name, apply_scripts);
    if let Err(e) = exec_result {
        cleanup_container(&runtime, &container_name);
        return Err(format!("apply script failed: {e}"));
    }

    // Step 3: Extract changed files (unique dir per invocation)
    let unique_id = format!(
        "{:x}{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        std::process::id(),
    );
    let extract_dir = std::env::temp_dir().join(format!("forjar-build-{unique_id}"));
    std::fs::create_dir_all(&extract_dir).map_err(|e| format!("create extract dir: {e}"))?;
    let changed = extract_changes(&runtime, &container_name, &extract_dir);
    cleanup_container(&runtime, &container_name);
    let changed_files = changed?;

    // Step 4: Scan extracted files through overlay_export
    let scan = overlay_export::scan_overlay_upper(&extract_dir, &extract_dir)
        .map_err(|e| format!("overlay scan: {e}"))?;
    // Clean up extract dir
    let _ = std::fs::remove_dir_all(&extract_dir);
    let entries = overlay_export::merge_overlay_entries(&scan);

    // Step 5: Assemble OCI image
    let layer_entries = vec![entries];
    let mut build_plan = plan.clone();
    if build_plan.layers.is_empty() {
        build_plan
            .layers
            .push(crate::core::types::LayerStrategy::Files {
                paths: vec!["(container diff)".into()],
            });
    }
    // Ensure plan layers match entry count
    while build_plan.layers.len() < layer_entries.len() {
        build_plan
            .layers
            .push(crate::core::types::LayerStrategy::Files {
                paths: vec!["(container diff)".into()],
            });
    }
    while build_plan.layers.len() > layer_entries.len() {
        build_plan.layers.pop();
    }

    let image = assemble_image(
        &build_plan,
        &layer_entries,
        output_dir,
        &OciLayerConfig::default(),
    )?;

    Ok(ContainerBuildResult {
        image,
        runtime,
        changed_files,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

/// Start an ephemeral container from a base image.
fn start_container(runtime: &str, container_name: &str, base_image: &str) -> Result<(), String> {
    let output = Command::new(runtime)
        .args([
            "run",
            "-d",
            "--rm",
            "--name",
            container_name,
            base_image,
            "sleep",
            "300",
        ])
        .output()
        .map_err(|e| format!("container start: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("container start failed: {}", stderr.trim()));
    }
    Ok(())
}

/// Execute apply scripts inside a running container.
fn execute_scripts(runtime: &str, container_name: &str, scripts: &[String]) -> Result<(), String> {
    use std::io::Write;
    use std::process::Stdio;

    for (i, script) in scripts.iter().enumerate() {
        if script.is_empty() {
            continue;
        }
        let mut child = Command::new(runtime)
            .args(["exec", "-i", container_name, "bash"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("exec script {i}: {e}"))?;

        if let Some(ref mut stdin) = child.stdin {
            stdin
                .write_all(script.as_bytes())
                .map_err(|e| format!("stdin write: {e}"))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| format!("wait script {i}: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "script {i} exit {}: {}",
                output.status.code().unwrap_or(-1),
                stderr.trim()
            ));
        }
    }
    Ok(())
}

/// Extract changed files from container to a local directory.
///
/// Uses `docker diff` to identify changes, then `docker cp` to extract them.
fn extract_changes(
    runtime: &str,
    container_name: &str,
    extract_dir: &Path,
) -> Result<usize, String> {
    // Get list of changed files
    let diff_output = Command::new(runtime)
        .args(["diff", container_name])
        .output()
        .map_err(|e| format!("docker diff: {e}"))?;

    if !diff_output.status.success() {
        let stderr = String::from_utf8_lossy(&diff_output.stderr);
        return Err(format!("docker diff failed: {}", stderr.trim()));
    }

    let diff_text = String::from_utf8_lossy(&diff_output.stdout);

    // docker diff format: "A /path" (added), "C /path" (changed dir), "D /path" (deleted)
    // We want A (added files) only — C entries are directory change markers
    let added_paths: Vec<&str> = diff_text
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with('A') {
                Some(trimmed[2..].trim())
            } else {
                None
            }
        })
        .collect();

    let file_count = added_paths.len();

    // Extract each added file via docker cp
    for path in &added_paths {
        let local_rel = path.strip_prefix('/').unwrap_or(path);
        let local_path = extract_dir.join(local_rel);

        if let Some(parent) = local_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let src = format!("{container_name}:{path}");
        let cp_result = Command::new(runtime)
            .args(["cp", &src, &local_path.to_string_lossy()])
            .output();

        // Best-effort: skip files that can't be copied (sockets, etc.)
        if let Ok(output) = cp_result {
            if !output.status.success() {
                continue;
            }
        }
    }

    Ok(file_count)
}

/// Clean up container (best-effort).
fn cleanup_container(runtime: &str, container_name: &str) {
    let _ = Command::new(runtime)
        .args(["rm", "-f", container_name])
        .output();
}

/// Format a container build result for CLI output.
pub fn format_container_build(result: &ContainerBuildResult) -> String {
    format!(
        "Container build ({runtime}): {files} files changed, {layers} layers, {size} bytes ({ms}ms)",
        runtime = result.runtime,
        files = result.changed_files,
        layers = result.image.layers.len(),
        size = result.image.total_size,
        ms = result.duration_ms,
    )
}
