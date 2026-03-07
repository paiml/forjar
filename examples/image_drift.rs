//! FJ-2106/E15: Image drift detection demonstration.
//!
//! Shows how forjar detects drift in deployed container images by comparing
//! the running container's image digest to the expected manifest digest
//! from the build.
//!
//! ```bash
//! cargo run --example image_drift
//! ```

fn main() {
    println!("=== Image Drift Detection ===\n");
    println!("Image drift compares deployed container digests to built image digests.\n");

    // Simulate drift scenarios
    let scenarios = [
        ("app-server", "sha256:abc123", "sha256:abc123", true),
        ("web-frontend", "sha256:abc123", "sha256:def456", false),
        ("worker", "sha256:abc123", "NOT_RUNNING", false),
    ];

    for (name, expected, actual, ok) in &scenarios {
        if *ok {
            println!("  {name}: OK (deployed={expected})");
        } else if *actual == "NOT_RUNNING" {
            println!("  {name}: DRIFTED — container not running (expected {expected})");
        } else {
            println!(
                "  {name}: DRIFTED — deployed={actual}, expected={expected}"
            );
        }
    }

    println!("\n=== How It Works ===\n");
    println!("1. Build stores manifest_digest in state lock");
    println!("2. `forjar drift` runs: docker inspect <container> --format '{{{{.Image}}}}'");
    println!("3. Compares actual digest to expected digest");
    println!("4. Reports drift if they differ or container is not running");

    println!("\n=== CLI Usage ===\n");
    println!("  forjar drift -f config.yaml         # check all resources including images");
    println!("  forjar drift -f config.yaml --json   # JSON output");
    println!("  forjar drift -f config.yaml --tripwire   # exit non-zero on drift");
}
