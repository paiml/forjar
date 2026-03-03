//! Benchmarks for forjar core operations (cargo bench).
//! Spec §9 targets: validate <10ms, plan(3m/20r) <2s, apply(no-change) <500ms, drift(100r) <1s.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use forjar::core::{parser, planner, resolver, state};
use forjar::tripwire::drift;

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
            let config: serde_yaml_ng::Value = serde_yaml_ng::from_str(black_box(yaml)).unwrap();
            black_box(config);
        });
    });
}

/// Build a linear chain DAG and return (in_degree, adjacency).
fn build_linear_chain(
    n: usize,
) -> (
    std::collections::HashMap<String, usize>,
    std::collections::HashMap<String, Vec<String>>,
) {
    let mut in_degree = std::collections::HashMap::new();
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
    (in_degree, adjacency)
}

/// Kahn's topological sort on (in_degree, adjacency).
fn kahns_topo_sort(
    in_degree: &mut std::collections::HashMap<String, usize>,
    adjacency: &std::collections::HashMap<String, Vec<String>>,
) -> Vec<String> {
    let mut queue: std::collections::BinaryHeap<std::cmp::Reverse<String>> = in_degree
        .iter()
        .filter(|(_, &d)| d == 0)
        .map(|(k, _)| std::cmp::Reverse(k.clone()))
        .collect();
    let mut order = Vec::with_capacity(in_degree.len());
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
    order
}

fn bench_topo_sort(c: &mut Criterion) {
    let mut group = c.benchmark_group("topo_sort");
    for n in [10, 50, 100] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                let (mut in_degree, adjacency) = build_linear_chain(n);
                let order = kahns_topo_sort(&mut in_degree, &adjacency);
                black_box(order);
            });
        });
    }
    group.finish();
}

/// Spec §9: `forjar validate` < 10ms
/// Parse + validate a realistic config with 3 machines and 20 resources.
fn bench_spec9_validate(c: &mut Criterion) {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");

    // Build a realistic 3-machine, 20-resource config
    let mut yaml = String::from(
        r#"version: "1.0"
name: bench-validate
machines:
  web:
    hostname: web.example.com
    addr: 10.0.1.1
  db:
    hostname: db.example.com
    addr: 10.0.1.2
  cache:
    hostname: cache.example.com
    addr: 10.0.1.3
resources:
"#,
    );

    // 20 resources across 3 machines
    for i in 0..8 {
        yaml.push_str(&format!(
            "  web-pkg-{i}:\n    type: package\n    machine: web\n    provider: apt\n    packages: [pkg-{i}]\n"
        ));
    }
    for i in 0..6 {
        yaml.push_str(&format!(
            "  db-file-{i}:\n    type: file\n    machine: db\n    path: /etc/db/conf-{i}.yml\n    content: \"key: value-{i}\"\n"
        ));
    }
    for i in 0..4 {
        yaml.push_str(&format!(
            "  cache-svc-{i}:\n    type: service\n    machine: cache\n    name: svc-{i}\n"
        ));
    }
    for i in 0..2 {
        yaml.push_str(&format!(
            "  web-mount-{i}:\n    type: mount\n    machine: web\n    source: /dev/sda{}\n    path: /mnt/data-{i}\n",
            i + 1
        ));
    }

    std::fs::write(&config_path, &yaml).unwrap();

    c.bench_function("spec9_validate_3m_20r", |b| {
        b.iter(|| {
            let result = parser::parse_and_validate(black_box(&config_path));
            black_box(result.unwrap());
        });
    });
}

