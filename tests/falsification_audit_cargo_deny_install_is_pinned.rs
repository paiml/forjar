//! The `audit` job installs cargo-deny with an explicit version and `--locked`.
//! An unpinned install floats to the newest release, whose rustc floor can
//! exceed the fleet toolchain (0.22.0 needs 1.95; runners have 1.93), and the
//! job then fails daily. Reads files only.

use std::path::Path;

fn install_lines() -> Vec<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/audit.yml");
    let text = std::fs::read_to_string(&path).expect("read audit.yml");
    text.lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('#') && l.contains("cargo install") && l.contains("cargo-deny"))
        .map(String::from)
        .collect()
}

#[test]
fn cargo_deny_install_exists_and_is_pinned_and_locked() {
    let lines = install_lines();
    assert!(!lines.is_empty(), "audit.yml must install cargo-deny");
    for l in lines {
        assert!(l.contains("--locked"), "install must be --locked: {l}");
        let ver = l.split("--version").nth(1).unwrap_or("").trim();
        let ok = !ver.is_empty()
            && ver.split('.').count() == 3
            && ver
                .split('.')
                .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
        assert!(ok, "install must carry an exact --version X.Y.Z: {l}");
    }
}
