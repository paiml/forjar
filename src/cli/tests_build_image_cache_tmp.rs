//! #768 review fixes: the sandbox build path must not leave a stale
//! `build-cache.hash` behind, and the base-export work dir is unique per call.

use super::build_image_base::{
    check_build_cache, invalidate_build_cache, unique_base_work, write_build_cache,
};

fn cached_dir(hash: &str) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    write_build_cache(d.path(), hash);
    std::fs::write(d.path().join("build-metrics.json"), "{}").unwrap();
    d
}

#[test]
fn a_written_hash_hits_until_invalidated() {
    let d = cached_dir("h1");
    assert!(check_build_cache(d.path(), "h1").is_some());
    invalidate_build_cache(d.path());
    assert!(
        check_build_cache(d.path(), "h1").is_none(),
        "a sandbox build overwrote the layout; the old hash must not hit"
    );
}

#[test]
fn invalidating_a_dir_with_no_cache_is_a_no_op() {
    let d = tempfile::tempdir().unwrap();
    invalidate_build_cache(d.path());
    invalidate_build_cache(&d.path().join("missing"));
}

#[test]
fn files_then_packages_then_files_does_not_serve_the_stale_image() {
    // files-only build writes H; a sandbox (packages) build invalidates; the
    // files-only inputs return to H: no CACHED.
    let d = cached_dir("H");
    invalidate_build_cache(d.path());
    assert!(check_build_cache(d.path(), "H").is_none());
}

#[test]
fn base_work_dirs_are_unique_exclusive_and_per_resource() {
    let a = unique_base_work("img").unwrap();
    let b = unique_base_work("img").unwrap();
    assert_ne!(a, b, "two calls in one process must not share a path");
    assert!(a.is_dir() && b.is_dir());
    assert!(a.to_string_lossy().contains("img"));
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}
