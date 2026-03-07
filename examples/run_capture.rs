//! FJ-2301: Run capture pipeline example.
//!
//! Demonstrates the end-to-end observability pipeline:
//! 1. Create a run directory with meta.yaml
//! 2. Capture simulated transport output to .log files
//! 3. Update meta.yaml with resource status
//! 4. Read back the logs using the log viewer
//!
//! ```bash
//! cargo run --example run_capture
//! ```

use forjar::core::executor::run_capture;
use forjar::core::types::ResourceRunStatus;
use forjar::transport::ExecOutput;

fn main() {
    let tmp = tempfile::tempdir().unwrap();
    let state_dir = tmp.path();
    let machine = "intel";
    let run_id = forjar::core::types::generate_run_id();

    println!("=== FJ-2301: Run Capture Pipeline ===\n");
    println!("  State dir: {}", state_dir.display());
    println!("  Run ID:    {run_id}");
    println!("  Machine:   {machine}\n");

    // 1. Create run directory with meta.yaml
    let dir = run_capture::run_dir(state_dir, machine, &run_id);
    run_capture::ensure_run_dir(&dir, &run_id, machine, "apply");
    println!("  Created: {}", dir.display());
    println!("  meta.yaml written\n");

    // 2. Simulate successful resource
    let ok_output = ExecOutput {
        exit_code: 0,
        stdout: "nginx is already the newest version (1.24.0-2).\n".into(),
        stderr: String::new(),
    };
    run_capture::capture_output(
        &dir, "nginx-pkg", "package", "apply", machine,
        "ssh", "apt-get install -y nginx", &ok_output, 1.2,
    );
    run_capture::update_meta_resource(
        &dir, "nginx-pkg",
        ResourceRunStatus::Converged {
            exit_code: Some(0), duration_secs: Some(1.2), failed: false,
        },
    );
    println!("  Captured: nginx-pkg.apply.log (success, 1.2s)");

    // 3. Simulate failed resource
    let fail_output = ExecOutput {
        exit_code: 100,
        stdout: "Reading package lists...\nBuilding dependency tree...\n".into(),
        stderr: "E: Unable to locate package cargo-watch\n".into(),
    };
    run_capture::capture_output(
        &dir, "cargo-tools", "package", "apply", machine,
        "ssh", "apt-get install -y cargo-watch", &fail_output, 0.8,
    );
    run_capture::update_meta_resource(
        &dir, "cargo-tools",
        ResourceRunStatus::Converged {
            exit_code: Some(100), duration_secs: Some(0.8), failed: true,
        },
    );
    println!("  Captured: cargo-tools.apply.log (FAILED, exit 100)\n");

    // 4. Read back meta.yaml
    let meta_str = std::fs::read_to_string(dir.join("meta.yaml")).unwrap();
    let meta: forjar::core::types::RunMeta = serde_yaml_ng::from_str(&meta_str).unwrap();
    println!("=== Meta.yaml Summary ===\n");
    println!("  Run:       {}", meta.run_id);
    println!("  Machine:   {}", meta.machine);
    println!("  Total:     {}", meta.summary.total);
    println!("  Converged: {}", meta.summary.converged);
    println!("  Failed:    {}", meta.summary.failed);
    println!();

    // 5. Read back a log file
    let log = std::fs::read_to_string(dir.join("cargo-tools.apply.log")).unwrap();
    println!("=== cargo-tools.apply.log ===\n");
    for line in log.lines().take(12) {
        println!("  {line}");
    }
    println!("  ...\n");

    // 6. List files in run directory
    println!("=== Run Directory Contents ===\n");
    for entry in std::fs::read_dir(&dir).unwrap().flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        println!("  {name} ({size} bytes)");
    }
}
