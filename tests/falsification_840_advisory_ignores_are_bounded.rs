//! PMAT-840 (#835): the 14 advisory ignores added for the zstd advisories
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
    assert_eq!(n, 14, "the PMAT-840 ignore set is exactly 14 lines");
}

// ---- reachability: the ignore silences the ID for ANY consumer of the crate,
// so the locked dependents are pinned to the reviewed ones.

/// A dependency edge: `(dep name, dep version if the lock spells one)`.
type LockDep = (String, Option<String>);

/// `(name, version, deps)` for one locked package.
type LockPkg = (String, String, Vec<LockDep>);

fn lock_graph() -> Vec<LockPkg> {
    let mut out = Vec::new();
    for block in read("Cargo.lock").split("[[package]]").skip(1) {
        let (mut name, mut version, mut deps, mut in_deps) = (None, None, Vec::new(), false);
        for line in block.lines().map(str::trim) {
            if let Some(v) = line.strip_prefix("name = ") {
                name = Some(v.trim_matches('"').to_string());
            } else if let Some(v) = line.strip_prefix("version = ") {
                version = Some(v.trim_matches('"').to_string());
            } else if line == "dependencies = [" {
                in_deps = true;
            } else if in_deps && line == "]" {
                in_deps = false;
            } else if in_deps {
                let mut it = line
                    .trim_end_matches(',')
                    .trim_matches('"')
                    .split_whitespace();
                if let Some(d) = it.next() {
                    deps.push((d.to_string(), it.next().map(str::to_string)));
                }
            }
        }
        out.push((
            name.expect("package without name"),
            version.expect("package without version"),
            deps,
        ));
    }
    assert!(
        out.len() > 100,
        "parsed only {} packages: parser broken",
        out.len()
    );
    out
}

/// Names of the packages whose dependency list reaches `name@version`. A dep
/// spelled without a version resolves to the sole locked version of that name.
fn dependents(name: &str, version: &str) -> Vec<String> {
    let graph = lock_graph();
    let versions: Vec<&str> = graph
        .iter()
        .filter(|(n, _, _)| n == name)
        .map(|(_, v, _)| v.as_str())
        .collect();
    let mut out: Vec<String> = graph
        .iter()
        .filter(|(_, _, deps)| {
            deps.iter().any(|(d, v)| {
                d == name
                    && match v {
                        Some(v) => v == version,
                        None => versions == [version],
                    }
            })
        })
        .map(|(n, _, _)| n.clone())
        .collect();
    out.sort();
    out
}

#[test]
fn zstd_0_13_reaches_forjar_only_through_batuta_common() {
    assert_eq!(
        dependents("zstd", "0.13.3"),
        ["batuta-common"],
        "zstd 0.13.3 must be consumed by batuta-common alone; forjar's own zstd is 0.14"
    );
    assert_eq!(
        dependents("zstd-safe", "7.2.4"),
        ["zstd"],
        "zstd-safe 7.2.4 must be consumed only by zstd 0.13.3"
    );
}

#[test]
fn basic_toml_reaches_forjar_only_through_i18n_config() {
    assert_eq!(dependents("basic-toml", "0.1.10"), ["i18n-config"]);
}

// ---- totals: a 15th ignore (or any new one) fails here.

fn deny_ids() -> Vec<String> {
    let mut v: Vec<String> = read("deny.toml")
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("{ id = \"RUSTSEC-"))
        .filter_map(|l| l.split('"').nth(1).map(str::to_string))
        .collect();
    v.sort();
    v
}

fn audit_ids() -> Vec<String> {
    let mut v: Vec<String> = read(".cargo/audit.toml")
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("\"RUSTSEC-"))
        .filter_map(|l| l.split('"').nth(1).map(str::to_string))
        .collect();
    v.sort();
    v
}

#[test]
fn every_ignore_line_is_counted_in_both_files() {
    assert_eq!(
        deny_ids().len(),
        23,
        "deny.toml ignore lines: 8 older + unsafe-libyaml + the 14 of PMAT-840"
    );
    assert_eq!(
        audit_ids().len(),
        24,
        "audit.toml ignore lines: the deny.toml set plus the legacy RUSTSEC-2026-0097"
    );
    let extra: Vec<String> = audit_ids()
        .into_iter()
        .filter(|i| !deny_ids().contains(i))
        .collect();
    assert_eq!(
        extra,
        ["RUSTSEC-2026-0097"],
        "audit.toml may differ from deny.toml only by the legacy -0097"
    );
}

#[test]
fn the_basic_toml_ignore_cites_its_upstream_advisory() {
    let r = deny_reason("RUSTSEC-2026-0374").expect("ignore missing");
    assert!(
        r.contains("https://rustsec.org/advisories/RUSTSEC-2026-0374.html"),
        "no upstream URL: {r}"
    );
}