/// Build a 3-machine, 20-resource YAML config with dependency chains for plan benchmarks.
fn build_3m_20r_config_with_deps() -> String {
    let mut yaml = String::from(
        "version: \"1.0\"\nname: bench-plan\nmachines:\n  web:\n    hostname: web.example.com\n    addr: 10.0.1.1\n  db:\n    hostname: db.example.com\n    addr: 10.0.1.2\n  cache:\n    hostname: cache.example.com\n    addr: 10.0.1.3\nresources:\n",
    );
    for i in 0..8 {
        yaml.push_str(&format!(
            "  web-pkg-{i}:\n    type: package\n    machine: web\n    provider: apt\n    packages: [pkg-{i}]\n"
        ));
    }
    for i in 0..6 {
        let dep = if i > 0 { format!("\n    depends_on: [db-file-{}]", i - 1) } else { String::new() };
        yaml.push_str(&format!(
            "  db-file-{i}:\n    type: file\n    machine: db\n    path: /etc/db/conf-{i}.yml\n    content: \"key: value-{i}\"{dep}\n"
        ));
    }
    for i in 0..4 {
        yaml.push_str(&format!("  cache-svc-{i}:\n    type: service\n    machine: cache\n    name: svc-{i}\n"));
    }
    for i in 0..2 {
        yaml.push_str(&format!(
            "  web-mount-{i}:\n    type: mount\n    machine: web\n    source: /dev/sda{}\n    path: /mnt/data-{i}\n",
            i + 1
        ));
    }
    yaml
}

/// Spec §9: `forjar plan` (3 machines, 20 resources) < 2s
fn bench_spec9_plan(c: &mut Criterion) {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    std::fs::write(&config_path, build_3m_20r_config_with_deps()).unwrap();

    c.bench_function("spec9_plan_3m_20r", |b| {
        b.iter(|| {
            let config = parser::parse_and_validate(black_box(&config_path)).unwrap();
            let order = resolver::build_execution_order(&config).unwrap();
            let locks = std::collections::HashMap::new();
            let plan = planner::plan(&config, &order, &locks, None);
            black_box(plan);
        });
    });
}

/// Build a 3-machine, 20-resource YAML config and return (yaml, resource_names).
fn build_3m_20r_config() -> (
    String,
    Vec<(String, forjar::core::types::ResourceType)>,
) {
    use forjar::core::types::ResourceType;

    let mut yaml = String::from(
        r#"version: "1.0"
name: bench-apply
machines:
  web:
    hostname: web.example.com
    addr: 10.0.1.1
  db:
    hostname: db.example.com
    addr: 10.0.1.2
  cache:
    hostname: cache.example.com
    addr: 10.0.1.3
resources:
"#,
    );

    let mut resource_names = vec![];
    for i in 0..8 {
        let name = format!("web-pkg-{i}");
        yaml.push_str(&format!(
            "  {name}:\n    type: package\n    machine: web\n    provider: apt\n    packages: [pkg-{i}]\n"
        ));
        resource_names.push((name, ResourceType::Package));
    }
    for i in 0..6 {
        let name = format!("db-file-{i}");
        yaml.push_str(&format!(
            "  {name}:\n    type: file\n    machine: db\n    path: /etc/db/conf-{i}.yml\n    content: \"key: value-{i}\"\n"
        ));
        resource_names.push((name, ResourceType::File));
    }
    for i in 0..4 {
        let name = format!("cache-svc-{i}");
        yaml.push_str(&format!(
            "  {name}:\n    type: service\n    machine: cache\n    name: svc-{i}\n"
        ));
        resource_names.push((name, ResourceType::Service));
    }
    for i in 0..2 {
        let name = format!("web-mount-{i}");
        yaml.push_str(&format!(
            "  {name}:\n    type: mount\n    machine: web\n    source: /dev/sda{}\n    path: /mnt/data-{i}\n",
            i + 1
        ));
        resource_names.push((name, ResourceType::Mount));
    }
    (yaml, resource_names)
}

/// Populate lock files so plan sees "no changes" for all resources.
fn populate_converged_locks(
    state_dir: &std::path::Path,
    resource_names: &[(String, forjar::core::types::ResourceType)],
) {
    use forjar::core::types::{ResourceLock, ResourceStatus, StateLock};

    for (machine_name, hostname) in [
        ("web", "web.example.com"),
        ("db", "db.example.com"),
        ("cache", "cache.example.com"),
    ] {
        let machine_dir = state_dir.join(machine_name);
        std::fs::create_dir_all(&machine_dir).unwrap();

        let mut resources = indexmap::IndexMap::new();
        for (rname, rtype) in resource_names {
            if rname.starts_with(&format!("{machine_name}-")) {
                resources.insert(
                    rname.clone(),
                    ResourceLock {
                        resource_type: rtype.clone(),
                        status: ResourceStatus::Converged,
                        applied_at: None,
                        duration_seconds: None,
                        hash: forjar::tripwire::hasher::hash_string(&format!("state-{rname}")),
                        details: std::collections::HashMap::new(),
                    },
                );
            }
        }

        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: machine_name.to_string(),
            hostname: hostname.to_string(),
            generated_at: "2026-02-26T00:00:00Z".to_string(),
            generator: "forjar-bench".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };
        state::save_lock(&machine_dir, &lock).unwrap();
    }
}

