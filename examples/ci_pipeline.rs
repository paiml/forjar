//! Demonstrates FJ-2400/2403 CI pipeline types: reproducible builds, MSRV, feature matrix.

use forjar::core::types::{
    FeatureMatrix, ModelIntegrityCheck, MsrvCheck, PurificationBenchmark, ReproBuildConfig,
};

fn main() {
    // Reproducible build config
    println!("=== Reproducible Build ===");
    let config = ReproBuildConfig::default();
    println!("  Reproducible: {}", config.is_reproducible());
    println!("  Cargo args: {:?}", config.cargo_args());
    println!("  Env vars: {:?}", config.env_vars());

    let with_epoch = ReproBuildConfig {
        source_date_epoch: Some(1709683200),
        ..Default::default()
    };
    println!("  With epoch env: {:?}", with_epoch.env_vars());

    // MSRV enforcement
    println!("\n=== MSRV Check ===");
    let msrv = MsrvCheck::new("1.88.0");
    for version in ["1.87.0", "1.88.0", "1.89.0", "2.0.0"] {
        println!(
            "  {} >= {}: {}",
            version,
            msrv.required,
            msrv.satisfies(version),
        );
    }

    // Feature flag matrix
    println!("\n=== Feature Matrix ===");
    let matrix = FeatureMatrix::new(vec!["encryption", "container-test"]);
    println!("  Features: {:?}", matrix.features);
    println!("  Combinations ({}):", matrix.combinations().len());
    for cmd in matrix.cargo_commands() {
        println!("    {cmd}");
    }

    // Purification benchmarks
    println!("\n=== Purification Benchmarks ===");
    let benchmarks = vec![
        PurificationBenchmark {
            resource_type: "file".into(),
            validate_us: 45.0,
            purify_us: 120.0,
            sample_count: 500,
        },
        PurificationBenchmark {
            resource_type: "package".into(),
            validate_us: 30.0,
            purify_us: 85.0,
            sample_count: 200,
        },
        PurificationBenchmark {
            resource_type: "service".into(),
            validate_us: 25.0,
            purify_us: 70.0,
            sample_count: 150,
        },
    ];
    for b in &benchmarks {
        println!("  {b}");
    }

    // Model integrity verification
    println!("\n=== Model Integrity ===");
    let checks = vec![
        ModelIntegrityCheck::check("llama3-8b", "abc123def456", "abc123def456", 8_000_000_000),
        ModelIntegrityCheck::check("gpt2", "deadbeef1234", "deadbeef1234", 500_000_000),
        ModelIntegrityCheck::check("bert-base", "aaa111bbb222", "ccc333ddd444", 440_000_000),
    ];
    for c in &checks {
        println!("  {c}");
    }
}
