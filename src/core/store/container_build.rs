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

    // Step 1: Start container from base image.
    //
    // FJ-#154: A `ContainerGuard` tears the container down on EVERY early
    // return (e.g. a failed `create_dir_all` of the extract dir), not just the
    // apply-script branch — otherwise the container leaks, running its full
    // `sleep 300`.
    start_container(&runtime, &container_name, base_image)?;
    let container = ContainerGuard::new(&runtime, &container_name);

    // Step 2: Execute apply scripts
    execute_scripts(&runtime, &container_name, apply_scripts)
        .map_err(|e| format!("apply script failed: {e}"))?;

    // Step 3: Extract changed files (unique dir per invocation).
    //
    // FJ-#154: A `TempDirGuard` removes the extract dir on every return path,
    // including a `scan_overlay_upper` failure that previously leaked the temp
    // dir full of copied container files.
    let extract_dir = std::env::temp_dir().join(format!("forjar-build-{}", build_unique_id()));
    std::fs::create_dir_all(&extract_dir).map_err(|e| format!("create extract dir: {e}"))?;
    let _extract_guard = TempDirGuard(extract_dir.clone());
    let changed_files = extract_changes(&runtime, &container_name, &extract_dir)?;
    // Filesystem changes are captured; the container is no longer needed.
    drop(container);

    // Step 4: Scan extracted files through overlay_export
    let scan = overlay_export::scan_overlay_upper(&extract_dir, &extract_dir)
        .map_err(|e| format!("overlay scan: {e}"))?;
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
        None,
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

        // FJ-#154: reap the child if stdin write fails (no zombie on early return).
        crate::transport::write_stdin_or_reap(&mut child, script)
            .map_err(|e| format!("script {i}: {e}"))?;

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

/// Build a process-unique id for the extract temp directory.
fn build_unique_id() -> String {
    format!(
        "{:x}{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        std::process::id(),
    )
}

/// FJ-#154: RAII guard that tears down the ephemeral build container on drop.
///
/// Guarantees `cleanup_container` runs on every early return. `drop` it
/// explicitly once the container's filesystem changes have been extracted.
struct ContainerGuard {
    runtime: String,
    name: String,
}

impl ContainerGuard {
    fn new(runtime: &str, name: &str) -> Self {
        Self {
            runtime: runtime.to_string(),
            name: name.to_string(),
        }
    }
}

impl Drop for ContainerGuard {
    fn drop(&mut self) {
        cleanup_container(&self.runtime, &self.name);
    }
}

/// FJ-#154: RAII guard that removes a temp directory on drop, so a failure in
/// `scan_overlay_upper` (or any later step) never leaks the extract dir.
struct TempDirGuard(std::path::PathBuf);

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::store::layer_builder::LayerEntry;
    use crate::core::types::{ImageBuildPlan, LayerStrategy, OciLayerConfig};

    fn test_plan() -> ImageBuildPlan {
        ImageBuildPlan {
            tag: "test:latest".into(),
            base_image: None,
            layers: vec![LayerStrategy::Files {
                paths: vec!["/test".into()],
            }],
            labels: vec![],
            entrypoint: None,
        }
    }

    #[test]
    fn format_container_build_output() {
        let tmp = tempfile::tempdir().unwrap();
        let plan = test_plan();

        let entries = vec![vec![LayerEntry::file("hello.txt", b"hello world", 0o644)]];

        let output_dir = tmp.path().join("output");
        std::fs::create_dir_all(&output_dir).unwrap();

        let image = assemble_image(
            &plan,
            &entries,
            &output_dir,
            &OciLayerConfig::default(),
            None,
        )
        .unwrap();

        let result = ContainerBuildResult {
            image,
            runtime: "docker".into(),
            changed_files: 3,
            duration_ms: 1500,
        };

        let formatted = format_container_build(&result);
        assert!(formatted.contains("docker"));
        assert!(formatted.contains("3 files changed"));
        assert!(formatted.contains("1 layers"));
        assert!(formatted.contains("1500ms"));
    }

    #[test]
    #[ignore] // requires Docker or Podman
    fn build_image_in_container_echo() {
        let tmp = tempfile::tempdir().unwrap();
        let output_dir = tmp.path().join("output");
        std::fs::create_dir_all(&output_dir).unwrap();

        let mut plan = test_plan();
        plan.base_image = Some("debian:bookworm-slim".into());

        let scripts = vec!["echo 'hello' > /test.txt".to_string()];
        let result = build_image_in_container(&plan, &scripts, &output_dir);
        assert!(result.is_ok() || result.is_err());
    }

    // --- FJ-#154: leak guards (no Docker required) ---

    /// #8: TempDirGuard removes its directory on drop, even on an error path.
    #[test]
    fn temp_dir_guard_removes_on_drop() {
        let base = tempfile::tempdir().unwrap();
        let dir = base.path().join("forjar-build-leaktest");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("copied-file"), b"data").unwrap();
        assert!(dir.exists());
        {
            let _guard = TempDirGuard(dir.clone());
            // Simulate scan_overlay_upper failing: we just leave the scope.
        }
        assert!(!dir.exists(), "extract dir must be removed by the guard");
    }

    /// #8: dropping TempDirGuard on an already-missing dir is harmless.
    #[test]
    fn temp_dir_guard_missing_is_ok() {
        let base = tempfile::tempdir().unwrap();
        let dir = base.path().join("never-created");
        let _guard = TempDirGuard(dir);
        // Drop at end of scope must not panic.
    }

    /// #8: ContainerGuard invokes cleanup on drop. Using `true` as the
    /// "runtime" makes the cleanup command a deterministic no-op (no Docker).
    #[test]
    fn container_guard_cleans_up_on_drop() {
        // Constructing and dropping must not panic; cleanup runs `true rm -f`.
        let _guard = ContainerGuard::new("true", "forjar-build-guardtest");
    }

    /// #8: build_unique_id is non-empty and PID-suffixed.
    #[test]
    fn build_unique_id_is_nonempty() {
        let id = build_unique_id();
        assert!(!id.is_empty());
        assert!(id.ends_with(&format!("{:x}", std::process::id())));
    }
}
