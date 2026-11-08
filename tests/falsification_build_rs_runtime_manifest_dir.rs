//! #456: build.rs must read CARGO_MANIFEST_DIR at run time, not bake it via env!().
#[test]
fn build_rs_does_not_bake_manifest_dir() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/build.rs")).unwrap();
    assert!(
        !src.contains("env!(\"CARGO_MANIFEST_DIR\")"),
        "build.rs bakes CARGO_MANIFEST_DIR at build-script compile time (#456)"
    );
}
