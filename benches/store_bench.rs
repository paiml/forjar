//! Benchmarks for forjar store operations.
//!
//! Run with: cargo bench --bench store_bench
//!
//! Covers: store path hashing, purity classification, closure hashing,
//! reproducibility scoring, FAR encode/decode, lockfile staleness,
//! sandbox validation, derivation closure, and script purification.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::BTreeMap;
use std::hint::black_box;

// ── Store Path ──────────────────────────────────────────────────────

fn bench_store_path(c: &mut Criterion) {
    use forjar::core::store::path::store_path;

    let recipe_hash = "blake3:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
    let inputs = [
        "blake3:1111111111111111111111111111111111111111111111111111111111111111",
        "blake3:2222222222222222222222222222222222222222222222222222222222222222",
        "blake3:3333333333333333333333333333333333333333333333333333333333333333",
    ];

    c.bench_function("store_path_hash", |b| {
        b.iter(|| {
            let path = store_path(
                black_box(recipe_hash),
                black_box(&inputs),
                black_box("x86_64"),
                black_box("apt"),
            );
            black_box(path);
        });
    });
}

// ── Purity Classification ───────────────────────────────────────────

fn bench_purity_classify(c: &mut Criterion) {
    use forjar::core::store::purity::{classify, PurityLevel, PuritySignals};

    let cases = [
        (
            "pure",
            PuritySignals {
                has_version: true,
                has_store: true,
                has_sandbox: true,
                has_curl_pipe: false,
                dep_levels: vec![],
            },
        ),
        (
            "pinned",
            PuritySignals {
                has_version: true,
                has_store: true,
                has_sandbox: false,
                has_curl_pipe: false,
                dep_levels: vec![],
            },
        ),
        (
            "constrained",
            PuritySignals {
                has_version: false,
                has_store: false,
                has_sandbox: false,
                has_curl_pipe: false,
                dep_levels: vec![],
            },
        ),
        (
            "impure",
            PuritySignals {
                has_version: false,
                has_store: false,
                has_sandbox: false,
                has_curl_pipe: true,
                dep_levels: vec![PurityLevel::Impure],
            },
        ),
    ];

    let mut group = c.benchmark_group("purity_classify");
    for (label, signals) in &cases {
        group.bench_with_input(BenchmarkId::from_parameter(label), signals, |b, sig| {
            b.iter(|| {
                let result = classify(black_box("test-resource"), black_box(sig));
                black_box(result);
            });
        });
    }
    group.finish();
}

// ── Closure Hash ────────────────────────────────────────────────────

fn bench_closure_hash(c: &mut Criterion) {
    use forjar::core::store::closure::closure_hash;

    let mut group = c.benchmark_group("closure_hash");
    for n in [3, 10, 50] {
        let hashes: Vec<String> = (0..n).map(|i| format!("blake3:{i:064x}")).collect();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{n}_nodes")),
            &hashes,
            |b, hashes| {
                b.iter(|| {
                    let hash = closure_hash(black_box(hashes));
                    black_box(hash);
                });
            },
        );
    }
    group.finish();
}

// ── Reproducibility Score ───────────────────────────────────────────

