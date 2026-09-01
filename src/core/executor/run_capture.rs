//! FJ-2301: Run log capture — persists transport output to disk.
//!
//! Called after `exec_script_retry` in `execute_resource()` to write
//! `.log` and `.script` files into `state/<machine>/runs/<run_id>/`.

use crate::core::types::{ResourceRunStatus, ResourceType, RunLogEntry, RunMeta};
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
    let mut meta = RunMeta::new(
        run_id.to_string(),
        machine_name.to_string(),
        command.to_string(),
    );
    // Dogfood #208 (logs-gc-deletes-arbitrary-runs-including-the-newest): every
    // run must carry a start timestamp. When it was left `None` the retention
    // sort in `logs --gc` compared "" to "" for every run, the stable sort then
    // preserved readdir (hash) order, and GC deleted an arbitrary subset —
    // including the 2nd-newest run — while keeping the oldest.
    meta.started_at = Some(crate::tripwire::eventlog::now_iso8601());
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
    // FJ-2301: Uphold STRONG blake3-state-v1 precondition `!input.is_empty()`.
    // Scripts are genuinely optional (capture_exec_output passes "" when a
    // resource has no script). Keep empty-script → empty-hash so consumers
    // (log parsers, UIs) can distinguish "no script" from an actual hash —
    // this is why we do NOT use the sentinel wrapper here.
    let script_hash = if script.is_empty() {
        String::new()
    } else {
        crate::tripwire::hasher::hash_string(script)
    };

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

/// WHERE a run log goes: the run directory's three coordinates.
///
/// Refs #390: grouped rather than passed loose because the writer needs nine
/// values and a nine-argument function is how the old call site ended up
/// hard-coding `let rt = "unknown"` — the caller had the type and the signature
/// made passing it feel like one argument too many.
pub struct RunSlot<'a> {
    /// `--state-dir`. Relative by default.
    pub state_dir: &'a Path,
    /// Machine whose run directory this is.
    pub machine_name: &'a str,
    /// This apply's run id; `None` still writes a log, under `run-adhoc`.
    pub run_id: Option<&'a str>,
}

/// WHAT was executed: the resource identity and the script that ran.
pub struct Executed<'a> {
    /// Resource id, which names the log file.
    pub resource_id: &'a str,
    /// Recorded in the log header — see the `type: unknown` note below.
    pub resource_type: &'a ResourceType,
    /// `create` / `update`, which also names the log file.
    pub action: &'a str,
    /// The exact text handed to the transport.
    pub script: &'a str,
}

/// FJ-2301 / Refs #390: persist one execution's script, both streams and its
/// exit code — and RETURN the path that now holds them.
///
/// This lived as a private helper inside `resource_ops.rs`, and that is a large
/// part of why `machine_wave.rs` never grew one: a sibling module could not
/// call it without reaching through another module's wall, so under
/// `--parallel` a failing task's stdout was destroyed rather than merely hidden
/// (#390-A). In the module whose whole job is run logs, calling it from the
/// wave path becomes a one-liner rather than a refactor.
///
/// The RETURN VALUE is what keeps `failure_text` honest. A failure message
/// names a transcript only when the code that writes transcripts says it wrote
/// one — never because the message reconstructed a path it expected to exist.
/// #390's reporter had already been sent looking for evidence once.
pub fn capture_exec_output(
    slot: &RunSlot,
    executed: &Executed,
    output: &ExecOutput,
    duration_secs: f64,
) -> Option<PathBuf> {
    let (state_dir, machine_name, run_id) = (slot.state_dir, slot.machine_name, slot.run_id);
    let (resource_id, resource_type, action) = (
        executed.resource_id,
        executed.resource_type,
        executed.action,
    );
    let script = executed.script;
    // `run-adhoc` is preserved verbatim from the original helper: a run with no
    // id still writes a log, and dozens of in-tree callers build `ApplyConfig`
    // with `run_id: None` and do execute resources.
    let rid = run_id.unwrap_or("run-adhoc");
    let dir = run_dir(state_dir, machine_name, rid);
    ensure_run_dir(&dir, rid, machine_name, "apply");
    // Refs #390: the old call site hard-coded `let rt = "unknown"` with the
    // comment "resource type not available here", so every run log ever written
    // recorded `type: unknown`. It is available at the caller; pass it.
    let rt = format!("{resource_type:?}").to_lowercase();
    capture_output(
        &dir,
        resource_id,
        &rt,
        action,
        machine_name,
        "transport",
        script,
        output,
        duration_secs,
    );
    let log = dir.join(format!("{resource_id}.{action}.log"));
    log.exists().then_some(log)
}
