//! #858: seven integration-test binaries ran in no CI job, so they rotted
//! unseen (one failed on main for weeks). Each is a `--test` argument of a
//! `cargo test` step in `.github/workflows/ci.yml`. Dropping one must fail here.
//!
//! The checker reads the parsed workflow, collects every `--test NAME` token of
//! every step `run:`, and names each required target that is absent. It does
//! not read comments: a commented-out `--test` line is not a wiring.
//!
//! mutation: each row of `mutants()` removes one required target from the
//! parsed text and the checker must name exactly that target.

use serde_yaml_ng::Value;
use std::collections::BTreeSet;

const REQUIRED: [&str; 7] = [
    "falsification_390_one_stream_reporter",
    "falsification_release_workflow_shape",
    "falsification_untyped_error_sites_ratchet",
    "falsification_e02_controlmaster_before_the_drift_gate",
    "falsification_replace_running_binary",
    "falsification_io_hash_answers_for_its_machine",
    "falsification_budget_dry_run",
];

fn ci_yml() -> String {
    let p = format!("{}/.github/workflows/ci.yml", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {p}: {e}"))
}

/// A shell line up to its first `#` that starts a comment (line start or after a blank).
fn strip_comment(line: &str) -> &str {
    let b = line.as_bytes();
    for i in 0..b.len() {
        if b[i] == b'#' && (i == 0 || b[i - 1].is_ascii_whitespace()) {
            return &line[..i];
        }
    }
    line
}

/// Every `--test NAME` token in every job step's `run:`.
fn wired(text: &str) -> BTreeSet<String> {
    let doc: Value = serde_yaml_ng::from_str(text).expect("ci.yml parses");
    let mut out = BTreeSet::new();
    let jobs = doc["jobs"].as_mapping().expect("jobs mapping");
    for (_, job) in jobs {
        for step in job["steps"].as_sequence().into_iter().flatten() {
            let Some(run) = step["run"].as_str() else {
                continue;
            };
            let code: Vec<&str> = run.lines().map(strip_comment).collect();
            let toks: Vec<&str> = code.iter().flat_map(|l| l.split_whitespace()).collect();
            for w in toks.windows(2) {
                if w[0] == "--test" {
                    out.insert(w[1].to_string());
                }
            }
        }
    }
    out
}

fn missing(text: &str) -> Vec<&'static str> {
    let have = wired(text);
    REQUIRED
        .iter()
        .copied()
        .filter(|t| !have.contains(*t))
        .collect()
}

/// `text` with every `--test <target>` occurrence deleted (a dropped wiring).
fn drop_target(text: &str, target: &str) -> String {
    text.replace(&format!("--test {target} \\\n"), "")
        .replace(&format!("--test {target}\n"), "\n")
}

#[test]
fn every_858_falsifier_is_wired_into_ci() {
    assert_eq!(missing(&ci_yml()), Vec::<&str>::new(), "not run by ci.yml");
}

#[test]
fn dropping_any_one_target_is_named() {
    let text = ci_yml();
    for t in REQUIRED {
        let mutated = drop_target(&text, t);
        assert_ne!(
            mutated, text,
            "mutant for {t} changed nothing: the probe is vacuous"
        );
        assert_eq!(
            missing(&mutated),
            vec![t],
            "dropping {t} must name exactly {t}"
        );
    }
}

#[test]
fn a_commented_out_target_is_not_wired() {
    let text = ci_yml().replace(
        "--test falsification_budget_dry_run",
        "# --test falsification_budget_dry_run",
    );
    assert!(missing(&text).contains(&"falsification_budget_dry_run"));
}

#[test]
fn every_required_target_exists_on_disk() {
    for t in REQUIRED {
        let p = format!("{}/tests/{t}.rs", env!("CARGO_MANIFEST_DIR"));
        assert!(std::path::Path::new(&p).exists(), "{p} is missing");
    }
}
