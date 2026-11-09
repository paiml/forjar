//! forjar#939: `Cargo.lock` must not pin a `libsqlite3-sys` release inside
//! RUSTSEC-2026-0459 (bundled C SQLite CVE-2026-119375, patched `>= 0.39.0`).
//!
//! `cargo audit` fails `ci / security` on every PR while the lock pins the
//! vulnerable range. The fix is a `rusqlite` bump that moves the lock, never an
//! ignore entry (an ignore is a waiver). Reads files only: no network.

use std::path::Path;

fn read(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// Versions of every locked `libsqlite3-sys` package, as `(major, minor, patch)`.
fn locked_libsqlite3_sys() -> Vec<(u32, u32, u32)> {
    let lock = read("Cargo.lock");
    let mut out = Vec::new();
    for block in lock.split("[[package]]").skip(1) {
        let mut name = "";
        let mut ver = "";
        for line in block.lines() {
            if let Some(v) = line.strip_prefix("name = ") {
                name = v.trim_matches('"');
            } else if let Some(v) = line.strip_prefix("version = ") {
                ver = v.trim_matches('"');
            }
        }
        if name == "libsqlite3-sys" {
            let p: Vec<u32> = ver.split('.').map(|n| n.parse().unwrap()).collect();
            out.push((p[0], p[1], p[2]));
        }
    }
    out
}

#[test]
fn lock_pins_no_libsqlite3_sys_below_0_39_0() {
    for v in locked_libsqlite3_sys() {
        assert!(
            v >= (0, 39, 0),
            "Cargo.lock pins libsqlite3-sys {}.{}.{}, inside RUSTSEC-2026-0459 (fixed in >= 0.39.0): bump rusqlite, do not ignore the advisory",
            v.0, v.1, v.2
        );
    }
}

#[test]
fn advisory_is_not_ignored_in_audit_or_deny_config() {
    for rel in ["deny.toml", ".cargo/audit.toml"] {
        let body = read(rel);
        let ignored = body
            .lines()
            .filter(|l| !l.trim_start().starts_with('#'))
            .any(|l| l.contains("RUSTSEC-2026-0459"));
        assert!(
            !ignored,
            "{rel} ignores RUSTSEC-2026-0459: that is a waiver, bump the lock instead"
        );
    }
}
