//! Benchmarks for forjar core operations.
//!
//! Run with: cargo bench
//!
//! Results include 95% confidence intervals via Criterion.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn bench_blake3_string(c: &mut Criterion) {
    let mut group = c.benchmark_group("blake3_string");
    for size in [64, 256, 1024, 4096] {
        let input: String = "x".repeat(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &input, |b, input| {
            b.iter(|| {
                let hash = blake3::hash(black_box(input.as_bytes()));
                black_box(hash);
            });
        });
    }
    group.finish();
}

fn bench_blake3_file(c: &mut Criterion) {
    let dir = tempfile::tempdir().unwrap();

    let mut group = c.benchmark_group("blake3_file");
    for size_kb in [1, 64, 1024] {
        let path = dir.path().join(format!("bench_{size_kb}k.bin"));
        let data = vec![0xABu8; size_kb * 1024];
        std::fs::write(&path, &data).unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(size_kb), &path, |b, path| {
            b.iter(|| {
                let mut file = std::fs::File::open(black_box(path)).unwrap();
                let mut hasher = blake3::Hasher::new();
                let mut buf = [0u8; 65536];
                loop {
                    use std::io::Read;
                    let n = file.read(&mut buf).unwrap();
                    if n == 0 {
                        break;
                    }
                    hasher.update(&buf[..n]);
                }
                black_box(hasher.finalize());
            });
        });
    }
    group.finish();
}

fn bench_yaml_parse(c: &mut Criterion) {
    let yaml = r#"
version: "1.0"
name: bench-config
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
  m2:
    hostname: m2
    addr: 127.0.0.2
resources:
  base-packages:
    type: package
    machine: [m1, m2]
    provider: apt
    packages: [curl, wget, htop, vim, git, tmux, jq, ripgrep]
  config-file:
    type: file
    machine: m1
    path: /etc/app/config.yaml
    content: |
      database:
        host: localhost
        port: 5432
      cache:
        ttl: 300
    owner: app
    group: app
    mode: "0640"
    depends_on: [base-packages]
  app-service:
    type: service
    machine: m1
    name: app
    state: running
    enabled: true
    depends_on: [config-file]
"#;

    c.bench_function("yaml_parse_config", |b| {
        b.iter(|| {
            let config: serde_yaml::Value = serde_yaml::from_str(black_box(yaml)).unwrap();
            black_box(config);
        });
    });
}

fn bench_topo_sort(c: &mut Criterion) {
    // Build a linear chain of N nodes
    let mut group = c.benchmark_group("topo_sort");
    for n in [10, 50, 100] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                let mut in_degree: std::collections::HashMap<String, usize> =
                    std::collections::HashMap::new();
                let mut adjacency: std::collections::HashMap<String, Vec<String>> =
                    std::collections::HashMap::new();

                for i in 0..n {
                    let name = format!("node-{i:04}");
                    in_degree.entry(name.clone()).or_insert(0);
                    adjacency.entry(name.clone()).or_default();

                    if i > 0 {
                        let prev = format!("node-{:04}", i - 1);
                        adjacency.entry(prev).or_default().push(name.clone());
                        *in_degree.entry(name).or_insert(0) += 1;
                    }
                }

                // Kahn's algorithm
                let mut queue: std::collections::BinaryHeap<std::cmp::Reverse<String>> = in_degree
                    .iter()
                    .filter(|(_, &d)| d == 0)
                    .map(|(k, _)| std::cmp::Reverse(k.clone()))
                    .collect();

                let mut order = Vec::with_capacity(n);
                while let Some(std::cmp::Reverse(node)) = queue.pop() {
                    if let Some(neighbors) = adjacency.get(&node) {
                        for neighbor in neighbors {
                            if let Some(d) = in_degree.get_mut(neighbor) {
                                *d -= 1;
                                if *d == 0 {
                                    queue.push(std::cmp::Reverse(neighbor.clone()));
                                }
                            }
                        }
                    }
                    order.push(node);
                }
                black_box(order);
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_blake3_string,
    bench_blake3_file,
    bench_yaml_parse,
    bench_topo_sort
);
criterion_main!(benches);
