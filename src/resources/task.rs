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
        return format!("if {check}; then echo 'task=completed'; else echo 'task=pending'; fi");
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
        script.push_str(&format!("cd '{dir}'\n"));
    }

    // Wrap command with timeout if specified.
    // Use a heredoc so multi-line commands and arbitrary quoting work
    // without escaping issues that break bashrs linting.
    if let Some(timeout_secs) = resource.timeout {
        script.push_str(&format!(
            "timeout {timeout_secs} bash <<'FORJAR_TIMEOUT'\n{command}\nFORJAR_TIMEOUT\n"
        ));
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
    format!("echo 'command={command}'")
}

/// FJ-2704: Generate shell script to scatter local artifacts to remote paths.
///
/// Each scatter entry is a "local:remote" mapping. Returns a script that copies
/// local files to their remote destinations before task execution.
pub fn scatter_script(resource: &Resource) -> Option<String> {
    if resource.scatter.is_empty() {
        return None;
    }
    let mut script = String::from("set -euo pipefail\n# FJ-2704: scatter artifacts\n");
    for mapping in &resource.scatter {
        if let Some((local, remote)) = mapping.split_once(':') {
            script.push_str(&format!(
                "mkdir -p \"$(dirname '{remote}')\"\ncp -r '{local}' '{remote}'\n"
            ));
        }
    }
    Some(script)
}

/// FJ-2704: Generate shell script to gather remote artifacts to local paths.
///
/// Each gather entry is a "remote:local" mapping. Returns a script that copies
/// remote files to their local destinations after task execution.
pub fn gather_script(resource: &Resource) -> Option<String> {
    if resource.gather.is_empty() {
        return None;
    }
    let mut script = String::from("set -euo pipefail\n# FJ-2704: gather artifacts\n");
    for mapping in &resource.gather {
        if let Some((remote, local)) = mapping.split_once(':') {
            script.push_str(&format!(
                "mkdir -p \"$(dirname '{local}')\"\ncp -r '{remote}' '{local}'\n"
            ));
        }
    }
    Some(script)
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
        assert!(script.contains("timeout 3600 bash <<'FORJAR_TIMEOUT'"));
        assert!(script.contains("long-running-train"));
        assert!(script.contains("FORJAR_TIMEOUT"));
    }

    #[test]
    fn test_apply_with_timeout_multiline() {
        let mut r = make_task_resource("git pull\ncargo build");
        r.timeout = Some(300);
        r.working_dir = Some("/opt/project".to_string());
        let script = apply_script(&r);
        assert!(script.contains("timeout 300 bash <<'FORJAR_TIMEOUT'"));
        assert!(script.contains("git pull\ncargo build"));
        assert!(script.contains("cd '/opt/project'"));
    }

    #[test]
    fn test_apply_with_timeout_quoting() {
        let mut r = make_task_resource("echo 'hello world'");
        r.timeout = Some(60);
        let script = apply_script(&r);
        // Heredoc preserves quotes without escaping
        assert!(script.contains("echo 'hello world'"));
        assert!(script.contains("timeout 60 bash <<'FORJAR_TIMEOUT'"));
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

    #[test]
    fn test_scatter_empty() {
        let r = make_task_resource("train");
        assert!(scatter_script(&r).is_none());
    }

    #[test]
    fn test_scatter_with_mappings() {
        let mut r = make_task_resource("train");
        r.scatter = vec![
            "/local/data.csv:/remote/data.csv".to_string(),
            "/local/config.yaml:/remote/config.yaml".to_string(),
        ];
        let script = scatter_script(&r).unwrap();
        assert!(script.contains("cp -r '/local/data.csv' '/remote/data.csv'"));
        assert!(script.contains("cp -r '/local/config.yaml' '/remote/config.yaml'"));
        assert!(script.contains("mkdir -p"));
    }

    #[test]
    fn test_gather_empty() {
        let r = make_task_resource("train");
        assert!(gather_script(&r).is_none());
    }

    #[test]
    fn test_gather_with_mappings() {
        let mut r = make_task_resource("train");
        r.gather = vec!["/remote/model.bin:/local/model.bin".to_string()];
        let script = gather_script(&r).unwrap();
        assert!(script.contains("cp -r '/remote/model.bin' '/local/model.bin'"));
        assert!(script.contains("# FJ-2704: gather artifacts"));
    }

    #[test]
    fn test_scatter_invalid_mapping_skipped() {
        let mut r = make_task_resource("train");
        r.scatter = vec!["no-colon-here".to_string()];
        let script = scatter_script(&r).unwrap();
        // Invalid mapping is skipped — no cp command generated
        assert!(!script.contains("cp"));
    }
}
