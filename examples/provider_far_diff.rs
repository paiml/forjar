//! FJ-1333–1336/1346/1345: Provider import, FAR archives, store diff/sync.
//!
//! Usage: cargo run --example provider_far_diff

use forjar::core::store::far::{
    decode_far_manifest, encode_far, FarFileEntry, FarManifest, FarProvenance, FAR_MAGIC,
};
use forjar::core::store::meta::{new_meta, Provenance};
use forjar::core::store::provider::{
    all_providers, capture_method, import_command, origin_ref_string, validate_import,
    ImportConfig, ImportProvider,
};
use forjar::core::store::store_diff::{
    build_sync_plan, compute_diff, has_diffable_provenance, upstream_check_command,
};
use std::collections::BTreeMap;

fn main() {
    println!("Forjar: Provider Import, FAR Archives & Store Diff");
    println!("{}", "=".repeat(55));

    // ── FJ-1333–1336: Provider Import ──
    println!("\n[FJ-1333] Provider Import Commands:");
    for provider in all_providers() {
        let cfg = ImportConfig {
            provider,
            reference: "example-pkg".into(),
            version: Some("1.0".into()),
            arch: "x86_64".into(),
            options: BTreeMap::new(),
        };
        println!("  {:?}:", provider);
        println!("    cmd: {}", import_command(&cfg));
        println!("    ref: {}", origin_ref_string(&cfg));
        println!("    capture: {}", capture_method(provider));
    }

    let valid_cfg = ImportConfig {
        provider: ImportProvider::Apt,
        reference: "nginx".into(),
        version: Some("1.24".into()),
        arch: "x86_64".into(),
        options: BTreeMap::new(),
    };
    println!("\n  Validation (good): {:?}", validate_import(&valid_cfg));

    // ── FJ-1346: FAR Archive ──
    println!("\n[FJ-1346] FAR Archive (magic={} bytes):", FAR_MAGIC.len());
    let manifest = FarManifest {
        name: "demo".into(),
        version: "1.0.0".into(),
        arch: "x86_64".into(),
        store_hash: "blake3:abc".into(),
        tree_hash: "blake3:def".into(),
        file_count: 1,
        total_size: 12,
        files: vec![FarFileEntry {
            path: "data.bin".into(),
            size: 12,
            blake3: "blake3:file1".into(),
        }],
        provenance: FarProvenance {
            origin_provider: "apt".into(),
            origin_ref: Some("demo-pkg".into()),
            origin_hash: None,
            created_at: "2026-01-01T00:00:00Z".into(),
            generator: "forjar 1.0.0".into(),
        },
        kernel_contracts: None,
    };
    let chunk_data = b"hello forjar".to_vec();
    let chunk_hash = blake3::hash(&chunk_data);
    let mut buf = Vec::new();
    encode_far(&manifest, &[(*chunk_hash.as_bytes(), chunk_data)], &mut buf).unwrap();
    println!("  Encoded: {} bytes", buf.len());

    let (decoded, table) = decode_far_manifest(std::io::Cursor::new(&buf)).unwrap();
    println!("  Decoded: name={}, chunks={}", decoded.name, table.len());
    println!("  Match: {}", decoded == manifest);

    // ── FJ-1345: Store Diff ──
    println!("\n[FJ-1345] Store Diff:");
    let mut meta = new_meta("blake3:store1", "blake3:r", &[], "x86_64", "apt");
    meta.provenance = Some(Provenance {
        origin_provider: "apt".into(),
        origin_ref: Some("nginx".into()),
        origin_hash: Some("blake3:old".into()),
        derived_from: None,
        derivation_depth: 0,
    });
    let diff = compute_diff(&meta, Some("blake3:new"));
    println!("  Upstream changed: {}", diff.upstream_changed);
    println!("  Provider: {}", diff.provider);
    println!("  Diffable: {}", has_diffable_provenance(&meta));
    if let Some(cmd) = upstream_check_command(&meta) {
        println!("  Check cmd: {cmd}");
    }

    let plan = build_sync_plan(&[(meta, Some("blake3:new".into()))]);
    println!(
        "  Sync plan: {} re-imports, {} replays",
        plan.re_imports.len(),
        plan.derivation_replays.len()
    );

    println!("\n{}", "=".repeat(55));
    println!("All provider/FAR/diff criteria survived.");
}
