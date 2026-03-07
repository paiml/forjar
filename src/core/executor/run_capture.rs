//! FJ-2301: Run log capture — persists transport output to disk.
//!
//! Called after `exec_script_retry` in `execute_resource()` to write
//! `.log` and `.script` files into `state/<machine>/runs/<run_id>/`.

use crate::core::types::{ResourceRunStatus, RunLogEntry, RunMeta};
use crate::transport::ExecOutput;
use std::path::{Path, PathBuf};

/// Compute the run directory path.
pub fn run_dir(state_dir: &Path, machine_name: &str, run_id: &str) -> PathBuf {
    state_dir.join(machine_name).join("runs").join(run_id)
}

/// Ensure the run directory exists and write meta.yaml if it doesn't exist yet.
pub fn ensure_run_dir(dir: &Path, run_id: &str, machine_name: &str, command: &str) {
    if dir.exists() {
        return;
    }
    let _ = std::fs::create_dir_all(dir);
    let meta = RunMeta::new(
        run_id.to_string(),
        machine_name.to_string(),
        command.to_string(),
    );
    let _ = serde_yaml_ng::to_string(&meta).map(|yaml| std::fs::write(dir.join("meta.yaml"), yaml));
}

/// Capture transport output to a log file in the run directory.
///
/// Writes `<resource_id>.<action>.log` with structured sections,
/// and `<resource_id>.script` with the raw script.
#[allow(clippy::too_many_arguments)]
pub fn capture_output(
    run_dir: &Path,
    resource_id: &str,
    resource_type: &str,
    action: &str,
    machine_name: &str,
    transport_type: &str,
    script: &str,
    output: &ExecOutput,
    duration_secs: f64,
) {
    if !run_dir.exists() {
        return;
    }

    let now = crate::tripwire::eventlog::now_iso8601();
    let script_hash = crate::tripwire::hasher::hash_string(script);

    let entry = RunLogEntry {
        resource_id: resource_id.to_string(),
        resource_type: resource_type.to_string(),
        action: action.to_string(),
        machine: machine_name.to_string(),
        transport: transport_type.to_string(),
        script: script.to_string(),
        script_hash,
        stdout: output.stdout.clone(),
        stderr: output.stderr.clone(),
        exit_code: output.exit_code,
        duration_secs,
        started_at: now.clone(),
        finished_at: now,
    };

    let log_content = entry.format_log();
    let log_path = run_dir.join(format!("{resource_id}.{action}.log"));
    let _ = std::fs::write(log_path, log_content);

    // FJ-2301/E20: Also write structured JSON log for machine-parseable output
    let json_path = run_dir.join(format!("{resource_id}.{action}.json"));
    let _ = std::fs::write(json_path, entry.format_json());

    let script_path = run_dir.join(format!("{resource_id}.script"));
    let _ = std::fs::write(script_path, script);
}

/// Update meta.yaml with resource status after execution.
pub fn update_meta_resource(run_dir: &Path, resource_id: &str, status: ResourceRunStatus) {
    let meta_path = run_dir.join("meta.yaml");
    let mut meta = match std::fs::read_to_string(&meta_path) {
        Ok(content) => serde_yaml_ng::from_str::<RunMeta>(&content)
            .unwrap_or_else(|_| RunMeta::new("unknown".into(), "unknown".into(), "apply".into())),
        Err(_) => return,
    };
    meta.record_resource(resource_id, status);
    let _ = serde_yaml_ng::to_string(&meta).map(|yaml| std::fs::write(&meta_path, yaml));
    // FJ-2301/E20: Also write meta.json for structured access
    let _ = serde_json::to_string_pretty(&meta)
        .map(|json| std::fs::write(run_dir.join("meta.json"), json));
}