/// Spec §9: `forjar apply` (no changes) < 500ms
/// Full apply pipeline when all resources are already converged (no-op path).
/// Measures: parse → resolve → plan-with-locks → all NoOp.
fn bench_spec9_apply_no_changes(c: &mut Criterion) {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let (yaml, resource_names) = build_3m_20r_config();
    std::fs::write(&config_path, &yaml).unwrap();
    populate_converged_locks(&state_dir, &resource_names);

    c.bench_function("spec9_apply_no_changes_3m_20r", |b| {
        b.iter(|| {
            let config = parser::parse_and_validate(black_box(&config_path)).unwrap();
            let order = resolver::build_execution_order(&config).unwrap();
            let mut locks = std::collections::HashMap::new();
            for machine_name in ["web", "db", "cache"] {
                let machine_dir = state_dir.join(machine_name);
                if let Ok(Some(lock)) = state::load_lock(&machine_dir, machine_name) {
                    locks.insert(machine_name.to_string(), lock);
                }
            }
            let plan = planner::plan(&config, &order, &locks, None);
            black_box(plan);
        });
    });
}

/// Spec §9: `forjar drift` (local, 100 files) < 1s
/// Drift detection against 100 lock entries — hash comparison only.
fn bench_spec9_drift(c: &mut Criterion) {
    use forjar::core::types::{ResourceLock, ResourceStatus, ResourceType, StateLock};

    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path();

    // Create a lock file with 100 resources
    let mut resources = indexmap::IndexMap::new();
    for i in 0..100 {
        let resource_id = format!("file-{i:03}");
        let hash = forjar::tripwire::hasher::hash_string(&format!("content-{i}"));
        resources.insert(
            resource_id,
            ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash,
                details: std::collections::HashMap::new(),
            },
        );
    }

    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "bench-host".to_string(),
        hostname: "bench-host.example.com".to_string(),
        generated_at: "2026-02-26T00:00:00Z".to_string(),
        generator: "forjar-bench".to_string(),
        blake3_version: "1.8".to_string(),
        resources,
    };
    state::save_lock(state_dir, &lock).unwrap();

    // Reload and detect drift (no actual files — tests hash comparison path)
    c.bench_function("spec9_drift_100_resources", |b| {
        b.iter(|| {
            let lock_data = state::load_lock(black_box(state_dir), "bench-host")
                .unwrap()
                .unwrap();
            let findings = drift::detect_drift(&lock_data);
            black_box(findings);
        });
    });
}

/// Spec §9: `forjar validate` scaling — measure parse time vs resource count.
fn bench_spec9_validate_scaling(c: &mut Criterion) {
    let dir = tempfile::tempdir().unwrap();
    let mut group = c.benchmark_group("validate_scaling");

    for n in [5, 20, 50, 100] {
        let config_path = dir.path().join(format!("forjar-{n}.yaml"));
        let mut yaml = String::from(
            "version: \"1.0\"\nname: bench\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n",
        );
        for i in 0..n {
            yaml.push_str(&format!(
                "  r-{i:03}:\n    type: file\n    machine: m\n    path: /tmp/r-{i:03}\n    content: \"data-{i}\"\n"
            ));
        }
        std::fs::write(&config_path, &yaml).unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(n), &config_path, |b, path| {
            b.iter(|| {
                let result = parser::parse_and_validate(black_box(path));
                black_box(result.unwrap());
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
    bench_topo_sort,
    bench_spec9_validate,
    bench_spec9_plan,
    bench_spec9_apply_no_changes,
    bench_spec9_drift,
    bench_spec9_validate_scaling,
);
criterion_main!(benches);
