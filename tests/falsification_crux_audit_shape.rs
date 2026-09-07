//! PMAT-164: the committed-quorum gate requires a falsification test for
//! `docs/audits/crux-1.26.0.md` itself — the competitive-UX audit for
//! release 1.26.0. The audit is prose + a markdown table, not code, so the
//! "revert" this test must catch is `docs(audits): ...` being emptied or
//! shrunk back to a stub, not a source-level regression.
//!
//! WHAT THIS TEST MUST NOT BECOME. A test that only checks the file exists
//! would pass against a one-line stub. Each assertion below instead checks
//! a structural property the audit's own spec demands: nine behaviours,
//! each with >= 3 reference-system rows, every row citing both a
//! third-party system `[X]` and forjar's own code `[V ...]`, a single
//! disposition per row, a named module citation (not the pre-refactor
//! `stamp.rs` file, since PMAT-161 made `state/stamp` a directory), a
//! GO-spec section naming its two candidate precedents, a Reconciliation
//! section mapping every `CHANGELOG.md` bullet to a row, and no leaked
//! `[X]` marker outside the audit (README.md / docs/book/src are read by
//! humans who were never told `[X]` means "asserted from memory, not
//! measured").
//!
//! RED OBSERVED: emptying `docs/audits/crux-1.26.0.md` with
//! `: > docs/audits/crux-1.26.0.md` and running this suite fails every
//! test except `readme_and_book_never_leak_the_x_marker` (which is
//! vacuously true against an empty repo state, since neither README.md
//! nor docs/book/src changed) and `the_audit_cites_no_pre_refactor_stamp_module`
//! and `the_audit_names_no_flat_owner_yaml` (also vacuously true against an
//! empty file — there is nothing to cite). The other five tests
//! (`the_audit_exists_and_its_title_names_1_26_0`,
//! `every_behaviour_has_at_least_three_reference_rows`,
//! `every_row_has_exactly_one_disposition`,
//! `every_row_cites_a_third_party_and_forjars_own_code`,
//! `the_go_spec_section_names_its_two_precedents`,
//! `the_reconciliation_section_maps_every_changelog_bullet`) go RED.

use std::path::Path;

fn audit_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/audits/crux-1.26.0.md")
}

fn read_audit() -> String {
    let p = audit_path();
    std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("RULE 1 (file exists): could not read {}: {e}", p.display()))
}

/// Rows are lines starting with `| (<n>)` — the first cell of the Findings
/// table names the behaviour number in parentheses.
fn behaviour_rows(content: &str) -> Vec<&str> {
    content
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            t.starts_with('|') && {
                let after_pipe = t[1..].trim_start();
                after_pipe.starts_with('(')
                    && after_pipe[1..]
                        .chars()
                        .next()
                        .map(|c| c.is_ascii_digit())
                        .unwrap_or(false)
            }
        })
        .collect()
}

/// Splits a markdown table row into trimmed cells, dropping the empty
/// leading/trailing element produced by the row's outer `|` delimiters.
fn split_row(line: &str) -> Vec<String> {
    let mut cells: Vec<String> = line.split('|').map(|c| c.trim().to_string()).collect();
    if cells.first().map(|s| s.is_empty()).unwrap_or(false) {
        cells.remove(0);
    }
    if cells.last().map(|s| s.is_empty()).unwrap_or(false) {
        cells.pop();
    }
    cells
}

/// Extracts the behaviour number `n` from a row's first cell, e.g.
/// `(7) undo/rollback refused ...` -> `7`.
fn row_behaviour_number(row: &str) -> Option<u32> {
    let cells = split_row(row);
    let first = cells.first()?;
    let rest = first.strip_prefix('(')?;
    let end = rest.find(')')?;
    rest[..end].parse().ok()
}

/// RULE 1: the audit file exists and its title names release 1.26.0.
#[test]
fn the_audit_exists_and_its_title_names_1_26_0() {
    let content = read_audit();
    let title = content
        .lines()
        .find(|l| l.trim_start().starts_with("# "))
        .unwrap_or_else(|| panic!("RULE 1: no top-level `# ` title in the audit"));
    assert!(
        title.contains("1.26.0"),
        "RULE 1: audit title must name 1.26.0, got: {title:?}"
    );
}

/// RULE 2: every behaviour (1) through (9) has at least three table rows.
#[test]
fn every_behaviour_has_at_least_three_reference_rows() {
    let content = read_audit();
    let rows = behaviour_rows(&content);
    for n in 1..=9u32 {
        let count = rows
            .iter()
            .filter(|r| row_behaviour_number(r) == Some(n))
            .count();
        assert!(
            count >= 3,
            "RULE 2: behaviour ({n}) has {count} table rows, need >= 3 reference systems"
        );
    }
}

/// RULE 3: every table row's disposition cell is `adopt(PMAT-...)` or
/// `reject(...)` — exactly one disposition per row, no bare/empty cell.
#[test]
fn every_row_has_exactly_one_disposition() {
    let content = read_audit();
    let rows = behaviour_rows(&content);
    assert!(!rows.is_empty(), "RULE 3: no behaviour rows found at all");
    for row in &rows {
        let cells = split_row(row);
        let disposition = cells
            .last()
            .unwrap_or_else(|| panic!("RULE 3: row has no cells at all: {row}"));
        assert!(
            disposition.starts_with("adopt(PMAT-") || disposition.starts_with("reject("),
            "RULE 3: row's disposition cell must match adopt(PMAT- or reject(, got {disposition:?} in row: {row}"
        );
    }
}

