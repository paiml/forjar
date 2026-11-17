//! RUSTSEC-2026-0543: chrono is unmaintained (all versions affected, no fixed
//! release, no vulnerability). It is locked only through bashrs and pmcp, both
//! of which depend on it non-optionally, so forjar cannot drop it alone.
//! deny.toml and .cargo/audit.toml carry ONE dated ignore for that ID.
//!
//! The ignore is only honest while it is bounded; this test pins it:
//!
//! * PRESENT exactly while chrono is locked: without the row the advisory
//!   check is red; with the row after chrono left the lock, the row is stale.
//! * SCOPED: chrono's locked reverse dependencies are exactly {bashrs, pmcp};
//!   a new dependent fails here and has to be reviewed, not inherited.
//! * IN SYNC: both files carry the id with the identical reason text.
//! * REMOVABLE: the reason states its removal condition.
//!
//! Reads files only: no network, no advisory DB, no cargo subprocess.

use std::collections::BTreeSet;
use std::path::Path;

const ID: &str = "RUSTSEC-2026-0543";
const CRATE: &str = "chrono";
const REVIEWED_DEPENDENTS: &[&str] = &["bashrs", "pmcp"];

fn read(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// `(name, dependency names)` for every `[[package]]` in the given lock text.
fn parse_lock(lock: &str) -> Vec<(String, Vec<String>)> {
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
        out.push((name.expect("a [[package]] block without a name"), deps));
    }
    out
}

fn locked(pkgs: &[(String, Vec<String>)]) -> bool {
    pkgs.iter().any(|(n, _)| n == CRATE)
}

fn dependents(pkgs: &[(String, Vec<String>)]) -> BTreeSet<String> {
    pkgs.iter()
        .filter(|(_, d)| d.iter().any(|x| x == CRATE))
        .map(|(n, _)| n.clone())
        .collect()
}

fn deny_line() -> Option<String> {
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

fn deny_reason() -> Option<String> {
    deny_line().and_then(|l| {
        l.split("reason = \"")
            .nth(1)
            .map(|r| r.split("\" }").next().unwrap_or("").to_string())
    })
}

fn audit_reason() -> Option<String> {
    read(".cargo/audit.toml")
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with(&format!("\"{ID}\"")))
        .find_map(|l| l.split("# ").nth(1).map(str::to_string))
}

/// The pure check, so it can be exercised against a doctored lock.
fn presence_ok(lock: &str, row_present: bool) -> bool {
    locked(&parse_lock(lock)) == row_present
}

fn dependents_ok(lock: &str) -> bool {
    let want: BTreeSet<String> = REVIEWED_DEPENDENTS.iter().map(|s| s.to_string()).collect();
    dependents(&parse_lock(lock)) == want
}

#[test]
fn the_ignore_is_present_exactly_while_chrono_is_locked() {
    assert!(
        presence_ok(&read("Cargo.lock"), deny_line().is_some()),
        "chrono locked vs deny.toml ignore of {ID} disagree: add the row, or delete the stale row (and its .cargo/audit.toml twin)"
    );
}

#[test]
fn chrono_is_reached_only_by_bashrs_and_pmcp() {
    assert!(
        dependents_ok(&read("Cargo.lock")),
        "chrono's locked dependents are {:?}, reviewed set is {REVIEWED_DEPENDENTS:?}",
        dependents(&parse_lock(&read("Cargo.lock")))
    );
}

#[test]
fn deny_and_audit_carry_the_same_id_with_identical_reason() {
    let d = deny_reason();
    let a = audit_reason();
    assert!(d.is_some(), "deny.toml lacks {ID}");
    assert!(a.is_some(), ".cargo/audit.toml lacks {ID}");
    assert_eq!(
        d, a,
        "deny.toml and .cargo/audit.toml reasons differ for {ID}"
    );
}

#[test]
fn the_reason_names_its_removal_condition() {
    let r = deny_reason().unwrap_or_default();
    assert!(
        r.contains("removed when"),
        "{ID} reason has no removal condition: {r}"
    );
    assert!(
        r.contains("audited 2026-11-17"),
        "{ID} reason has no audit date: {r}"
    );
}

#[test]
fn the_checks_fail_on_a_doctored_lock() {
    let lock = read("Cargo.lock");
    // chrono gone from the lock while the row exists -> stale row detected.
    let without: String = lock.replace("name = \"chrono\"", "name = \"chrono-gone\"");
    assert!(!presence_ok(&without, true), "stale row must be detected");
    // a third dependent appears -> scope check fails.
    let extra = format!("{lock}\n[[package]]\nname = \"newcomer\"\nversion = \"1.0.0\"\ndependencies = [\n \"chrono\",\n]\n");
    assert!(!dependents_ok(&extra), "a new dependent must be detected");
}
