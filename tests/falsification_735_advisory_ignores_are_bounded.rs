//! PMAT-735 (#835): the 14 advisory ignores added for the zstd advisories
//! (RUSTSEC-2026-0391, -0395..-0406) and basic-toml (RUSTSEC-2026-0374) are
//! only honest while bounded. Each ignore must be
//!
//! * LOCKED: its crate is still in Cargo.lock, else the ignore is stale and
//!   must be deleted in the same change;
//! * IN SYNC: present in both deny.toml and .cargo/audit.toml (audit.yml runs
//!   cargo-deny AND cargo-audit), with the identical reason text;
//! * TRACED: its reason names the removal condition.
//!
//! Reads files only: no network, no advisory DB, no cargo subprocess.

use std::path::Path;

/// `(advisory id, locked crate name, locked version)`.
fn ignores() -> Vec<(String, &'static str, &'static str)> {
    let mut v: Vec<(String, &'static str, &'static str)> = vec![
        ("RUSTSEC-2026-0391".into(), "zstd", "0.13.3"),
        ("RUSTSEC-2026-0374".into(), "basic-toml", "0.1.10"),
    ];
    for n in 395..=406 {
        v.push((format!("RUSTSEC-2026-0{n}"), "zstd", "0.13.3"));
    }
    v
}

fn read(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

fn locked(name: &str, version: &str) -> bool {
    read("Cargo.lock").split("[[package]]").skip(1).any(|b| {
        b.contains(&format!("name = \"{name}\"\n"))
            && b.contains(&format!("version = \"{version}\"\n"))
    })
}

/// The deny.toml reason for `id`, if the id is ignored (comments skipped).
fn deny_reason(id: &str) -> Option<String> {
    read("deny.toml")
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('#') && l.contains(&format!("\"{id}\"")))
        .find_map(|l| {
            l.split("reason = \"").nth(1).map(|r| {
                r.trim_end_matches(|c| c != '"')
                    .trim_end_matches('"')
                    .to_string()
            })
        })
}

/// The `.cargo/audit.toml` comment for `id`, if the id is ignored.
fn audit_reason(id: &str) -> Option<String> {
    read(".cargo/audit.toml")
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with(&format!("\"{id}\"")))
        .find_map(|l| l.split(",  # ").nth(1).map(str::to_string))
}

#[test]
fn each_ignore_is_present_exactly_while_its_crate_is_locked() {
    for (id, krate, ver) in ignores() {
        assert_eq!(
            locked(krate, ver),
            deny_reason(&id).is_some(),
            "{id}: {krate} {ver} locked = {}, deny.toml ignores it = {}; delete a stale ignore, review a new one",
            locked(krate, ver),
            deny_reason(&id).is_some()
        );
    }
}

#[test]
fn deny_and_audit_agree_with_identical_reasons() {
    for (id, _, _) in ignores() {
        let (d, a) = (deny_reason(&id), audit_reason(&id));
        assert!(
            d.is_some() && a.is_some(),
            "{id}: missing from deny.toml ({d:?}) or audit.toml ({a:?})"
        );
        assert_eq!(d, a, "{id}: deny.toml and .cargo/audit.toml reasons differ");
    }
}

#[test]
fn each_reason_names_its_removal_condition() {
    for (id, _, _) in ignores() {
        let r = deny_reason(&id).unwrap_or_default();
        assert!(
            r.len() >= 40 && r.contains("removed when"),
            "{id}: reason lacks a removal condition: {r}"
        );
    }
}

#[test]
fn the_ignore_set_does_not_grow_unreviewed() {
    let n = read("deny.toml")
        .lines()
        .map(str::trim)
        .filter(|l| {
            !l.starts_with('#')
                && l.contains("RUSTSEC-2026-0")
                && ignores().iter().any(|(i, _, _)| l.contains(i.as_str()))
        })
        .count();
    assert_eq!(n, 14, "the PMAT-735 ignore set is exactly 14 lines");
}
