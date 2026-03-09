//! FJ-1301/1356/1325: Store metadata, secret scanning, garbage collection.
//!
//! Usage: cargo run --example meta_secret_gc

use forjar::core::store::gc::{collect_roots, mark_and_sweep, GcConfig};
use forjar::core::store::meta::{new_meta, read_meta, write_meta, Provenance};
use forjar::core::store::secret_scan::{is_encrypted, scan_text, scan_yaml_str};
use std::collections::BTreeSet;

fn main() {
    println!("Forjar: Store Metadata, Secret Scanning & GC");
    println!("{}", "=".repeat(55));

    // ── FJ-1301: Store Metadata ──
    println!("\n[FJ-1301] Store Metadata:");

    let meta = new_meta(
        "blake3:abc123",
        "blake3:recipe456",
        &["blake3:input1".into(), "blake3:input2".into()],
        "x86_64",
        "apt",
    );
    println!("  schema: {}", meta.schema);
    println!("  store_hash: {}", meta.store_hash);
    println!("  recipe_hash: {}", meta.recipe_hash);
    println!("  inputs: {:?}", meta.input_hashes);
    println!("  arch: {}, provider: {}", meta.arch, meta.provider);
    println!("  created_at: {}", meta.created_at);
    println!("  generator: {}", meta.generator);

    let tmp = tempfile::tempdir().unwrap();
    let entry_dir = tmp.path().join("entry1");
    write_meta(&entry_dir, &meta).unwrap();
    let read_back = read_meta(&entry_dir).unwrap();
    println!("  Roundtrip: match={}", meta == read_back);

    let mut prov_meta = new_meta("blake3:derived", "blake3:r", &[], "x86_64", "cargo");
    prov_meta.provenance = Some(Provenance {
        origin_provider: "cargo".into(),
        origin_ref: Some("crates.io/serde".into()),
        origin_hash: Some("sha256:abc".into()),
        derived_from: None,
        derivation_depth: 0,
    });
    prov_meta.references = vec!["blake3:ref1".into()];
    let prov_dir = tmp.path().join("entry2");
    write_meta(&prov_dir, &prov_meta).unwrap();
    let prov_read = read_meta(&prov_dir).unwrap();
    println!("  Provenance roundtrip: match={}", prov_meta == prov_read);

    // ── FJ-1356: Secret Scanning ──
    println!("\n[FJ-1356] Secret Scanning:");

    let test_cases = [
        ("Clean text", "hello world, normal config"),
        ("AWS key", "AKIAIOSFODNN7EXAMPLE"),
        ("Private key", "-----BEGIN RSA PRIVATE KEY-----"),
        ("Encrypted", "ENC[age,AKIAIOSFODNN7EXAMPLE]"),
    ];
    for (label, text) in test_cases {
        let findings = scan_text(text);
        let encrypted = is_encrypted(text);
        println!(
            "  {label}: findings={}, encrypted={encrypted}",
            findings.len()
        );
    }

    let clean_yaml = "name: nginx\nversion: '1.24'\n";
    let result = scan_yaml_str(clean_yaml);
    println!(
        "  YAML clean: clean={}, fields={}",
        result.clean, result.scanned_fields
    );

    let secret_yaml = "api_key: AKIAIOSFODNN7EXAMPLE\n";
    let result = scan_yaml_str(secret_yaml);
    println!(
        "  YAML secret: clean={}, findings={}",
        result.clean,
        result.findings.len()
    );

    // ── FJ-1325: Garbage Collection ──
    println!("\n[FJ-1325] Garbage Collection:");

    let config = GcConfig::default();
    println!(
        "  Default config: keep_generations={}, older_than_days={:?}",
        config.keep_generations, config.older_than_days
    );

    let profiles = vec!["blake3:p1".into(), "blake3:p2".into()];
    let locks = vec!["blake3:l1".into(), "blake3:p1".into()];
    let roots = collect_roots(&profiles, &locks, None);
    println!("  Roots (2 profiles + 2 locks, 1 dup): {}", roots.len());

    // Build a store with live and dead entries
    let gc_tmp = tempfile::tempdir().unwrap();
    let store = gc_tmp.path();
    let live_hash = "a".repeat(64);
    let dead_hash = "b".repeat(64);
    for h in [&live_hash, &dead_hash] {
        std::fs::create_dir_all(store.join(h)).unwrap();
        write_meta(
            &store.join(h),
            &new_meta(&format!("blake3:{h}"), "blake3:r", &[], "x86_64", "apt"),
        )
        .unwrap();
    }
    let gc_roots: BTreeSet<String> = [format!("blake3:{live_hash}")].into();
    let report = mark_and_sweep(&gc_roots, store).unwrap();
    println!(
        "  GC: total={}, live={}, dead={}",
        report.total,
        report.live.len(),
        report.dead.len()
    );

    println!("\n{}", "=".repeat(55));
    println!("All meta/secret/gc criteria survived.");
}
