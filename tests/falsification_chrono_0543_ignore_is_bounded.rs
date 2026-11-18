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

/// The `review by YYYY-MM-DD` date in a reason, if it is well formed.
fn review_by(reason: &str) -> Option<String> {
    let rest = reason.split("review by ").nth(1)?;
    let d: String = rest.chars().take(10).collect();
    let b = d.as_bytes();
    let ok = b.len() == 10
        && b.iter().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                *c == b'-'
            } else {
                c.is_ascii_digit()
            }
        });
    ok.then_some(d)
}

/// Version of `name` in the lock text.
fn locked_version(lock: &str, name: &str) -> Option<String> {
    lock.split("[[package]]").skip(1).find_map(|block| {
        let mut lines = block.lines().map(str::trim).filter(|l| !l.is_empty());
        let n = lines.next()?.strip_prefix("name = ")?.trim_matches('"');
        if n != name {
            return None;
        }
        let v = lines.next()?.strip_prefix("version = ")?;
        Some(v.trim_matches('"').to_string())
    })
}

/// Reason text names `<crate> <version>`; true when each matches the lock.
fn versions_ok(lock: &str, reason: &str) -> bool {
    [CRATE, "bashrs", "pmcp"]
        .iter()
        .all(|c| locked_version(lock, c).is_some_and(|v| reason.contains(&format!("{c} {v}"))))
}

#[test]
fn both_rows_carry_the_same_review_by_date() {
    let d = review_by(&deny_reason().unwrap_or_default());
    let a = review_by(&audit_reason().unwrap_or_default());
    assert!(
        d.is_some(),
        "deny.toml {ID} reason lacks `review by YYYY-MM-DD`"
    );
    assert!(
        a.is_some(),
        ".cargo/audit.toml {ID} reason lacks `review by YYYY-MM-DD`"
    );
    assert_eq!(
        d, a,
        "review-by dates differ between deny.toml and audit.toml"
    );
}

#[test]
fn the_reason_versions_match_cargo_lock() {
    let lock = read("Cargo.lock");
    let r = deny_reason().unwrap_or_default();
    assert!(
        versions_ok(&lock, &r),
        "reason names versions that differ from Cargo.lock (chrono/bashrs/pmcp): {r}"
    );
    // mutation: chrono bumped in the lock while the reason still says 0.4.45.
    let v = locked_version(&lock, CRATE).expect("chrono locked");
    let bumped = lock.replacen(
        &format!("name = \"chrono\"\nversion = \"{v}\""),
        "name = \"chrono\"\nversion = \"0.4.99\"",
        1,
    );
    assert_ne!(bumped, lock, "mutation did not apply");
    assert!(
        !versions_ok(&bumped, &r),
        "a chrono bump must make the reason stale"
    );
}

/// UTC date `YYYY-MM-DD` for a count of days since 1970-01-01 (civil-from-days).
fn civil(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// True when the zero-padded `review` date is earlier than `today`.
fn expired(review: &str, today: &str) -> bool {
    review < today
}

#[test]
fn expiry_compares_against_an_injected_clock() {
    assert_eq!(civil(0), "1970-01-01");
    assert_eq!(civil(20_000), "2024-10-04");
    assert!(expired("2027-02-17", "2027-02-18"), "date before today");
    assert!(!expired("2027-02-17", "2027-02-17"), "date on today");
    assert!(!expired("2027-02-17", "2027-02-16"), "date after today");
}

#[test]
fn the_review_by_date_has_not_passed() {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after 1970")
        .as_secs();
    let today = civil(i64::try_from(secs / 86_400).expect("day count fits"));
    let d = review_by(&deny_reason().unwrap_or_default()).expect("review-by date");
    assert!(
        !expired(&d, &today),
        "{ID} review-by {d} is earlier than today {today} (UTC): re-audit and move the date"
    );
}
