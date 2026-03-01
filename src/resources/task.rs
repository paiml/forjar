//! ALB-027: Task resource handler.
//!
//! Runs an arbitrary command, tracks exit code, hashes output artifacts
//! for idempotency, supports completion_check and timeout.

use crate::core::types::Resource;

/// Generate shell script to check if a task has already completed.
///
/// If `completion_check` is set, runs it: exit 0 = already done.
/// If `output_artifacts` are set, checks if all exist.
/// Otherwise, always reports as needing execution.
pub fn check_script(resource: &Resource) -> String {
    if let Some(ref check) = resource.completion_check {
        return format!(
            "if {check}; then echo 'task=completed'; else echo 'task=pending'; fi"
        );
    }

    if !resource.output_artifacts.is_empty() {
        let checks: Vec<String> = resource
            .output_artifacts
            .iter()
            .map(|a| format!("[ -e '{a}' ]"))
            .collect();
        return format!(
            "if {} ; then echo 'task=completed'; else echo 'task=pending'; fi",
            checks.join(" && ")
        );
    }

    "echo 'task=pending'".to_string()
}

/// Generate shell script to execute the task command.
///
/// - Uses `set -euo pipefail` for strict error handling
/// - Supports `working_dir` to cd before execution
/// - Supports `timeout` for time-limited execution
pub fn apply_script(resource: &Resource) -> String {
    let command = resource.command.as_deref().unwrap_or("true");

    let mut script = String::from("set -euo pipefail\n");

    // Change to working directory if specified
    if let Some(ref dir) = resource.working_dir {
        script.push_str(&format!("cd '{}'\n", dir));
    }

    // Wrap command with timeout if specified
    if let Some(timeout_secs) = resource.timeout {
        script.push_str(&format!("timeout {} {}\n", timeout_secs, command));
    } else {
        script.push_str(command);
        script.push('\n');
    }

    script
}

/// Generate shell to query task state (for BLAKE3 hashing).
///
/// Hashes output_artifacts if specified, otherwise reports command string.
pub fn state_query_script(resource: &Resource) -> String {
    if !resource.output_artifacts.is_empty() {
        let hash_cmds: Vec<String> = resource
            .output_artifacts
            .iter()
            .map(|a| format!("[ -f '{a}' ] && b3sum '{a}' 2>/dev/null || echo 'missing:{a}'"))
            .collect();
        return hash_cmds.join("\n");
    }

    let command = resource.command.as_deref().unwrap_or("true");
    format!("echo 'command={}'", command)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{MachineTarget, ResourceType};

    fn make_task_resource(cmd: &str) -> Resource {
        Resource {
            resource_type: ResourceType::Task,
            machine: MachineTarget::Single("worker".to_string()),
            command: Some(cmd.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn test_check_no_completion_check_no_artifacts() {
        let r = make_task_resource("echo hello");
        let script = check_script(&r);
        assert_eq!(script, "echo 'task=pending'");
    }

    #[test]
    fn test_check_with_completion_check() {
        let mut r = make_task_resource("train model");
        r.completion_check = Some("test -f model.bin".to_string());
        let script = check_script(&r);
        assert!(script.contains("test -f model.bin"));
        assert!(script.contains("task=completed"));
        assert!(script.contains("task=pending"));
    }

    #[test]
    fn test_check_with_output_artifacts() {
        let mut r = make_task_resource("build");
        r.output_artifacts = vec!["out/model.bin".to_string(), "out/vocab.json".to_string()];
        let script = check_script(&r);
        assert!(script.contains("[ -e 'out/model.bin' ]"));
        assert!(script.contains("[ -e 'out/vocab.json' ]"));
        assert!(script.contains("task=completed"));
    }

    #[test]
    fn test_apply_basic() {
        let r = make_task_resource("apr train apply config.yaml");
        let script = apply_script(&r);
        assert!(script.contains("set -euo pipefail"));
        assert!(script.contains("apr train apply config.yaml"));
    }

    #[test]
    fn test_apply_with_working_dir() {
        let mut r = make_task_resource("make build");
        r.working_dir = Some("/opt/project".to_string());
        let script = apply_script(&r);
        assert!(script.contains("cd '/opt/project'"));
        assert!(script.contains("make build"));
    }

    #[test]
    fn test_apply_with_timeout() {
        let mut r = make_task_resource("long-running-train");
        r.timeout = Some(3600);
        let script = apply_script(&r);
        assert!(script.contains("timeout 3600 long-running-train"));
    }

    #[test]
    fn test_state_query_with_artifacts() {
        let mut r = make_task_resource("train");
        r.output_artifacts = vec!["model.bin".to_string()];
        let script = state_query_script(&r);
        assert!(script.contains("b3sum 'model.bin'"));
        assert!(script.contains("missing:model.bin"));
    }

    #[test]
    fn test_state_query_no_artifacts() {
        let r = make_task_resource("echo hello");
        let script = state_query_script(&r);
        assert!(script.contains("command=echo hello"));
    }

    #[test]
    fn test_apply_no_command_defaults_to_true() {
        let mut r = make_task_resource("placeholder");
        r.command = None;
        let script = apply_script(&r);
        assert!(script.contains("true"));
    }
}
