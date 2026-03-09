//! Memory profiling benchmarks using dhat-rs.
//!
//! Run with: cargo test --bench memory_bench --features dhat-heap
//!
//! Each test measures heap allocations (total bytes, total blocks, peak bytes)
//! for key forjar operations. Regression thresholds enforce the spec §9
//! target of < 50MB peak heap.

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use forjar::core::{codegen, parser, planner, resolver, state, types::*};
use forjar::tripwire::{drift, hasher};

/// Helper: build a synthetic N-resource config YAML and write it to a temp path.
fn write_bench_config(dir: &std::path::Path, n_resources: usize) -> std::path::PathBuf {
    let mut yaml = String::from(
        "version: \"1.0\"\nname: mem-bench\nmachines:\n  web:\n    hostname: web\n    addr: 10.0.1.1\n  db:\n    hostname: db\n    addr: 10.0.1.2\n  cache:\n    hostname: cache\n    addr: 10.0.1.3\nresources:\n",
    );
    for i in 0..n_resources {
        match i % 4 {
            0 => yaml.push_str(&format!(
                "  pkg-{i}:\n    type: package\n    machine: web\n    provider: apt\n    packages: [pkg-{i}]\n"
            )),
            1 => yaml.push_str(&format!(
                "  file-{i}:\n    type: file\n    machine: db\n    path: /etc/conf-{i}.yml\n    content: \"val-{i}\"\n"
            )),
            2 => yaml.push_str(&format!(
                "  svc-{i}:\n    type: service\n    machine: cache\n    name: svc-{i}\n"
            )),
            _ => yaml.push_str(&format!(
                "  mount-{i}:\n    type: mount\n    machine: web\n    source: /dev/sda1\n    path: /mnt/data-{i}\n"
            )),
        }
    }
    let path = dir.join("forjar.yaml");
    std::fs::write(&path, &yaml).unwrap();
    path
}

/// Helper: build a synthetic lock with N converged resources.
fn write_bench_lock(state_dir: &std::path::Path, n: usize) {
    let mut resources = indexmap::IndexMap::new();
    for i in 0..n {
        resources.insert(
            format!("file-{i:03}"),
            ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: hasher::hash_string(&format!("content-{i}")),
                details: std::collections::HashMap::new(),
            },
        );
    }
    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "bench-host".to_string(),
        hostname: "bench-host".to_string(),
        generated_at: "2026-02-26T00:00:00Z".to_string(),
        generator: "forjar-bench".to_string(),
        blake3_version: "1.8".to_string(),
        resources,
    };
    state::save_lock(state_dir, &lock).unwrap();
}

// ── Allocation tests ──────────────────────────────────────────────

#[test]
fn mem_validate_20r() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = write_bench_config(dir.path(), 20);

    let profiler = dhat::Profiler::builder().testing().build();
    let _config = parser::parse_and_validate(&config_path).unwrap();
    let stats = dhat::HeapStats::get();
    drop(profiler);

    eprintln!(
        "validate(20r): {} total bytes, {} total blocks, {} peak bytes",
        stats.total_bytes, stats.total_blocks, stats.max_bytes
    );

    // Spec §9: < 50MB peak for any single operation
    assert!(
        stats.max_bytes < 50 * 1024 * 1024,
        "validate peak heap {:.2}MB exceeds 50MB",
        stats.max_bytes as f64 / (1024.0 * 1024.0)
    );
}

#[test]
fn mem_validate_100r() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = write_bench_config(dir.path(), 100);

    let profiler = dhat::Profiler::builder().testing().build();
    let _config = parser::parse_and_validate(&config_path).unwrap();
    let stats = dhat::HeapStats::get();
    drop(profiler);

    eprintln!(
        "validate(100r): {} total bytes, {} total blocks, {} peak bytes",
        stats.total_bytes, stats.total_blocks, stats.max_bytes
    );
    assert!(
        stats.max_bytes < 50 * 1024 * 1024,
        "validate(100r) peak heap {:.2}MB exceeds 50MB",
        stats.max_bytes as f64 / (1024.0 * 1024.0)
    );
}

#[test]
fn mem_plan_20r() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = write_bench_config(dir.path(), 20);
    let config = parser::parse_and_validate(&config_path).unwrap();

    let profiler = dhat::Profiler::builder().testing().build();
    let order = resolver::build_execution_order(&config).unwrap();
    let locks = std::collections::HashMap::new();
    let _plan = planner::plan(&config, &order, &locks, None);
    let stats = dhat::HeapStats::get();
    drop(profiler);

    eprintln!(
        "plan(20r): {} total bytes, {} total blocks, {} peak bytes",
        stats.total_bytes, stats.total_blocks, stats.max_bytes
    );
    assert!(
        stats.max_bytes < 50 * 1024 * 1024,
        "plan peak heap {:.2}MB exceeds 50MB",
        stats.max_bytes as f64 / (1024.0 * 1024.0)
    );
}

#[test]
fn mem_codegen_20r() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = write_bench_config(dir.path(), 20);
    let config = parser::parse_and_validate(&config_path).unwrap();

    let profiler = dhat::Profiler::builder().testing().build();
    for (_id, resource) in &config.resources {
        let _ = codegen::apply_script(resource);
        let _ = codegen::check_script(resource);
    }
    let stats = dhat::HeapStats::get();
    drop(profiler);

    eprintln!(
        "codegen(20r): {} total bytes, {} total blocks, {} peak bytes",
        stats.total_bytes, stats.total_blocks, stats.max_bytes
    );
    assert!(
        stats.max_bytes < 50 * 1024 * 1024,
        "codegen peak heap {:.2}MB exceeds 50MB",
        stats.max_bytes as f64 / (1024.0 * 1024.0)
    );
}

