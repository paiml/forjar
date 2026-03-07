//! Example: Infrastructure mutation testing runner.
//!
//! Demonstrates mutation test execution with local sandbox safety.
//! Run with: `cargo run --example mutation_runner`

fn main() {
    use forjar::core::store::convergence_runner;
    use forjar::core::store::mutation_runner;
    use forjar::core::types::MutationOperator;

    println!("=== Infrastructure Mutation Testing (FJ-2604) ===\n");

    // Show backend detection
    let run_config = mutation_runner::MutationRunConfig::default();
    let mode = convergence_runner::resolve_mode(run_config.backend);
    println!("Backend: {} (mode: {mode})\n", run_config.backend);

    // 1. Show applicable operators per resource type
    for rtype in &["file", "service", "package", "mount"] {
        let ops = mutation_runner::applicable_operators(rtype);
        let names: Vec<_> = ops.iter().map(|o| o.to_string()).collect();
        println!("  {rtype}: [{}]", names.join(", "));
    }

    // 2. Show mutation scripts
    println!("\nMutation scripts:");
    for op in &[
        MutationOperator::DeleteFile,
        MutationOperator::StopService,
        MutationOperator::RemovePackage,
    ] {
        println!(
            "  {op}: {}",
            mutation_runner::mutation_script(*op, "example")
        );
    }

    // 3. Run mutation suite with sandbox-safe file targets
    let targets = vec![
        mutation_runner::MutationTarget {
            resource_id: "app-config".into(),
            resource_type: "file".into(),
            apply_script: r#"mkdir -p "$FORJAR_SANDBOX/etc/forjar" && echo 'port: 8080' > "$FORJAR_SANDBOX/etc/forjar/app-config""#.into(),
            drift_script: r#"cat "$FORJAR_SANDBOX/etc/forjar/app-config" 2>/dev/null || echo 'MISSING'"#.into(),
            expected_hash: String::new(),
        },
        mutation_runner::MutationTarget {
            resource_id: "db-config".into(),
            resource_type: "file".into(),
            apply_script: r#"mkdir -p "$FORJAR_SANDBOX/etc/forjar" && echo 'host: localhost' > "$FORJAR_SANDBOX/etc/forjar/db-config""#.into(),
            drift_script: r#"cat "$FORJAR_SANDBOX/etc/forjar/db-config" 2>/dev/null || echo 'MISSING'"#.into(),
            expected_hash: String::new(),
        },
        mutation_runner::MutationTarget {
            resource_id: "nginx-svc".into(),
            resource_type: "service".into(),
            apply_script: r#"mkdir -p "$FORJAR_SANDBOX/run" && echo 'running' > "$FORJAR_SANDBOX/run/nginx-svc.pid""#.into(),
            drift_script: r#"cat "$FORJAR_SANDBOX/run/nginx-svc.pid" 2>/dev/null || echo 'STOPPED'"#.into(),
            expected_hash: String::new(),
        },
    ];

    println!("\nRunning mutation suite ({} targets)...\n", targets.len());
    let config = mutation_runner::MutationRunConfig {
        parallelism: 2,
        test_reconvergence: false,
        ..mutation_runner::MutationRunConfig::default()
    };
    let report = mutation_runner::run_mutation_parallel(targets, &config);

    print!("{}", mutation_runner::format_mutation_run(&report));

    // Show safety: system operators are rejected locally
    println!("\nSafety: system operators rejected in local mode:");
    for op in &[
        MutationOperator::StopService,
        MutationOperator::RemovePackage,
        MutationOperator::KillProcess,
    ] {
        println!("  {op}: requires container backend (skipped locally)");
    }
}
