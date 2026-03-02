//! FJ-1316–FJ-1319: Sandbox lifecycle executor.
//!
//! Implements the 10-step sandbox build lifecycle:
//! 1. Create namespace (PID/mount/net via pepita)
//! 2. Overlay mount (lower=inputs, upper=tmpfs)
//! 3. Bind inputs read-only
//! 4. cgroup limits (memory_mb, cpus)
//! 5. Seccomp BPF (Full level: deny connect/mount/ptrace)
//! 6. Execute bashrs-purified build script
//! 7. Extract outputs from $out
//! 8. hash_directory() → store hash
//! 9. Atomic move to store
//! 10. Destroy namespace
//!
//! All I/O operations produce plans (command lists) rather than executing
//! directly, following forjar's dry-run-first principle.

use super::sandbox::{SandboxConfig, SandboxLevel};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A single step in the sandbox lifecycle.
#[derive(Debug, Clone, PartialEq)]
pub struct SandboxStep {
    /// Human-readable description
    pub description: String,
    /// Shell command to execute (if applicable)
    pub command: Option<String>,
    /// Step number (1-based)
    pub step: u8,
}

/// The full sandbox execution plan.
#[derive(Debug, Clone, PartialEq)]
pub struct SandboxPlan {
    /// Ordered steps to execute
    pub steps: Vec<SandboxStep>,
    /// Namespace identifier (derived from store hash prefix)
    pub namespace_id: String,
    /// Overlay mount points
    pub overlay: OverlayConfig,
    /// Seccomp BPF rules (empty for non-Full levels)
    pub seccomp_rules: Vec<SeccompRule>,
    /// cgroup path
    pub cgroup_path: String,
}

/// Overlay filesystem configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayConfig {
    /// Read-only lower layers (input store paths)
    pub lower_dirs: Vec<PathBuf>,
    /// Writable upper directory (tmpfs)
    pub upper_dir: PathBuf,
    /// Work directory for overlayfs
    pub work_dir: PathBuf,
    /// Merged mount point
    pub merged_dir: PathBuf,
}

/// A seccomp BPF deny rule.
#[derive(Debug, Clone, PartialEq)]
pub struct SeccompRule {
    /// Syscall name to deny
    pub syscall: String,
    /// Action (always "deny" for sandbox)
    pub action: String,
}

/// Result of a completed sandbox build.
#[derive(Debug, Clone, PartialEq)]
pub struct SandboxResult {
    /// BLAKE3 hash of the output directory
    pub output_hash: String,
    /// Store path where the output was placed
    pub store_path: String,
    /// All lifecycle steps that were executed
    pub steps_executed: Vec<String>,
}

/// Generate the full sandbox execution plan for a build.
///
/// This produces a plan but does NOT execute it. The plan describes
/// every namespace, mount, cgroup, and seccomp operation needed.
pub fn plan_sandbox_build(
    config: &SandboxConfig,
    build_hash: &str,
    input_paths: &BTreeMap<String, PathBuf>,
    script: &str,
    store_dir: &Path,
) -> SandboxPlan {
    let hash_short = &build_hash[..16.min(build_hash.len())];
    let namespace_id = format!("forjar-build-{hash_short}");
    let build_root = PathBuf::from(format!("/tmp/forjar-sandbox/{namespace_id}"));
    let cgroup_path = super::sandbox::cgroup_path(build_hash);

    let overlay = OverlayConfig {
        lower_dirs: input_paths.values().cloned().collect(),
        upper_dir: build_root.join("upper"),
        work_dir: build_root.join("work"),
        merged_dir: build_root.join("merged"),
    };

    let seccomp_rules = seccomp_rules_for_level(config.level);

    let mut steps = Vec::new();

    // Step 1: Create namespace
    steps.push(SandboxStep {
        step: 1,
        description: "Create PID/mount/net namespace".to_string(),
        command: Some(format!(
            "unshare --pid --mount --net --fork --map-root-user -- /bin/true # ns={namespace_id}"
        )),
    });

    // Step 2: Overlay mount
    let lower = overlay
        .lower_dirs
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(":");
    steps.push(SandboxStep {
        step: 2,
        description: "Mount overlayfs (lower=inputs, upper=tmpfs)".to_string(),
        command: Some(format!(
            "mount -t overlay overlay -o lowerdir={lower},upperdir={},workdir={} {}",
            overlay.upper_dir.display(),
            overlay.work_dir.display(),
            overlay.merged_dir.display(),
        )),
    });

    // Step 3: Bind inputs read-only
    for (name, path) in input_paths {
        steps.push(SandboxStep {
            step: 3,
            description: format!("Bind input '{name}' read-only"),
            command: Some(format!(
                "mount --bind --read-only {} {}/inputs/{name}",
                path.display(),
                overlay.merged_dir.display(),
            )),
        });
    }

    // Step 4: cgroup limits
    steps.push(SandboxStep {
        step: 4,
        description: format!(
            "Apply cgroup limits (memory={}MB, cpus={})",
            config.memory_mb, config.cpus
        ),
        command: Some(format!(
            "mkdir -p {cg} && echo {mem} > {cg}/memory.max && echo {cpu_quota} 100000 > {cg}/cpu.max",
            cg = cgroup_path,
            mem = config.memory_mb * 1024 * 1024,
            cpu_quota = (config.cpus * 100_000.0) as u64,
        )),
    });

    // Step 5: Seccomp BPF (Full level only)
    if !seccomp_rules.is_empty() {
        let denied: Vec<&str> = seccomp_rules.iter().map(|r| r.syscall.as_str()).collect();
        steps.push(SandboxStep {
            step: 5,
            description: format!("Apply seccomp BPF (deny: {})", denied.join(", ")),
            command: Some(format!(
                "seccomp-bpf --deny {} -- /bin/sh",
                denied.join(",")
            )),
        });
    }

    // Step 6: Execute build script
    let script_hash = blake3::hash(script.as_bytes());
    steps.push(SandboxStep {
        step: 6,
        description: format!(
            "Execute bashrs-purified build (script hash: {})",
            &script_hash.to_hex()[..16]
        ),
        command: Some(format!(
            "timeout {}s nsenter --target $PID --pid --mount --net -- /bin/sh -c '{}'",
            config.timeout,
            script.replace('\'', "'\\''"),
        )),
    });

    // Step 7: Extract outputs
    let out_dir = overlay.merged_dir.join("out");
    steps.push(SandboxStep {
        step: 7,
        description: "Extract outputs from $out".to_string(),
        command: Some(format!("test -d {}", out_dir.display())),
    });

    // Step 8: hash_directory
    steps.push(SandboxStep {
        step: 8,
        description: "Compute BLAKE3 hash of output directory".to_string(),
        command: Some(format!("forjar-hash-dir {}", out_dir.display())),
    });

    // Step 9: Atomic move to store
    steps.push(SandboxStep {
        step: 9,
        description: "Atomic move to content-addressed store".to_string(),
        command: Some(format!(
            "mv {} {}/HASH/content",
            out_dir.display(),
            store_dir.display(),
        )),
    });

    // Step 10: Destroy namespace
    steps.push(SandboxStep {
        step: 10,
        description: "Destroy namespace and clean up".to_string(),
        command: Some(format!(
            "umount {merged} && rm -rf {root}",
            merged = overlay.merged_dir.display(),
            root = build_root.display(),
        )),
    });

    SandboxPlan {
        steps,
        namespace_id,
        overlay,
        seccomp_rules,
        cgroup_path,
    }
}

