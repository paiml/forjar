//! Benchmarks for copia delta sync operations.
//!
//! Run with: cargo bench --bench copia_bench
//!
//! FJ-247: Copia delta sync — signature computation, delta generation,
//! patch script serialization, and signature parsing.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;

/// Benchmark copia signature computation at various file sizes.
/// Signatures are per-block BLAKE3 hashes (4KB blocks).
fn bench_copia_signatures(c: &mut Criterion) {
    use forjar::copia;

    let mut group = c.benchmark_group("copia_signatures");
    for size_mb in [1, 4, 16] {
        let data = vec![0xABu8; size_mb * 1024 * 1024];
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{size_mb}MB")),
            &data,
            |b, data| {
                b.iter(|| {
                    let sigs = copia::compute_signatures(black_box(data));
                    black_box(sigs);
                });
            },
        );
    }
    group.finish();
}

/// Benchmark copia delta computation for files with varying change percentages.
/// Simulates model fine-tuning scenarios: 2% change (typical), 50% change, 100% change.
fn bench_copia_delta(c: &mut Criterion) {
    use forjar::copia;

    let size = 4 * 1024 * 1024; // 4MB test file
    let mut old_data = vec![0u8; size];
    // Make blocks unique
    for i in 0..(size / copia::BLOCK_SIZE) {
        old_data[i * copia::BLOCK_SIZE] = (i % 256) as u8;
    }
    let remote_sigs = copia::compute_signatures(&old_data);

    let mut group = c.benchmark_group("copia_delta");
    for change_pct in [2, 10, 50, 100] {
        let mut new_data = old_data.clone();
        let blocks = size / copia::BLOCK_SIZE;
        let changed = (blocks * change_pct) / 100;
        for i in 0..changed {
            new_data[i * copia::BLOCK_SIZE] = 0xFF;
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{change_pct}pct")),
            &new_data,
            |b, new_data| {
                b.iter(|| {
                    let delta = copia::compute_delta(black_box(new_data), &remote_sigs);
                    black_box(delta);
                });
            },
        );
    }
    group.finish();
}

/// Benchmark copia patch script generation (measures serialization overhead).
fn bench_copia_patch_script(c: &mut Criterion) {
    use forjar::copia;

    let size = 1024 * 1024; // 1MB
    let old_data = vec![0u8; size];
    let remote_sigs = copia::compute_signatures(&old_data);

    // 10% change
    let mut new_data = old_data;
    let blocks = size / copia::BLOCK_SIZE;
    let changed = blocks / 10;
    for i in 0..changed {
        new_data[i * copia::BLOCK_SIZE] = 0xFF;
    }
    let delta = copia::compute_delta(&new_data, &remote_sigs);

    c.bench_function("copia_patch_script_1MB_10pct", |b| {
        b.iter(|| {
            let script = copia::patch_script(
                black_box("/opt/models/test.gguf"),
                black_box(&delta),
                Some("noah"),
                None,
                Some("0644"),
            );
            black_box(script);
        });
    });
}

/// Benchmark copia signature parsing (measures remote output deserialization).
fn bench_copia_parse_signatures(c: &mut Criterion) {
    use forjar::copia;

    // Generate a realistic signature output for 1024 blocks (4MB file)
    let mut output = String::from("SIZE:4194304\n");
    for i in 0..1024 {
        let hash = blake3::hash(&[i as u8; copia::BLOCK_SIZE]).to_hex();
        output.push_str(&format!("{i} {hash}\n"));
    }

    c.bench_function("copia_parse_signatures_1024_blocks", |b| {
        b.iter(|| {
            let sigs = copia::parse_signatures(black_box(&output)).unwrap();
            black_box(sigs);
        });
    });
}

criterion_group!(
    copia_benches,
    bench_copia_signatures,
    bench_copia_delta,
    bench_copia_patch_script,
    bench_copia_parse_signatures,
);
criterion_main!(copia_benches);