fn bench_repro_score(c: &mut Criterion) {
    use forjar::core::store::purity::PurityLevel;
    use forjar::core::store::repro_score::{compute_score, ReproInput};

    let mut group = c.benchmark_group("repro_score");
    for n in [1, 5, 20] {
        let inputs: Vec<ReproInput> = (0..n)
            .map(|i| ReproInput {
                name: format!("resource-{i}"),
                purity: match i % 4 {
                    0 => PurityLevel::Pure,
                    1 => PurityLevel::Pinned,
                    2 => PurityLevel::Constrained,
                    _ => PurityLevel::Impure,
                },
                has_store: i % 2 == 0,
                has_lock_pin: i % 3 == 0,
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{n}_resources")),
            &inputs,
            |b, inputs| {
                b.iter(|| {
                    let score = compute_score(black_box(inputs));
                    black_box(score);
                });
            },
        );
    }
    group.finish();
}

// ── FAR Encode ──────────────────────────────────────────────────────

fn bench_far_encode(c: &mut Criterion) {
    use forjar::core::store::far::{encode_far, FarFileEntry, FarManifest, FarProvenance};

    let mut group = c.benchmark_group("far_encode");
    for size_kb in [1, 1024, 10240] {
        let data = vec![0xABu8; size_kb * 1024];
        let chunk_hash = blake3::hash(&data);
        let chunks = vec![(*chunk_hash.as_bytes(), data.clone())];

        let manifest = FarManifest {
            name: "bench-package".to_string(),
            version: "1.0.0".to_string(),
            arch: "x86_64".to_string(),
            store_hash: "blake3:abc123".to_string(),
            tree_hash: "blake3:def456".to_string(),
            file_count: 1,
            total_size: (size_kb * 1024) as u64,
            files: vec![FarFileEntry {
                path: "bin/bench".to_string(),
                size: (size_kb * 1024) as u64,
                blake3: chunk_hash.to_hex().to_string(),
            }],
            provenance: FarProvenance {
                origin_provider: "bench".to_string(),
                origin_ref: None,
                origin_hash: None,
                created_at: "2026-03-02T00:00:00Z".to_string(),
                generator: "forjar-bench".to_string(),
            },
            kernel_contracts: None,
        };

        let label = if size_kb >= 1024 {
            format!("{}MB", size_kb / 1024)
        } else {
            format!("{size_kb}KB")
        };
        group.bench_with_input(BenchmarkId::from_parameter(&label), &chunks, |b, chunks| {
            b.iter(|| {
                let mut buf = Vec::with_capacity(size_kb * 1024 + 4096);
                encode_far(black_box(&manifest), black_box(chunks), &mut buf).unwrap();
                black_box(buf);
            });
        });
    }
    group.finish();
}

// ── FAR Decode ──────────────────────────────────────────────────────

fn bench_far_decode(c: &mut Criterion) {
    use forjar::core::store::far::{
        decode_far_manifest, encode_far, FarFileEntry, FarManifest, FarProvenance,
    };

    let data = vec![0xABu8; 64 * 1024];
    let chunk_hash = blake3::hash(&data);
    let chunks = vec![(*chunk_hash.as_bytes(), data)];

    let manifest = FarManifest {
        name: "bench-decode".to_string(),
        version: "1.0.0".to_string(),
        arch: "x86_64".to_string(),
        store_hash: "blake3:abc123".to_string(),
        tree_hash: "blake3:def456".to_string(),
        file_count: 1,
        total_size: 65536,
        files: vec![FarFileEntry {
            path: "bin/bench".to_string(),
            size: 65536,
            blake3: chunk_hash.to_hex().to_string(),
        }],
        provenance: FarProvenance {
            origin_provider: "bench".to_string(),
            origin_ref: None,
            origin_hash: None,
            created_at: "2026-03-02T00:00:00Z".to_string(),
            generator: "forjar-bench".to_string(),
        },
        kernel_contracts: None,
    };

    let mut encoded = Vec::new();
    encode_far(&manifest, &chunks, &mut encoded).unwrap();

    c.bench_function("far_decode_manifest_64KB", |b| {
        b.iter(|| {
            let cursor = std::io::Cursor::new(black_box(&encoded));
            let (m, entries) = decode_far_manifest(cursor).unwrap();
            black_box((m, entries));
        });
    });
}

// ── Lockfile Staleness ──────────────────────────────────────────────

fn bench_lockfile_staleness(c: &mut Criterion) {
    use forjar::core::store::lockfile::{check_staleness, LockFile, Pin};

    let mut group = c.benchmark_group("lockfile_staleness");
    for n in [10, 100, 1000] {
        let mut pins = BTreeMap::new();
        let mut current_hashes = BTreeMap::new();

        for i in 0..n {
            let name = format!("pkg-{i:04}");
            let hash = format!("blake3:{i:064x}");
            pins.insert(
                name.clone(),
                Pin {
                    provider: "apt".to_string(),
                    version: Some(format!("1.0.{i}")),
                    hash: hash.clone(),
                    git_rev: None,
                    pin_type: None,
                },
            );
            // 10% of pins are stale
            let current = if i % 10 == 0 {
                format!("blake3:{:064x}", i + 999_999)
            } else {
                hash
            };
            current_hashes.insert(name, current);
        }

        let lockfile = LockFile {
            schema: "1.0".to_string(),
            pins,
        };

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{n}_pins")),
            &(&lockfile, &current_hashes),
            |b, (lockfile, hashes)| {
                b.iter(|| {
                    let stale = check_staleness(black_box(lockfile), black_box(hashes));
                    black_box(stale);
                });
            },
        );
    }
    group.finish();
}

