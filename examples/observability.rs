//! Demonstrates FJ-2301 observability types: log filtering, truncation, progress.

use forjar::core::types::{
    LogFilter, LogGcResult, LogTruncation, ProgressConfig, RunLogPath, StructuredLogOutput,
    VerbosityLevel,
};

fn main() {
    // Verbosity levels from CLI -v flags
    println!("=== Verbosity Levels ===");
    for count in 0..=3 {
        let v = VerbosityLevel::from_count(count);
        let flag = if count == 0 {
            "(none)".to_string()
        } else {
            format!("-{}", "v".repeat(count as usize))
        };
        println!(
            "  {flag}: {v} (scripts={}, raw={})",
            v.shows_scripts(),
            v.streams_raw(),
        );
    }

    // Log filtering
    println!("\n=== Log Filters ===");
    let machine_filter = LogFilter::for_machine("intel");
    println!("  Machine filter: {:?}", machine_filter.machine);
    println!("  Has criteria: {}", machine_filter.has_criteria());

    let failure_filter = LogFilter::failures();
    println!("  Failures only: {}", failure_filter.failures_only);

    // Log truncation
    println!("\n=== Log Truncation ===");
    let trunc = LogTruncation {
        first_bytes: 10,
        last_bytes: 10,
    };
    let small = "short log";
    let large = "ABCDEFGHIJ__this_is_the_middle_section_that_gets_cut__KLMNOPQRST";
    println!(
        "  Small log truncated: {}",
        trunc.should_truncate(small.len())
    );
    println!(
        "  Large log truncated: {}",
        trunc.should_truncate(large.len())
    );
    let truncated = trunc.truncate(large);
    println!("  Result:\n{truncated}");

    // Run log paths
    println!("\n=== Run Log Paths ===");
    let path = RunLogPath::new("state", "intel", "r-abc123");
    println!("  Run dir:      {}", path.run_dir());
    println!("  Meta:         {}", path.meta_path());
    println!(
        "  Apply log:    {}",
        path.resource_log("nginx-pkg", "apply")
    );
    println!(
        "  Check log:    {}",
        path.resource_log("nginx-svc", "check")
    );

    // GC result
    println!("\n=== Log GC ===");
    let gc = LogGcResult {
        runs_removed: 5,
        bytes_freed: 52_428_800,
        runs_kept: 10,
    };
    println!("  {gc}");

    // Structured JSON output
    println!("\n=== Structured Output ===");
    let out = StructuredLogOutput {
        run_id: "r-abc123".into(),
        machine: "intel".into(),
        resource_id: "nginx-pkg".into(),
        log_path: "state/intel/runs/r-abc123/nginx-pkg.apply.log".into(),
        exit_code: 0,
        duration_secs: 1.5,
        truncated: false,
    };
    println!("  {}", serde_json::to_string_pretty(&out).unwrap());

    // Progress config
    println!("\n=== Progress Config ===");
    let pc = ProgressConfig::default();
    println!("  Show progress: {}", pc.show_progress);
    println!("  Update interval: {}ms", pc.update_interval_ms);
}
