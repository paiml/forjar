//! Example: Infrastructure mutation testing runner.
//!
//! Demonstrates mutation test execution with parallel sandboxes.
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
        println!("  {op}: {}", mutation_runner::mutation_script(*op, "example"));
    }

    // 3. Run mutation suite
    let targets = vec![
        mutation_runner::MutationTarget {
            resource_id: "nginx-config".into(),
            resource_type: "file".into(),
            apply_script: "echo 'apply nginx config'".into(),
            drift_script: "echo 'check drift'".into(),
            expected_hash: "blake3:expected".into(),
        },
        mutation_runner::MutationTarget {
            resource_id: "nginx-svc".into(),
            resource_type: "service".into(),
            apply_script: "systemctl start nginx".into(),
            drift_script: "systemctl is-active nginx".into(),
            expected_hash: "blake3:expected".into(),
        },
        mutation_runner::MutationTarget {
            resource_id: "curl-pkg".into(),
            resource_type: "package".into(),
            apply_script: "apt install curl".into(),
            drift_script: "dpkg -l curl".into(),
            expected_hash: "blake3:expected".into(),
        },
    ];

    println!("\nRunning mutation suite ({} targets)...\n", targets.len());
    let config = mutation_runner::MutationRunConfig {
        parallelism: 2,
        ..mutation_runner::MutationRunConfig::default()
    };
    let report = mutation_runner::run_mutation_parallel(targets, &config);

    print!("{}", mutation_runner::format_mutation_run(&report));
}