// ── Sandbox Validate ────────────────────────────────────────────────

fn bench_sandbox_validate(c: &mut Criterion) {
    use forjar::core::store::sandbox::{preset_profile, validate_config};

    let presets = ["full", "network-only", "minimal", "gpu"];

    let mut group = c.benchmark_group("sandbox_validate");
    for name in &presets {
        let config = preset_profile(name).unwrap();
        group.bench_with_input(BenchmarkId::from_parameter(name), &config, |b, config| {
            b.iter(|| {
                let errors = validate_config(black_box(config));
                black_box(errors);
            });
        });
    }
    group.finish();
}

// ── Derivation Closure Hash ─────────────────────────────────────────

fn bench_derivation_closure(c: &mut Criterion) {
    use forjar::core::store::derivation::{derivation_closure_hash, Derivation, DerivationInput};

    let mut inputs = BTreeMap::new();
    let mut input_hashes = BTreeMap::new();
    for i in 0..5 {
        let name = format!("input-{i}");
        let hash = format!("blake3:{i:064x}");
        inputs.insert(
            name.clone(),
            DerivationInput::Store {
                store: hash.clone(),
            },
        );
        input_hashes.insert(name, hash);
    }

    let derivation = Derivation {
        inputs,
        script: "cp -r $inputs/* $out/".to_string(),
        sandbox: None,
        arch: "x86_64".to_string(),
        out_var: "$out".to_string(),
    };

    c.bench_function("derivation_closure_hash_5_inputs", |b| {
        b.iter(|| {
            let hash = derivation_closure_hash(black_box(&derivation), black_box(&input_hashes));
            black_box(hash);
        });
    });
}

// ── Purify Script ───────────────────────────────────────────────────

fn bench_purify_script(c: &mut Criterion) {
    use forjar::core::purifier::purify_script;

    let small = "echo hello\ndate\n";
    let medium = (0..20)
        .map(|i| format!("echo \"step {i}\"\nsleep 1\n"))
        .collect::<String>();
    let large = (0..100)
        .map(|i| {
            format!("if [ -f /tmp/test_{i} ]; then\n  cat /tmp/test_{i}\n  rm /tmp/test_{i}\nfi\n")
        })
        .collect::<String>();

    let mut group = c.benchmark_group("purify_script");
    for (label, script) in [
        ("small", small.to_string()),
        ("medium", medium),
        ("large", large),
    ] {
        group.bench_with_input(BenchmarkId::from_parameter(label), &script, |b, script| {
            b.iter(|| {
                let _ = black_box(purify_script(black_box(script)));
            });
        });
    }
    group.finish();
}

criterion_group!(
    store_benches,
    bench_store_path,
    bench_purity_classify,
    bench_closure_hash,
    bench_repro_score,
    bench_far_encode,
    bench_far_decode,
    bench_lockfile_staleness,
    bench_sandbox_validate,
    bench_derivation_closure,
    bench_purify_script,
);
criterion_main!(store_benches);
