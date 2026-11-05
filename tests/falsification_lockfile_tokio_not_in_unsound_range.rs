//! EMG-REL-001 E1 (PMAT-775): `Cargo.lock` must not pin a tokio release inside
//! RUSTSEC-2026-0386 (`AsyncFd::new` violates I/O safety, class `unsound`).
//!
//! `cargo deny check` 0.20 reports an `unsound` advisory as an ERROR, so the
//! scheduled Security Audit on main went red (run 40086542926) and every PR
//! running `ci / lint` and `ci / security` inherited it. The fix is a lock bump,
//! never an ignore entry: the advisory's patched ranges are
//! `>= 1.51.6, < 1.52.0` and `>= 1.53.3`. Reads files only: no network.

use std::path::Path;

const ID: &str = "RUSTSEC-2026-0386";

fn read(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// Versions of every locked `tokio` package, as `(major, minor, patch)`.
fn locked_tokio() -> Vec<(u32, u32, u32)> {
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
        if name == "tokio" {
            let p: Vec<u32> = ver.split('.').map(|n| n.parse().unwrap()).collect();
            out.push((p[0], p[1], p[2]));
        }
    }
    out
}

fn patched(v: (u32, u32, u32)) -> bool {
    ((1, 51, 6)..(1, 52, 0)).contains(&v) || v >= (1, 53, 3)
}

#[test]
fn locked_tokio_is_outside_the_unsound_range() {
    let pins = locked_tokio();
    assert!(
        !pins.is_empty(),
        "no tokio in Cargo.lock: test measures nothing"
    );
    for v in pins {
        assert!(
            patched(v),
            "{ID}: tokio {v:?} is locked in the unsound range; `cargo update -p tokio`"
        );
    }
}

#[test]
fn advisory_is_fixed_not_ignored() {
    for f in ["deny.toml", ".cargo/audit.toml"] {
        assert!(
            !read(f).contains(ID),
            "{ID} must be fixed by a lock bump, not ignored in {f}"
        );
    }
}
