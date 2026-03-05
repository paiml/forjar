//! Demonstrates FJ-2301 image build log types for per-layer output capture.

use forjar::core::types::{ImageBuildLog, LayerBuildLog};

fn main() {
    // Per-layer log entries
    println!("=== Layer Build Logs ===");
    let cached = LayerBuildLog::cached("base-system", 0);
    println!("{cached}");

    let mut packages = LayerBuildLog::new("python-runtime", 1, 12.5);
    packages.log_bytes = 8192;
    packages.exit_code = Some(0);
    packages.log_path = Some(LayerBuildLog::default_log_path("training-image", 1));
    println!("{packages}");
    println!("  Log path: {}", packages.log_path.as_deref().unwrap());

    let mut build = LayerBuildLog::new("ml-deps", 2, 47.3);
    build.log_bytes = 1_048_576;
    build.exit_code = Some(0);
    build.stderr_tail = Some("Successfully installed torch-2.2.0".into());
    println!("{build}");

    let files = LayerBuildLog::new("training-code", 3, 0.01);
    println!("{files}");

    // Failed layer
    let mut failed = LayerBuildLog::new("broken-dep", 4, 30.0);
    failed.exit_code = Some(1);
    failed.stderr_tail = Some("error: could not compile".into());
    failed.log_bytes = 512;
    println!("{failed}");
    println!("  Succeeded: {}", failed.succeeded());

    // Complete image build log
    println!("\n=== Image Build Log ===");
    let log = ImageBuildLog {
        image_ref: "myregistry.io/training:2.1.0-cuda12.4.1".into(),
        layers: vec![cached, packages, build, files],
        manifest_log: Some("state/builds/training-image/manifest.log".into()),
        push_log: None,
        total_duration_secs: 60.1,
    };
    println!("{log}");
    println!("Cached layers: {}", log.cached_count());
    println!("Failed layers: {}", log.failed_count());
    println!("Total log bytes: {}", log.total_log_bytes());
    println!("All succeeded: {}", log.all_succeeded());
    println!("Build dir: {}", ImageBuildLog::build_dir("training-image"));
}
