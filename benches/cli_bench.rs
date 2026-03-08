//! FJ-2900: CLI pipeline benchmarks — codegen, score, lint.
//!
//! Criterion.rs benchmarks for the parse → plan → codegen pipeline
//! and recipe quality scoring.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use forjar::core::{codegen, parser, planner, resolver, scoring};

fn build_config_yaml(n_resources: usize) -> String {
    let mut yaml = String::from(
        "version: \"1.0\"\nname: bench\ndescription: \"Benchmark config\"\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n",
    );
    for i in 0..n_resources {
        if i % 3 == 0 {
            yaml.push_str(&format!(
                "  pkg-{i}:\n    type: package\n    machine: m\n    provider: apt\n    packages: [pkg-{i}]\n"
            ));
        } else if i % 3 == 1 {
            yaml.push_str(&format!(
                "  file-{i}:\n    type: file\n    machine: m\n    path: /etc/conf-{i}.yml\n    content: \"val-{i}\"\n    mode: \"0644\"\n    owner: root\n"
            ));
        } else {
            yaml.push_str(&format!(
                "  svc-{i}:\n    type: service\n    machine: m\n    name: svc-{i}\n"
            ));
        }
    }
    yaml
}

fn bench_codegen_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("codegen");
    let yaml = build_config_yaml(20);
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("forjar.yaml");
    std::fs::write(&file, &yaml).unwrap();
    let config = parser::parse_and_validate(&file).unwrap();

    for (_id, resource) in &config.resources {
        group.bench_with_input(
            BenchmarkId::new("apply_script", &resource.resource_type),
            resource,
            |b, r| {
                b.iter(|| {
                    let _ = black_box(codegen::apply_script(r));
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("check_script", &resource.resource_type),
            resource,
            |b, r| {
                b.iter(|| {
                    let _ = black_box(codegen::check_script(r));
                });
            },
        );
        // Only bench first of each type
        break;
    }
    group.finish();
}

fn bench_score(c: &mut Criterion) {
    let mut group = c.benchmark_group("score");
    for (label, n) in [("5r", 5), ("20r", 20)] {
        let yaml = build_config_yaml(n);
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(&file, &yaml).unwrap();

        let input = scoring::ScoringInput {
            status: "qualified".to_string(),
            idempotency: "strong".to_string(),
            budget_ms: 0,
            runtime: None,
            raw_yaml: Some(yaml.clone()),
        };

        group.bench_with_input(BenchmarkId::new("compute", label), &input, |b, inp| {
            b.iter(|| {
                let _ = black_box(scoring::compute_from_file(&file, inp));
            });
        });
    }
    group.finish();
}

fn bench_full_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline");
    for (label, n) in [("5r", 5), ("20r", 20), ("100r", 100)] {
        let yaml = build_config_yaml(n);
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(&file, &yaml).unwrap();

        group.bench_with_input(
            BenchmarkId::new("parse_plan_codegen", label),
            &file,
            |b, f| {
                b.iter(|| {
                    let config = parser::parse_and_validate(f).unwrap();
                    let order = resolver::build_execution_order(&config).unwrap();
                    let locks = std::collections::HashMap::new();
                    let plan = planner::plan(&config, &order, &locks, None);
                    for (id, _) in &config.resources {
                        if let Some(r) = config.resources.get(id) {
                            let _ = black_box(codegen::apply_script(r));
                        }
                    }
                    black_box(plan)
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_codegen_pipeline,
    bench_score,
    bench_full_pipeline
);
criterion_main!(benches);
