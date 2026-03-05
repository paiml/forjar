//! FJ-2105: Distribution — load, push, FAR export, and multi-arch builds.
//!
//! ```bash
//! cargo run --example distribution
//! ```

use forjar::core::types::{
    ArchBuild, BuildReport, DistResult, DistTarget, LayerReport, PushKind, PushResult,
};

fn main() {
    // Distribution targets
    let targets = vec![
        DistTarget::Load { runtime: "docker".into() },
        DistTarget::Push {
            registry: "ghcr.io".into(),
            name: "myorg/training".into(),
            tag: "2.1.0-cuda12.4.1".into(),
        },
        DistTarget::Far { output_path: "training-image.far".into() },
    ];

    println!("=== Distribution Targets ===");
    for t in &targets {
        println!("  {}", t.description());
    }
    println!();

    // Simulate registry push
    let push_results = vec![
        PushResult {
            kind: PushKind::Layer,
            digest: "sha256:base_layer".into(),
            size: 80_000_000,
            existed: true,
            duration_secs: 0.0,
        },
        PushResult {
            kind: PushKind::Layer,
            digest: "sha256:ml_deps".into(),
            size: 1_200_000_000,
            existed: false,
            duration_secs: 15.3,
        },
        PushResult {
            kind: PushKind::Config,
            digest: "sha256:config".into(),
            size: 2048,
            existed: false,
            duration_secs: 0.1,
        },
        PushResult {
            kind: PushKind::Manifest,
            digest: "sha256:manifest".into(),
            size: 512,
            existed: false,
            duration_secs: 0.2,
        },
    ];

    println!("=== Registry Push ===");
    for r in &push_results {
        let status = if r.existed { "existed" } else { "uploaded" };
        let mb = r.size as f64 / (1024.0 * 1024.0);
        println!(
            "  {:?}: {} ({status}, {mb:.1} MB, {:.1}s)",
            r.kind, r.digest, r.duration_secs,
        );
    }
    println!();

    // Multi-arch build
    let arches = vec![
        ArchBuild {
            manifest_digest: Some("sha256:amd64_manifest".into()),
            duration_secs: Some(48.5),
            ..ArchBuild::linux_amd64()
        },
        ArchBuild {
            manifest_digest: Some("sha256:arm64_manifest".into()),
            duration_secs: Some(72.3),
            ..ArchBuild::linux_arm64()
        },
    ];

    println!("=== Multi-Arch Builds ===");
    for a in &arches {
        println!(
            "  {}: {} ({:.1}s)",
            a.platform,
            a.manifest_digest.as_deref().unwrap_or("pending"),
            a.duration_secs.unwrap_or(0.0),
        );
    }
    println!();

    // Complete build report
    let report = BuildReport {
        image_ref: "ghcr.io/myorg/training:2.1.0-cuda12.4.1".into(),
        digest: "sha256:final_digest".into(),
        total_size: 1_300_000_000,
        layer_count: 3,
        duration_secs: 48.5,
        layers: vec![
            LayerReport {
                index: 0,
                name: "python-runtime".into(),
                store_hash: "blake3:aaa".into(),
                size: 80_000_000,
                cached: true,
                duration_secs: 0.2,
            },
            LayerReport {
                index: 1,
                name: "ml-deps".into(),
                store_hash: "blake3:bbb".into(),
                size: 1_200_000_000,
                cached: false,
                duration_secs: 47.3,
            },
            LayerReport {
                index: 2,
                name: "training-code".into(),
                store_hash: "blake3:ccc".into(),
                size: 20_000_000,
                cached: false,
                duration_secs: 0.01,
            },
        ],
        distribution: vec![DistResult {
            target: "ghcr.io/myorg/training:2.1.0-cuda12.4.1".into(),
            success: true,
            duration_secs: 15.6,
            error: None,
        }],
        architectures: arches,
    };

    println!("=== Build Report ===");
    print!("{}", report.format_summary());
}