#[test]
fn mem_drift_100r() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path();
    write_bench_lock(state_dir, 100);
    let lock_data = state::load_lock(state_dir, "bench-host").unwrap().unwrap();

    let profiler = dhat::Profiler::builder().testing().build();
    let _findings = drift::detect_drift(&lock_data);
    let stats = dhat::HeapStats::get();
    drop(profiler);

    eprintln!(
        "drift(100r): {} total bytes, {} total blocks, {} peak bytes",
        stats.total_bytes, stats.total_blocks, stats.max_bytes
    );
    assert!(
        stats.max_bytes < 50 * 1024 * 1024,
        "drift peak heap {:.2}MB exceeds 50MB",
        stats.max_bytes as f64 / (1024.0 * 1024.0)
    );
}

#[test]
fn mem_blake3_1mb() {
    let data = vec![0xABu8; 1_048_576];

    let profiler = dhat::Profiler::builder().testing().build();
    let _hash = blake3::hash(&data);
    let stats = dhat::HeapStats::get();
    drop(profiler);

    eprintln!(
        "blake3(1MB): {} total bytes, {} total blocks, {} peak bytes",
        stats.total_bytes, stats.total_blocks, stats.max_bytes
    );
    // BLAKE3 should allocate nearly nothing — hashing is stack-based
    assert!(
        stats.total_bytes < 4096,
        "blake3 allocated {} bytes — expected near-zero",
        stats.total_bytes
    );
}

#[test]
fn mem_full_pipeline_100r() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = write_bench_config(dir.path(), 100);

    let profiler = dhat::Profiler::builder().testing().build();

    let config = parser::parse_and_validate(&config_path).unwrap();
    let order = resolver::build_execution_order(&config).unwrap();
    let locks = std::collections::HashMap::new();
    let _plan = planner::plan(&config, &order, &locks, None);
    for (_id, resource) in &config.resources {
        let _ = codegen::apply_script(resource);
    }

    let stats = dhat::HeapStats::get();
    drop(profiler);

    eprintln!(
        "full_pipeline(100r): {} total bytes, {} total blocks, {} peak bytes ({:.2}MB peak)",
        stats.total_bytes,
        stats.total_blocks,
        stats.max_bytes,
        stats.max_bytes as f64 / (1024.0 * 1024.0)
    );
    assert!(
        stats.max_bytes < 50 * 1024 * 1024,
        "full pipeline peak heap {:.2}MB exceeds 50MB spec target",
        stats.max_bytes as f64 / (1024.0 * 1024.0)
    );
}

#[test]
fn mem_store_lock_save_load_100r() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path();

    // Pre-build the lock data outside profiling
    let mut resources = indexmap::IndexMap::new();
    for i in 0..100 {
        resources.insert(
            format!("file-{i:03}"),
            ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: hasher::hash_string(&format!("content-{i}")),
                details: std::collections::HashMap::new(),
            },
        );
    }
    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "bench-host".to_string(),
        hostname: "bench-host".to_string(),
        generated_at: "2026-02-26T00:00:00Z".to_string(),
        generator: "forjar-bench".to_string(),
        blake3_version: "1.8".to_string(),
        resources,
    };

    let profiler = dhat::Profiler::builder().testing().build();
    state::save_lock(state_dir, &lock).unwrap();
    let _loaded = state::load_lock(state_dir, "bench-host").unwrap().unwrap();
    let stats = dhat::HeapStats::get();
    drop(profiler);

    eprintln!(
        "lock_save_load(100r): {} total bytes, {} total blocks, {} peak bytes",
        stats.total_bytes, stats.total_blocks, stats.max_bytes
    );
    assert!(
        stats.max_bytes < 10 * 1024 * 1024,
        "lock save/load peak heap {:.2}MB exceeds 10MB",
        stats.max_bytes as f64 / (1024.0 * 1024.0)
    );
}

#[test]
fn mem_copia_delta_4mb() {
    use forjar::copia;

    let size = 4 * 1024 * 1024;
    let mut old_data = vec![0u8; size];
    for i in 0..(size / copia::BLOCK_SIZE) {
        old_data[i * copia::BLOCK_SIZE] = (i % 256) as u8;
    }
    let remote_sigs = copia::compute_signatures(&old_data);

    // 10% change
    let mut new_data = old_data;
    let blocks = size / copia::BLOCK_SIZE;
    let changed = blocks / 10;
    for i in 0..changed {
        new_data[i * copia::BLOCK_SIZE] = 0xFF;
    }

    let profiler = dhat::Profiler::builder().testing().build();
    let _delta = copia::compute_delta(&new_data, &remote_sigs);
    let stats = dhat::HeapStats::get();
    drop(profiler);

    eprintln!(
        "copia_delta(4MB, 10%): {} total bytes, {} total blocks, {} peak bytes ({:.2}MB peak)",
        stats.total_bytes,
        stats.total_blocks,
        stats.max_bytes,
        stats.max_bytes as f64 / (1024.0 * 1024.0)
    );
    assert!(
        stats.max_bytes < 50 * 1024 * 1024,
        "copia delta peak heap {:.2}MB exceeds 50MB",
        stats.max_bytes as f64 / (1024.0 * 1024.0)
    );
}
