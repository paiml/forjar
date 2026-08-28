//! GH-236: the store records a digest of the bytes it holds, alongside the
//! address derived from the recipe that asked for them.
//!
//! The claim being falsified is narrow and was measurably true on 1.20.1:
//! rewriting `<entry>/content/out.mp4` in place changed NOTHING any API could
//! observe. `read_meta` returned a byte-identical struct and `store_path`
//! returned the same address, because neither ever read a produced byte. There
//! was no `nix store verify` equivalent because there was nothing to compare
//! against.
//!
//! Both addresses are needed and they answer different questions:
//!
//! * INPUT addressing — "has this recipe with these inputs already been built?"
//!   It is what makes staleness detection and cache lookup possible. Replacing
//!   it with content addressing would destroy that, which is why
//!   `input_addressing_is_preserved_not_replaced` exists.
//! * OUTPUT addressing — "are these the bytes we produced?" and "have I already
//!   stored these exact bytes under another name?" Corruption detection and
//!   dedup.
//!
//! Library-only, matching tests/falsification_store_validate_gc.rs: the repo
//! has no assert_cmd, so CLI-adjacent behaviour is exercised in-crate.

use forjar::core::store::content::content_hash;
use forjar::core::store::meta::{new_meta, read_meta, seal_output, write_meta, Addressing};
use forjar::core::store::path::store_path;
use forjar::core::store::verify::{verify_entry, EntryStatus};
use std::path::Path;

/// Create `<root>/<name>/content/out.mp4` holding `bytes`.
fn entry_with_content(root: &Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let entry = root.join(name);
    std::fs::create_dir_all(entry.join("content")).expect("create content dir");
    std::fs::write(entry.join("content/out.mp4"), bytes).expect("write artifact");
    entry
}

/// THE HEADLINE. Corruption under an unchanged recipe must be visible.
///
/// RED on 1.20.1 by failing to compile: `forjar::core::store::verify`,
/// `StoreMeta::output_hash` and `seal_output` did not exist. It stays red under
/// a half-fix that adds the field but never populates it, because `seal_output`
/// is what makes `output_hash` `Some` — an entry that was never sealed reports
/// `Unsealed`, not `Mismatch`.
#[test]
fn corrupting_an_artifact_under_an_unchanged_recipe_is_detected() {
    let dir = tempfile::tempdir().expect("tempdir");
    let entry = entry_with_content(dir.path(), &"a".repeat(64), b"GOOD ARTIFACT BYTES");

    let mut meta = new_meta("blake3:a", "blake3:recipe", &[], "x86_64", "ffmpeg");
    seal_output(&entry, &mut meta, Addressing::Content).expect("seal");
    write_meta(&entry, &meta).expect("write meta");

    // Bit rot / a partial write / a manual edit. The recipe, the inputs and
    // meta.yaml are all untouched.
    std::fs::write(entry.join("content/out.mp4"), b"CORRUPTED BYTES!!!!").expect("corrupt");

    let verdict = verify_entry(&entry);
    assert!(
        matches!(verdict.status, EntryStatus::Mismatch { .. }),
        "CORRUPTION INVISIBLE: the store cannot tell that the bytes it holds \
         are not the bytes it produced — got {:?}",
        verdict.status
    );
    assert!(verdict.is_failure(), "a corrupt entry must fail the gate");
}

/// The index key dedup would be built on: identical bytes, one address.
///
/// Two entries under different recipe hashes with byte-identical `content/`
/// trees must hash to the same output digest. Without this there is nothing to
/// key an `output_hash -> store_hash` table by, which is why the issue's own
/// 149.9 GiB mp4 store pays twice for a recipe edit that changed no output.
#[test]
fn byte_identical_outputs_share_one_content_address() {
    let dir = tempfile::tempdir().expect("tempdir");
    let a = entry_with_content(dir.path(), &"a".repeat(64), b"IDENTICAL RENDERED BYTES");
    let b = entry_with_content(dir.path(), &"b".repeat(64), b"IDENTICAL RENDERED BYTES");

    let ha = content_hash(&a.join("content")).expect("hash a");
    let hb = content_hash(&b.join("content")).expect("hash b");
    assert_eq!(ha, hb, "identical bytes must have one content address");

    let c = entry_with_content(dir.path(), &"c".repeat(64), b"DIFFERENT RENDERED BYTES");
    let hc = content_hash(&c.join("content")).expect("hash c");
    assert_ne!(ha, hc, "different bytes must not collide");
}

/// The guard against "fixing" #236 by making the store content-addressed.
///
/// Green before AND after, deliberately. It exists to go red if someone
/// replaces input addressing rather than adding output addressing alongside it:
/// `store_path` is what answers "has this recipe with these inputs already been
/// built?", and a store that cannot answer that has no staleness detection and
/// no cache lookup.
#[test]
fn input_addressing_is_preserved_not_replaced() {
    let a = store_path("blake3:recipeA", &[], "x86_64", "ffmpeg");
    let b = store_path("blake3:recipeB", &[], "x86_64", "ffmpeg");
    assert_ne!(
        a, b,
        "two different recipes must still occupy two different addresses"
    );
    assert_eq!(
        a,
        store_path("blake3:recipeA", &[], "x86_64", "ffmpeg"),
        "the derivation address must stay deterministic"
    );
}

/// THE MIGRATION GUARD. A schema-1.0 meta.yaml already on disk must still load.
///
/// Goes red if `output_hash` is added without `#[serde(default)]`, which would
/// make `read_meta` reject every entry in every existing store — an instant,
/// total regression. It also pins that such an entry reports `Unsealed` rather
/// than a false `Mismatch`: there is no recorded digest to be wrong about, and
/// calling that corruption would make `--repair` delete good data.
#[test]
fn a_schema_1_0_entry_still_loads_and_reports_unsealed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let entry = entry_with_content(dir.path(), &"d".repeat(64), b"LEGACY ARTIFACT");

    // Written by hand, exactly as forjar 1.20.1 serialized it.
    let legacy = "\
schema: '1.0'
store_hash: blake3:store
recipe_hash: blake3:recipe
input_hashes:
- blake3:in1
arch: x86_64
provider: ffmpeg
created_at: 2026-08-16T09:03:53Z
generator: forjar 1.13.1
references: []
";
    std::fs::write(entry.join("meta.yaml"), legacy).expect("write legacy meta");

    let meta = read_meta(&entry).expect("a schema-1.0 meta.yaml must still load");
    assert_eq!(meta.schema, "1.0");
    assert_eq!(meta.output_hash, None);
    assert_eq!(meta.addressing, Addressing::Derivation);

    let verdict = verify_entry(&entry);
    assert_eq!(
        verdict.status,
        EntryStatus::Unsealed,
        "a pre-1.1 entry has no recorded digest; reporting a mismatch would be a fabrication"
    );
    assert!(
        !verdict.is_failure(),
        "an unsealed entry must not fail the gate, or every legacy store turns a CI job red"
    );
}
