//! #714 (PMAT-714): `cargo deny check advisories` reds `ci / lint` on
//! RUSTSEC-2024-0449 — unsafe-libyaml 0.2.11 is unmaintained and has no
//! patched release. It enters through four crates, two of which forjar does not
//! own (bashrs 7.4.1, pforge-config 0.1.4), so no change in this repo can drop
//! it from Cargo.lock today. deny.toml carries ONE scoped ignore for that ID.
//!
//! An ignore is only honest while it is bounded, so this test pins both ends:
//!
//! * PRESENT while the crate is locked: without the deny.toml line, `ci / lint`
//!   is red on every PR (the defect). Revert the line and this test fails.
//! * STALE once the crate is gone: when bashrs#452, a pforge release and
//!   forjar's own YAML swap take unsafe-libyaml out of Cargo.lock, the ignore
//!   must be deleted in the same change, or this test fails.
//! * SCOPED: the ignore silences the ID for any crate that pulls unsafe-libyaml,
//!   so the set of locked crates that reach it is pinned to the four reviewed
//!   ones. A fifth consumer fails here and has to be reviewed, not inherited.
//! * IN SYNC: cargo-audit reads `.cargo/audit.toml`, not deny.toml, and both
//!   lanes run in audit.yml, so the ID must be in both or neither.
//!
//! Reads files only: no network, no advisory DB, no cargo subprocess.

use std::collections::BTreeSet;
use std::path::Path;

const ID: &str = "RUSTSEC-2024-0449";
const CRATE: &str = "unsafe-libyaml";
/// The YAML crates that link unsafe-libyaml directly.
const YAML_CRATES: &[&str] = &["serde_yaml", "serde_yaml_ng"];
/// The reviewed crates that reach unsafe-libyaml through `YAML_CRATES`.
const REVIEWED_CONSUMERS: &[&str] = &["bashrs", "forjar", "forjar-contracts", "pforge-config"];

fn read(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// `(name, dependency names)` for every `[[package]]` in Cargo.lock. A
/// dependency entry is `"name"` or `"name version"`; only the name is kept.
fn lock_packages() -> Vec<(String, Vec<String>)> {
    let lock = read("Cargo.lock");
    let mut out = Vec::new();
    for block in lock.split("[[package]]").skip(1) {
        let mut name = None;
        let mut deps = Vec::new();
        let mut in_deps = false;
        for line in block.lines() {
            let line = line.trim();
            if let Some(v) = line.strip_prefix("name = ") {
                name = Some(v.trim_matches('"').to_string());
            } else if line == "dependencies = [" {
                in_deps = true;
            } else if in_deps && line == "]" {
                in_deps = false;
            } else if in_deps {
                let entry = line.trim_end_matches(',').trim_matches('"');
                if let Some(dep) = entry.split_whitespace().next() {
                    deps.push(dep.to_string());
                }
            }
        }
        let name = name.expect("a Cargo.lock [[package]] block without a name");
        out.push((name, deps));
    }
    assert!(
        out.len() > 100,
        "parsed only {} packages from Cargo.lock: parser broken",
        out.len()
    );
    out
}

fn crate_is_locked() -> bool {
    lock_packages().iter().any(|(n, _)| n == CRATE)
}

/// The deny.toml `[advisories].ignore` entry naming `ID`, if any (one line).
fn deny_ignore_line() -> Option<String> {
    let deny = read("deny.toml");
    let section = deny
        .split("\n[advisories]")
        .nth(1)
        .and_then(|s| s.split("\n[").next())
        .expect("deny.toml has no [advisories] section");
    section
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('#'))
        .find(|l| l.contains(ID))
        .map(str::to_string)
}

fn audit_toml_ignores_id() -> bool {
    read(".cargo/audit.toml")
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('#'))
        .any(|l| l.contains(&format!("\"{ID}\"")))
}

#[test]
fn the_ignore_is_present_exactly_while_the_crate_is_locked() {
    let locked = crate_is_locked();
    let ignored = deny_ignore_line().is_some();
    assert_eq!(
        locked, ignored,
        "{CRATE} locked = {locked}, deny.toml ignores {ID} = {ignored}.\n\
         Locked without the ignore: `cargo deny check advisories` reds ci / lint on every PR.\n\
         Ignored without the crate: the ignore is stale; delete it (and its .cargo/audit.toml twin)."
    );
}

#[test]
fn the_ignore_carries_a_reason_and_its_removal_ticket() {
    let Some(line) = deny_ignore_line() else {
        assert!(
            !crate_is_locked(),
            "{CRATE} is locked and deny.toml does not ignore {ID}"
        );
        return;
    };
    let reason = line
        .split("reason")
        .nth(1)
        .and_then(|r| r.split('"').nth(1))
        .unwrap_or("");
    assert!(
        reason.len() >= 40,
        "the {ID} ignore has no real reason: {line}"
    );
    assert!(
        reason.contains("#714"),
        "the {ID} ignore does not name its removal ticket #714: {line}"
    );
}

#[test]
fn only_the_reviewed_crates_reach_unsafe_libyaml() {
    let pkgs = lock_packages();
    let direct: BTreeSet<&str> = pkgs
        .iter()
        .filter(|(_, d)| d.iter().any(|x| x == CRATE))
        .map(|(n, _)| n.as_str())
        .collect();
    for n in &direct {
        assert!(
            YAML_CRATES.contains(n),
            "{n} now links {CRATE} directly; review it before the {ID} ignore covers it"
        );
    }
    let consumers: BTreeSet<&str> = pkgs
        .iter()
        .filter(|(_, d)| d.iter().any(|x| direct.contains(x.as_str())))
        .map(|(n, _)| n.as_str())
        .collect();
    for n in &consumers {
        assert!(
            REVIEWED_CONSUMERS.contains(n),
            "{n} now pulls {CRATE} through a YAML crate; review it before the {ID} ignore covers it"
        );
    }
}

#[test]
fn deny_and_audit_exemption_lists_agree_on_this_id() {
    assert_eq!(
        deny_ignore_line().is_some(),
        audit_toml_ignores_id(),
        "deny.toml and .cargo/audit.toml disagree on {ID}; audit.yml runs both, keep them in sync"
    );
}