/// Generate seccomp BPF rules for a given sandbox level.
pub fn seccomp_rules_for_level(level: SandboxLevel) -> Vec<SeccompRule> {
    match level {
        SandboxLevel::Full => vec![
            SeccompRule {
                syscall: "connect".to_string(),
                action: "deny".to_string(),
            },
            SeccompRule {
                syscall: "mount".to_string(),
                action: "deny".to_string(),
            },
            SeccompRule {
                syscall: "ptrace".to_string(),
                action: "deny".to_string(),
            },
        ],
        _ => Vec::new(),
    }
}

/// Validate that a sandbox plan is well-formed.
pub fn validate_plan(plan: &SandboxPlan) -> Vec<String> {
    let mut errors = Vec::new();

    if plan.steps.is_empty() {
        errors.push("sandbox plan has no steps".to_string());
    }
    if plan.namespace_id.is_empty() {
        errors.push("namespace_id cannot be empty".to_string());
    }
    if plan.overlay.lower_dirs.is_empty() {
        errors.push("overlay requires at least one lower directory".to_string());
    }

    // Verify step ordering
    let mut prev_step = 0u8;
    for step in &plan.steps {
        if step.step < prev_step {
            errors.push(format!(
                "step {} appears after step {} (out of order)",
                step.step, prev_step
            ));
        }
        prev_step = step.step;
    }

    errors
}

/// Simulate sandbox execution (dry-run) and produce a result.
///
/// Used for testing and CI gates — computes what the sandbox WOULD produce
/// without actually creating namespaces or mounts.
pub fn simulate_sandbox_build(
    config: &SandboxConfig,
    build_hash: &str,
    input_paths: &BTreeMap<String, PathBuf>,
    script: &str,
    store_dir: &Path,
) -> SandboxResult {
    let plan = plan_sandbox_build(config, build_hash, input_paths, script, store_dir);

    // Simulate: the output hash is derived from inputs + script
    let mut hash_inputs: Vec<&str> = input_paths
        .values()
        .map(|p| p.to_str().unwrap_or(""))
        .collect();
    hash_inputs.sort();
    hash_inputs.push(script);
    let output_hash = crate::tripwire::hasher::composite_hash(&hash_inputs);

    let hash_bare = output_hash.strip_prefix("blake3:").unwrap_or(&output_hash);
    let store_path = format!("{}/{hash_bare}/content", store_dir.display());

    SandboxResult {
        output_hash,
        store_path,
        steps_executed: plan.steps.iter().map(|s| s.description.clone()).collect(),
    }
}

/// Count the total steps in a plan (for progress reporting).
pub fn plan_step_count(plan: &SandboxPlan) -> usize {
    plan.steps.len()
}
