//! forjar#456: build.rs must resolve `contracts/` at run time.
//!
//! `env!("CARGO_MANIFEST_DIR")` is expanded when the build script is COMPILED,
//! so a reused build-script binary (same CARGO_TARGET_DIR, new source dir)
//! reads the old directory. `std::env::var` reads it when the script RUNS.

fn code_lines() -> Vec<String> {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/build.rs"))
        .expect("read build.rs");
    src.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .map(str::to_string)
        .collect()
}

#[test]
fn build_rs_has_no_compile_time_manifest_dir() {
    let bad: Vec<_> = code_lines()
        .into_iter()
        .filter(|l| {
            l.contains("CARGO_MANIFEST_DIR") && (l.contains("env!(") || l.contains("option_env!("))
        })
        .collect();
    assert!(
        bad.is_empty(),
        "compile-time CARGO_MANIFEST_DIR in build.rs: {bad:?}"
    );
}

#[test]
fn build_rs_reads_manifest_dir_at_run_time() {
    let n = code_lines()
        .iter()
        .filter(|l| l.contains("std::env::var(\"CARGO_MANIFEST_DIR\")"))
        .count();
    assert!(
        n >= 2,
        "expected both contracts/ lookups to read the env at run time, found {n}"
    );
}

/// Reading the variable at run time fixes WHAT build.rs reads when it runs.
/// Whether it runs at all under a shared `CARGO_TARGET_DIR` is decided by
/// cargo's rerun directives: without this one, a second checkout whose
/// `contracts/` is not newer than the cached output reuses the first
/// checkout's `CONTRACT_*` values.
#[test]
fn build_rs_reruns_when_the_manifest_dir_changes() {
    let n = code_lines()
        .iter()
        .filter(|l| l.contains("cargo::rerun-if-env-changed=CARGO_MANIFEST_DIR"))
        .count();
    assert!(
        n >= 1,
        "build.rs must emit cargo::rerun-if-env-changed=CARGO_MANIFEST_DIR, found {n}"
    );
}
