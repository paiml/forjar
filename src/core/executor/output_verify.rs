//! FJ-2731 (PMAT-200): a task that declares outputs must produce them.
//!
//! # The defect
//!
//! `apply`'s success test was the script's exit code and nothing else. A task
//! could exit 0 having produced none of its declared `output_artifacts` and be
//! recorded `Converged`.
//!
//! That is not hypothetical. The transport writes the whole script to `bash`'s
//! STDIN, so a command that READS stdin consumes the rest of its own script.
//! Measured on the published 1.12.1 binary with a two-line recipe:
//!
//! ```text
//!   command: |
//!     cat > eaten.txt
//!     echo SECOND-LINE-RAN > second.txt
//!   output_artifacts: ["second.txt"]
//!
//!   apply  -> "Apply complete: 1 converged"
//!   eaten.txt contains: echo SECOND-LINE-RAN > second.txt   # line 2, eaten
//!   second.txt         : does not exist
//! ```
//!
//! Line 2 never ran, the declared artifact was never created, and apply called
//! it converged — then said `1 converged` again on every subsequent run,
//! breaking f(f(x)) = f(x) at the apply level. `check` and `plan` both caught
//! it; only `apply` did not.
//!
//! # Why the check belongs here rather than in the transport
//!
//! Fixing the stdin theft (a separate change) removes THIS cause. It does not
//! remove the class: a script can exit 0 without producing its outputs for many
//! reasons — a swallowed error, a wrong path, a tool that fails soft. The
//! release principle is "absence of evidence is not success", and apply was the
//! last read path still exempt from it.
//!
//! # Local only
//!
//! Verification runs on the controller's filesystem, so it applies only to
//! resources targeting a local machine — exactly the rule
//! `core::task::probe::probe_all` already follows. Checking a remote target's
//! artifacts against this host would compare the wrong filesystem, which is a
//! worse failure than not checking.

use crate::core::types::{Machine, Resource};
use crate::transport;

/// Declared `output_artifacts` that do not exist after a successful apply.
///
/// Returns an empty vec when there is nothing to verify: no declared outputs,
/// or a non-local target.
pub(crate) fn missing_outputs(resource: &Resource, machine: &Machine) -> Vec<String> {
    if resource.output_artifacts.is_empty() || !crate::transport::machine_is_local(machine) {
        return Vec::new();
    }
    let base = crate::core::task::probe::probe_base_dir(resource);
    resource
        .output_artifacts
        .iter()
        .filter(|a| !crate::core::task::probe::resolve_under(&base, a).exists())
        .cloned()
        .collect()
}

/// The apply-path entry point: `Some(error)` when a resource that exited 0 did
/// not produce its declared outputs, `None` when there is nothing to answer for.
pub(crate) fn unproduced_outputs_error(resource: &Resource, machine: &Machine) -> Option<String> {
    let missing = missing_outputs(resource, machine);
    if missing.is_empty() {
        None
    } else {
        Some(missing_outputs_error(&missing))
    }
}

/// Human-readable failure for a resource that exited 0 without its outputs.
pub(crate) fn missing_outputs_error(missing: &[String]) -> String {
    format!(
        "command exited 0 but declared output artifact(s) were not produced: {}. \
         The resource is NOT converged — a script can exit 0 without doing its \
         job (a swallowed error, a wrong path, or a command that consumed the \
         rest of the script from stdin).",
        missing.join(", ")
    )
}

/// Run the pre_apply hook; returns error string on failure.
pub(crate) fn run_pre_apply_hook(
    machine: &Machine,
    hook: &str,
    timeout: Option<u64>,
) -> Option<String> {
    match transport::exec_script_timeout(machine, hook, timeout) {
        Ok(out) if !out.success() => Some(format!(
            "pre_apply hook failed (exit {}): {}",
            out.exit_code,
            out.stderr.trim()
        )),
        Err(e) => Some(format!("pre_apply hook error: {e}")),
        _ => None,
    }
}

/// Run post_apply hook; returns error string on failure.
pub(crate) fn check_post_hook(
    machine: &Machine,
    hook: &str,
    timeout: Option<u64>,
) -> Option<String> {
    match transport::exec_script_timeout(machine, hook, timeout) {
        Ok(pout) if !pout.success() => Some(format!(
            "post_apply hook failed (exit {}): {}",
            pout.exit_code,
            pout.stderr.trim()
        )),
        Err(e) => Some(format!("post_apply hook error: {e}")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::ResourceType;

    fn local() -> Machine {
        serde_yaml_ng::from_str("hostname: localhost\naddr: localhost\n").unwrap()
    }
    fn remote() -> Machine {
        serde_yaml_ng::from_str("hostname: far\naddr: 10.9.9.9\n").unwrap()
    }
    fn task(dir: &std::path::Path, outs: &[&str]) -> Resource {
        Resource {
            resource_type: ResourceType::Task,
            output_artifacts: outs.iter().map(|s| s.to_string()).collect(),
            working_dir: Some(dir.display().to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn a_missing_declared_artifact_is_reported() {
        let d = tempfile::tempdir().unwrap();
        let r = task(d.path(), &["second.txt"]);
        assert_eq!(
            missing_outputs(&r, &local()),
            vec!["second.txt".to_string()]
        );
    }

    #[test]
    fn a_produced_artifact_is_not_reported() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("second.txt"), "ok").unwrap();
        assert!(missing_outputs(&task(d.path(), &["second.txt"]), &local()).is_empty());
    }

    #[test]
    fn only_the_missing_ones_are_named() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("there.txt"), "ok").unwrap();
        assert_eq!(
            missing_outputs(&task(d.path(), &["there.txt", "gone.txt"]), &local()),
            vec!["gone.txt".to_string()]
        );
    }

    #[test]
    fn a_directory_artifact_counts_as_produced() {
        // Consistent with the staleness probe, which identifies a directory
        // artifact by EXISTENCE — hashing its contents was the v1.11.0
        // idempotency pump.
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("build")).unwrap();
        assert!(missing_outputs(&task(d.path(), &["build"]), &local()).is_empty());
    }

    #[test]
    fn a_resource_declaring_no_outputs_is_not_verified() {
        // Most infra resources declare nothing; they must be unaffected.
        let d = tempfile::tempdir().unwrap();
        assert!(missing_outputs(&task(d.path(), &[]), &local()).is_empty());
    }

    #[test]
    fn a_remote_resource_is_never_verified_against_this_host() {
        // The artifact lives on the far machine. Checking the controller's
        // filesystem would fail every remote task that works perfectly.
        let d = tempfile::tempdir().unwrap();
        assert!(
            missing_outputs(&task(d.path(), &["second.txt"]), &remote()).is_empty(),
            "a remote target must not be judged by this host's filesystem"
        );
    }

    #[test]
    fn the_apply_entry_point_is_silent_when_there_is_nothing_to_answer_for() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("x"), "ok").unwrap();
        assert!(unproduced_outputs_error(&task(d.path(), &["x"]), &local()).is_none());
        assert!(unproduced_outputs_error(&task(d.path(), &[]), &local()).is_none());
        assert!(unproduced_outputs_error(&task(d.path(), &["gone"]), &remote()).is_none());
        assert!(unproduced_outputs_error(&task(d.path(), &["gone"]), &local()).is_some());
    }

    #[test]
    fn the_error_names_the_artifacts_and_the_likely_cause() {
        let e = missing_outputs_error(&["a.txt".into(), "b.txt".into()]);
        assert!(e.contains("a.txt") && e.contains("b.txt"), "{e}");
        assert!(e.contains("NOT converged"), "{e}");
    }
}
