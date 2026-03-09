//! FJ-1301/1325/1356: Store metadata, secret scanning, GC.
//! Usage: cargo test --test falsification_meta_secret_gc

use forjar::core::store::gc::{collect_roots, mark_and_sweep, GcConfig};
use forjar::core::store::meta::{new_meta, read_meta, write_meta, Provenance, StoreMeta};
use forjar::core::store::secret_scan::{
    is_encrypted, scan_text, scan_yaml_str, ScanResult, SecretFinding,
};
use std::collections::BTreeSet;

// ── FJ-1301: new_meta ──

#[test]
fn new_meta_fields() {
    let m = new_meta(
        "blake3:abc",
        "blake3:recipe",
        &["blake3:in1".into()],
        "x86_64",
        "apt",
    );
    assert_eq!(m.schema, "1.0");
    assert_eq!(m.store_hash, "blake3:abc");
    assert_eq!(m.recipe_hash, "blake3:recipe");
    assert_eq!(m.input_hashes, vec!["blake3:in1"]);
    assert_eq!(m.arch, "x86_64");
    assert_eq!(m.provider, "apt");
    assert!(!m.created_at.is_empty());
    assert!(m.generator.starts_with("forjar"));
    assert!(m.references.is_empty());
    assert!(m.provenance.is_none());
}

// ── FJ-1301: write_meta + read_meta ──

#[test]
fn meta_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("entry");
    let m = new_meta("blake3:abc", "blake3:r", &[], "x86_64", "apt");
    write_meta(&dir, &m).unwrap();
    let read = read_meta(&dir).unwrap();
    assert_eq!(m, read);
}

#[test]
fn meta_with_provenance() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("entry2");
    let mut m = new_meta("blake3:abc", "blake3:r", &[], "x86_64", "cargo");
    m.provenance = Some(Provenance {
        origin_provider: "cargo".into(),
        origin_ref: Some("crates.io/serde".into()),
        origin_hash: Some("abc123".into()),
        derived_from: None,
        derivation_depth: 0,
    });
    m.references = vec!["blake3:ref1".into()];
    write_meta(&dir, &m).unwrap();
    let read = read_meta(&dir).unwrap();
    assert_eq!(m, read);
}

#[test]
fn meta_serde_roundtrip() {
    let m = new_meta(
        "blake3:x",
        "blake3:r",
        &["blake3:i".into()],
        "aarch64",
        "nix",
    );
    let yaml = serde_yaml_ng::to_string(&m).unwrap();
    let parsed: StoreMeta = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(m, parsed);
}

#[test]
fn read_meta_missing() {
    assert!(read_meta(std::path::Path::new("/nonexistent/path")).is_err());
}

// ── FJ-1356: is_encrypted ──

#[test]
fn encrypted_detection() {
    assert!(is_encrypted("ENC[age,abc123]"));
    assert!(is_encrypted("prefix ENC[age,data] suffix"));
    assert!(!is_encrypted("plain text"));
    assert!(!is_encrypted("ENC[other,abc]"));
    assert!(!is_encrypted(""));
}

// ── FJ-1356: scan_text ──

#[test]
fn scan_aws_key() {
    let findings = scan_text("AKIAIOSFODNN7EXAMPLE");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].0, "aws_access_key");
}

#[test]
fn scan_github_token() {
    let token = format!("ghp_{}", "A".repeat(40));
    let findings = scan_text(&token);
    assert!(findings.iter().any(|f| f.0 == "github_token"));
}

#[test]
fn scan_private_key() {
    let findings = scan_text("-----BEGIN RSA PRIVATE KEY-----");
    assert!(findings.iter().any(|f| f.0 == "private_key_pem"));
}

#[test]
fn scan_clean_text() {
    let findings = scan_text("hello world, normal config");
    assert!(findings.is_empty());
}

#[test]
fn scan_encrypted_skipped() {
    let findings = scan_text("ENC[age,AKIAIOSFODNN7EXAMPLE]");
    assert!(findings.is_empty(), "encrypted values should be skipped");
}

#[test]
fn scan_stripe_key() {
    let key = format!("sk_live_{}", "A".repeat(24));
    let findings = scan_text(&key);
    assert!(findings.iter().any(|f| f.0 == "stripe_key"));
}

// ── FJ-1356: scan_yaml_str ──

#[test]
fn scan_yaml_clean() {
    let yaml = "name: nginx\nversion: '1.24'\n";
    let result = scan_yaml_str(yaml);
    assert!(result.clean);
    assert!(result.findings.is_empty());
    assert!(result.scanned_fields >= 2);
}

#[test]
fn scan_yaml_with_secret() {
    let yaml = "api_key: AKIAIOSFODNN7EXAMPLE\n";
    let result = scan_yaml_str(yaml);
    assert!(!result.clean);
    assert!(!result.findings.is_empty());
    assert!(result
        .findings
        .iter()
        .any(|f| f.pattern_name == "aws_access_key"));
}

