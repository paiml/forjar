//! Benchmarks for the copia rolling delta engine.
//!
//! Run with: cargo bench --bench copia_bench
//!
//! FJ-242 (rolling): weak-checksum throughput, rolling delta generation, patch
//! script serialization, and signature parsing.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use forjar::copia;
use std::hint::black_box;

fn signature_of(data: &[u8]) -> copia::Signature {
    let mut out = format!("SIZE:{}\n", data.len());
    for (i, chunk) in data.chunks(copia::BLOCK_SIZE).enumerate() {
        let weak = copia::weak_checksum(chunk);
        let strong = blake3::hash(chunk).to_hex();
        out.push_str(&format!("{i} {weak} {strong}\n"));
    }
    copia::parse_signature(&out).unwrap().unwrap()
}

/// Weak (Adler) checksum throughput — the per-block cost the receiver's awk mirrors.
fn bench_copia_weak_checksum(c: &mut Criterion) {
    let mut group = c.benchmark_group("copia_weak_checksum");
    for size_mb in [1, 4, 16] {
        let data = vec![0xABu8; size_mb * 1024 * 1024];
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{size_mb}MB")),
            &data,
            |b, data| {
                b.iter(|| {
                    let mut acc = 0u32;
                    for chunk in data.chunks(copia::BLOCK_SIZE) {
                        acc = acc.wrapping_add(copia::weak_checksum(black_box(chunk)));
                    }
                    black_box(acc);
                });
            },
        );
    }
    group.finish();
}

/// Rolling delta generation for files with varying change percentages.
fn bench_copia_rolling_delta(c: &mut Criterion) {
    let size = 4 * 1024 * 1024;
    let mut old_data = vec![0u8; size];
    for i in 0..(size / copia::BLOCK_SIZE) {
        old_data[i * copia::BLOCK_SIZE] = (i % 256) as u8;
    }
    let sig = signature_of(&old_data);

    let mut group = c.benchmark_group("copia_rolling_delta");
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
                    let delta = copia::rolling_delta(black_box(new_data), &sig);
                    black_box(delta);
                });
            },
        );
    }
    group.finish();
}

/// Patch script generation (serialization overhead).
fn bench_copia_patch_script(c: &mut Criterion) {
    let size = 1024 * 1024;
    let mut old_data = vec![0u8; size];
    for i in 0..(size / copia::BLOCK_SIZE) {
        old_data[i * copia::BLOCK_SIZE] = (i % 256) as u8;
    }
    let sig = signature_of(&old_data);
    let mut new_data = old_data;
    let changed = (size / copia::BLOCK_SIZE) / 10;
    for i in 0..changed {
        new_data[i * copia::BLOCK_SIZE] = 0xFF;
    }
    let delta = copia::rolling_delta(&new_data, &sig);

    c.bench_function("copia_patch_script_1MB_10pct", |b| {
        b.iter(|| {
            let script = copia::patch_script(
                black_box("/opt/models/test.gguf"),
                black_box(&delta),
                black_box("b3verifyhash"),
                Some("noah"),
                None,
                Some("0644"),
            );
            black_box(script);
        });
    });
}

/// Signature parsing (remote output → copia::Signature).
fn bench_copia_parse_signature(c: &mut Criterion) {
    let mut output = String::from("SIZE:4194304\n");
    for i in 0..1024u32 {
        let hash = blake3::hash(&[i as u8; copia::BLOCK_SIZE]).to_hex();
        output.push_str(&format!("{i} {} {hash}\n", i.wrapping_mul(2_654_435_761)));
    }
    c.bench_function("copia_parse_signature_1024_blocks", |b| {
        b.iter(|| {
            let sig = copia::parse_signature(black_box(&output)).unwrap();
            black_box(sig);
        });
    });
}

criterion_group!(
    copia_benches,
    bench_copia_weak_checksum,
    bench_copia_rolling_delta,
    bench_copia_patch_script,
    bench_copia_parse_signature,
);
criterion_main!(copia_benches);