/// RULE 4: every row's third-party cell carries `[X]` and every row's
/// forjar cell carries `[V`.
#[test]
fn every_row_cites_a_third_party_and_forjars_own_code() {
    let content = read_audit();
    let rows = behaviour_rows(&content);
    assert!(!rows.is_empty(), "RULE 4: no behaviour rows found at all");
    for row in &rows {
        let cells = split_row(row);
        assert!(
            cells.len() >= 4,
            "RULE 4: row has only {} cells, expected >= 4 (behaviour, system, third-party, forjar, ...): {row}",
            cells.len()
        );
        let third_party = &cells[2];
        let forjar = &cells[3];
        assert!(
            third_party.contains("[X]"),
            "RULE 4: third-party cell must carry [X], got {third_party:?} in row: {row}"
        );
        assert!(
            forjar.contains("[V"),
            "RULE 4: forjar cell must carry [V, got {forjar:?} in row: {row}"
        );
    }
}

/// RULE 5: the audit cites no `src/core/state/stamp.rs` — PMAT-161 made
/// `state/stamp` a directory (`state/stamp/mod.rs`), so a citation of the
/// old flat file is stale and unresolvable on the audited branch.
#[test]
fn the_audit_cites_no_pre_refactor_stamp_module() {
    let content = read_audit();
    assert!(
        !content.contains("src/core/state/stamp.rs"),
        "RULE 5: audit cites the pre-refactor flat file `src/core/state/stamp.rs`; \
         the module is a directory (`src/core/state/stamp/mod.rs`) since PMAT-161"
    );
}

/// RULE 5 (second half): the audit names no `owner.yaml`.
#[test]
fn the_audit_names_no_flat_owner_yaml() {
    let content = read_audit();
    assert!(
        !content.contains("owner.yaml"),
        "RULE 5: audit must not cite a flat `owner.yaml`"
    );
}

/// RULE 6: a section headed "What the GO spec" exists and names both Nix
/// profile generations and Kubernetes rollout history.
#[test]
fn the_go_spec_section_names_its_two_precedents() {
    let content = read_audit();
    let lines: Vec<&str> = content.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.trim_start().starts_with("##") && l.contains("What the GO spec"))
        .unwrap_or_else(|| panic!("RULE 6: no section headed \"What the GO spec\" found"));
    let end = lines[start + 1..]
        .iter()
        .position(|l| l.trim_start().starts_with("## "))
        .map(|i| start + 1 + i)
        .unwrap_or(lines.len());
    let section = lines[start..end].join("\n");
    assert!(
        section.contains("Nix profile generations"),
        "RULE 6: \"What the GO spec\" section must name Nix profile generations"
    );
    assert!(
        section.contains("Kubernetes rollout history"),
        "RULE 6: \"What the GO spec\" section must name Kubernetes rollout history"
    );
}

/// RULE 7: a Reconciliation section exists with at least nine mapped
/// entries (one per CHANGELOG behaviour bullet).
#[test]
fn the_reconciliation_section_maps_every_changelog_bullet() {
    let content = read_audit();
    let lines: Vec<&str> = content.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.trim_start().starts_with("## Reconciliation"))
        .unwrap_or_else(|| panic!("RULE 7: no \"## Reconciliation\" section found"));
    let end = lines[start + 1..]
        .iter()
        .position(|l| l.trim_start().starts_with("## "))
        .map(|i| start + 1 + i)
        .unwrap_or(lines.len());
    let section = &lines[start..end];
    let mapped = section
        .iter()
        .filter(|l| {
            let t = l.trim_start();
            // e.g. "1. **(1)** \"...\"" — a numbered entry naming its
            // behaviour number in bold parentheses.
            t.chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
                && t.contains(". **(")
        })
        .count();
    assert!(
        mapped >= 9,
        "RULE 7: Reconciliation section has {mapped} mapped entries, need >= 9"
    );
}

/// RULE 8: neither README.md nor any file under docs/book/src contains the
/// literal `[X]` marker — that notation means "asserted from memory, not
/// measured" only inside this audit; leaking it into human-facing docs
/// would be read as a checkbox or a typo.
#[test]
fn readme_and_book_never_leak_the_x_marker() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let readme = root.join("README.md");
    if let Ok(content) = std::fs::read_to_string(&readme) {
        assert!(
            !content.contains("[X]"),
            "RULE 8: README.md must not contain the literal [X] marker"
        );
    }
    let book_src = root.join("docs/book/src");
    if let Ok(entries) = std::fs::read_dir(&book_src) {
        let mut stack: Vec<std::path::PathBuf> =
            entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
        while let Some(path) = stack.pop() {
            if path.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&path) {
                    stack.extend(entries.filter_map(|e| e.ok().map(|e| e.path())));
                }
            } else if let Ok(content) = std::fs::read_to_string(&path) {
                assert!(
                    !content.contains("[X]"),
                    "RULE 8: {} must not contain the literal [X] marker",
                    path.display()
                );
            }
        }
    }
}