#[test]
fn scan_yaml_nested() {
    let yaml = "db:\n  password: AKIAIOSFODNN7EXAMPLE\n";
    let result = scan_yaml_str(yaml);
    assert!(!result.clean);
    assert!(result.findings[0].location.contains("db"));
}

#[test]
fn scan_yaml_encrypted_value() {
    let yaml = "secret: ENC[age,encrypted_data_here]\n";
    let result = scan_yaml_str(yaml);
    assert!(result.clean, "encrypted values should not trigger findings");
}

// ── FJ-1325: collect_roots ──

#[test]
fn roots_from_profiles_and_locks() {
    let profiles = vec!["blake3:p1".into(), "blake3:p2".into()];
    let locks = vec!["blake3:l1".into(), "blake3:p1".into()]; // p1 duplicated
    let roots = collect_roots(&profiles, &locks, None);
    assert_eq!(roots.len(), 3); // p1, p2, l1 deduplicated
    assert!(roots.contains("blake3:p1"));
    assert!(roots.contains("blake3:l1"));
}

#[test]
fn roots_empty() {
    let roots = collect_roots(&[], &[], None);
    assert!(roots.is_empty());
}

#[test]
fn roots_with_nonexistent_gc_dir() {
    let roots = collect_roots(
        &["blake3:a".into()],
        &[],
        Some(std::path::Path::new("/nonexistent")),
    );
    assert_eq!(roots.len(), 1);
}

// ── FJ-1325: GcConfig default ──

#[test]
fn gc_config_default() {
    let config = GcConfig::default();
    assert_eq!(config.keep_generations, 5);
    assert!(config.older_than_days.is_none());
}

// ── FJ-1325: mark_and_sweep ──

#[test]
fn gc_mark_and_sweep_live_and_dead() {
    let tmp = tempfile::tempdir().unwrap();
    let store = tmp.path();

    // Create two store entries with meta.yaml
    let live_hash = "a".repeat(64);
    let dead_hash = "b".repeat(64);
    let live_dir = store.join(&live_hash);
    let dead_dir = store.join(&dead_hash);
    std::fs::create_dir_all(&live_dir).unwrap();
    std::fs::create_dir_all(&dead_dir).unwrap();

    // Write meta for both (no references)
    let live_meta = new_meta(
        &format!("blake3:{live_hash}"),
        "blake3:r",
        &[],
        "x86_64",
        "apt",
    );
    let dead_meta = new_meta(
        &format!("blake3:{dead_hash}"),
        "blake3:r",
        &[],
        "x86_64",
        "apt",
    );
    write_meta(&live_dir, &live_meta).unwrap();
    write_meta(&dead_dir, &dead_meta).unwrap();

    // Only live_hash is a root
    let roots: BTreeSet<String> = [format!("blake3:{live_hash}")].into();
    let report = mark_and_sweep(&roots, store).unwrap();
    assert!(report.live.contains(&format!("blake3:{live_hash}")));
    assert!(report.dead.contains(&format!("blake3:{dead_hash}")));
    assert_eq!(report.total, 2);
}

#[test]
fn gc_follows_references() {
    let tmp = tempfile::tempdir().unwrap();
    let store = tmp.path();

    let root_hash = "a".repeat(64);
    let ref_hash = "b".repeat(64);
    let orphan_hash = "c".repeat(64);

    for h in [&root_hash, &ref_hash, &orphan_hash] {
        std::fs::create_dir_all(store.join(h)).unwrap();
    }

    // root references ref_hash
    let mut root_meta = new_meta(
        &format!("blake3:{root_hash}"),
        "blake3:r",
        &[],
        "x86_64",
        "apt",
    );
    root_meta.references = vec![format!("blake3:{ref_hash}")];
    write_meta(&store.join(&root_hash), &root_meta).unwrap();
    write_meta(
        &store.join(&ref_hash),
        &new_meta(
            &format!("blake3:{ref_hash}"),
            "blake3:r",
            &[],
            "x86_64",
            "apt",
        ),
    )
    .unwrap();
    write_meta(
        &store.join(&orphan_hash),
        &new_meta(
            &format!("blake3:{orphan_hash}"),
            "blake3:r",
            &[],
            "x86_64",
            "apt",
        ),
    )
    .unwrap();

    let roots: BTreeSet<String> = [format!("blake3:{root_hash}")].into();
    let report = mark_and_sweep(&roots, store).unwrap();
    assert_eq!(report.live.len(), 2); // root + referenced
    assert_eq!(report.dead.len(), 1); // orphan
    assert!(report.dead.contains(&format!("blake3:{orphan_hash}")));
}

#[test]
fn gc_empty_store() {
    let tmp = tempfile::tempdir().unwrap();
    let roots = BTreeSet::new();
    let report = mark_and_sweep(&roots, tmp.path()).unwrap();
    assert_eq!(report.total, 0);
    assert!(report.live.is_empty());
    assert!(report.dead.is_empty());
}
