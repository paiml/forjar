//! Refs #410: steps 8–9 of the sandbox plan — hash `$out`, move it to
//! `<store>/<hash>/content` — run in-process through `seal_output`.
//!
//! Before, step 8 named `forjar-hash-dir`, a binary forjar never shipped, and
//! step 9 moved `$out` to the literal path `<store>/HASH/content`. These tests
//! pin what replaces them: the store address is the hash of the bytes moved,
//! the entry is read back, and anything the hash would not cover is refused
//! before a byte moves.

use forjar::core::store::sandbox_seal::seal_output;
use forjar::tripwire::hasher::hash_directory;
use std::path::Path;

fn build_out(root: &Path) -> std::path::PathBuf {
    let out = root.join("out");
    std::fs::create_dir_all(out.join("bin")).unwrap();
    std::fs::write(out.join("bin/tool"), "#!/bin/sh\necho tool\n").unwrap();
    std::fs::write(out.join("README"), "built\n").unwrap();
    out
}

#[test]
fn falsify_410_seal_moves_out_to_its_content_address() {
    let dir = tempfile::tempdir().unwrap();
    let out = build_out(dir.path());
    let store = dir.path().join("store");
    let expected = hash_directory(&out).unwrap();

    let sealed = seal_output(&out, &store).expect("a plain tree seals");

    assert_eq!(
        sealed.output_hash, expected,
        "the address is the hash of $out"
    );
    let hex = expected.strip_prefix("blake3:").unwrap();
    assert_eq!(sealed.store_path, store.join(hex).join("content"));
    assert!(!sealed.already_present);
    assert!(!out.exists(), "$out was copied, not moved");
    assert_eq!(
        std::fs::read_to_string(sealed.store_path.join("bin/tool")).unwrap(),
        "#!/bin/sh\necho tool\n"
    );
    assert_eq!(hash_directory(&sealed.store_path).unwrap(), expected);
    assert!(
        !sealed.store_path.to_string_lossy().contains("HASH"),
        "the literal placeholder is back: {}",
        sealed.store_path.display()
    );
}

#[test]
fn falsify_410_seal_refuses_a_symlink_the_hash_would_skip() {
    let dir = tempfile::tempdir().unwrap();
    let out = build_out(dir.path());
    std::os::unix::fs::symlink("/etc/hostname", out.join("bin/link")).unwrap();
    let store = dir.path().join("store");

    let err = seal_output(&out, &store).expect_err("a symlink is not covered by the hash");

    assert!(
        err.contains("bin/link"),
        "the refusal names the entry: {err}"
    );
    assert!(
        out.join("README").exists(),
        "$out moved despite the refusal"
    );
    assert!(!store.exists(), "the store was written despite the refusal");
}

#[test]
fn falsify_410_seal_of_an_existing_entry_leaves_out_alone() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("store");
    let first = seal_output(&build_out(&dir.path().join("a")), &store).unwrap();

    let out = build_out(&dir.path().join("b"));
    let again = seal_output(&out, &store).expect("same bytes, same entry");

    assert_eq!(again.store_path, first.store_path);
    assert!(again.already_present);
    assert!(
        out.exists(),
        "an already-sealed build must not consume $out"
    );
}

#[test]
fn falsify_410_seal_refuses_to_overwrite_an_altered_entry() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("store");
    let first = seal_output(&build_out(&dir.path().join("a")), &store).unwrap();
    std::fs::write(first.store_path.join("README"), "tampered\n").unwrap();

    let out = build_out(&dir.path().join("b"));
    let err = seal_output(&out, &store).expect_err("an entry that no longer matches its address");

    assert!(err.contains("refusing to overwrite"), "{err}");
    assert_eq!(
        std::fs::read_to_string(first.store_path.join("README")).unwrap(),
        "tampered\n",
        "the altered entry was overwritten"
    );
    assert!(out.exists());
}

#[test]
fn falsify_410_seal_refuses_a_missing_out() {
    let dir = tempfile::tempdir().unwrap();
    let err = seal_output(&dir.path().join("out"), &dir.path().join("store"))
        .expect_err("no $out, nothing to seal");
    assert!(err.contains("no output directory"), "{err}");
}
