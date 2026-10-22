//! Regression guard for the committed `Cargo.lock`: quinn-proto must be at
//! 0.11.18 or later.
//!
//! quinn-proto 0.11.17 is affected by RUSTSEC-2026-0338, RUSTSEC-2026-0340 and
//! RUSTSEC-2026-0343; 0.11.18 is the fixed release. The bump is a lock-only
//! change, so nothing else in the suite would notice a later lock refresh or a
//! merge that brings 0.11.17 back. This test reads the lock with std only.
//!
//! It refuses in both directions: a vulnerable version fails, and so does a lock
//! with no quinn-proto entry at all, because zero matches would otherwise pass
//! vacuously while measuring nothing.

use std::path::Path;

const MIN: (u64, u64, u64) = (0, 11, 18);
const ADVISORIES: &str = "RUSTSEC-2026-0338, RUSTSEC-2026-0340, RUSTSEC-2026-0343";

/// Every `version` recorded for a `[[package]]` named `name` in a Cargo.lock.
fn locked_versions(lock: &str, name: &str) -> Vec<String> {
    let mut out = Vec::new();
    for block in lock.split("[[package]]").skip(1) {
        let field = |key: &str| {
            block.lines().find_map(|l| {
                let rest = l.trim().strip_prefix(key)?.trim_start();
                let v = rest.strip_prefix('=')?.trim();
                Some(v.trim_matches('"').to_string())
            })
        };
        if field("name").as_deref() == Some(name) {
            if let Some(v) = field("version") {
                out.push(v);
            }
        }
    }
    out
}

/// True when `version` is at least `MIN`. A pre-release of a version is below
/// that version, so `0.11.18-rc.1` is not accepted. Unparseable is a failure.
fn at_least_min(version: &str) -> bool {
    let (core, pre) = match version.split_once('-') {
        Some((c, _)) => (c, true),
        None => (version.split('+').next().unwrap_or(version), false),
    };
    let parts: Vec<u64> = match core.split('.').map(str::parse).collect() {
        Ok(p) => p,
        Err(_) => return false,
    };
    let [a, b, c] = parts[..] else { return false };
    let v = (a, b, c);
    v > MIN || (v == MIN && !pre)
}

/// The verdict on a lock's text: Ok, or the reason it is refused.
fn check(lock: &str) -> Result<(), String> {
    let versions = locked_versions(lock, "quinn-proto");
    if versions.is_empty() {
        return Err(format!(
            "Cargo.lock has no quinn-proto entry; this guard for {ADVISORIES} \
             measured nothing (if quinn is gone on purpose, retire this test)"
        ));
    }
    let bad: Vec<&String> = versions.iter().filter(|v| !at_least_min(v)).collect();
    if bad.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Cargo.lock pins quinn-proto {bad:?}; versions below 0.11.18 are \
             affected by {ADVISORIES}"
        ))
    }
}

#[test]
fn committed_lock_has_patched_quinn_proto() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock");
    let lock = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    if let Err(why) = check(&lock) {
        panic!("{why}");
    }
}

#[test]
fn guard_refuses_vulnerable_absent_and_prerelease() {
    let entry = |v: &str| {
        format!(
            "[[package]]\nname = \"quinn-proto\"\nversion = \"{v}\"\n\
             source = \"registry+https://github.com/rust-lang/crates.io-index\"\n"
        )
    };
    let other = "[[package]]\nname = \"quinn\"\nversion = \"0.11.9\"\n";
    assert!(check(&entry("0.11.18")).is_ok());
    assert!(check(&entry("0.12.0")).is_ok());
    assert!(check(&entry("0.11.17")).is_err());
    assert!(check(&entry("0.11.18-rc.1")).is_err());
    assert!(check(&entry("garbage")).is_err());
    assert!(check(other).is_err(), "absent quinn-proto must not pass");
    assert!(check(&format!("{other}{}", entry("0.11.17"))).is_err());
    assert!(check(&format!("{}{}", entry("0.11.18"), entry("0.11.17"))).is_err());
}
