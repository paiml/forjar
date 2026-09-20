//! forjar#573: the quorum receipt's `diff_sha256` must not depend on how many
//! objects the clone it was computed in happens to contain.
//!
//! # The defect
//!
//! `scripts/quorum-gate.sh` binds a receipt to the diff it adjudicated by
//! hashing `git diff <merge-base> <head>`. Every file in that output carries an
//! `index <a>..<b> <mode>` line, and git ABBREVIATES those blob hashes to
//! whatever length keeps them unambiguous IN THE CURRENT REPOSITORY. A clone
//! with more objects needs more characters. So the same commit range, with
//! byte-identical content on both sides, renders as different TEXT in two
//! checkouts, and the binding hash differs with it.
//!
//! Measured on PR #593 (head db49991e, base 537252d3): this workstation's clone
//! held 42982 objects and abbreviated to 8, producing 3617ae6b...; the
//! clean-room runner abbreviated to 9 and produced 3d09e341.... Setting
//! `core.abbrev=9` locally reproduced the runner's hash exactly.
//!
//! That is the whole of forjar#573, filed as "reports STALE for a correct
//! receipt on some runners; the same head passes on rerun". It is NOT flaky. It
//! is per-clone deterministic, and a rerun only appears to fix it when the job
//! lands on a runner whose object count agrees with the author's — which is
//! precisely the shape that makes a bug look like infrastructure.
//!
//! # The fix, and why this test is not a spelling check
//!
//! `--full-index` prints all 40 characters, so the text no longer depends on the
//! repository. The first assertion below MEASURES that: it hashes the same
//! commit range under two different `core.abbrev` settings and requires one
//! answer. The second assertion is the ANTI-VACUITY arm — it requires that
//! WITHOUT `--full-index` those same two settings DISAGREE, so a test that could
//! never have caught the bug fails instead of passing quietly.
//!
//! # mutation
//!
//! Remove `--full-index` from the `diff_text=` assignment in
//! scripts/quorum-gate.sh and `abbrev_independence_is_what_makes_the_binding_portable`
//! goes RED.

use std::process::Command;

fn repo_root() -> &'static str {
    env!("CARGO_MANIFEST_DIR")
}

/// `git diff` over a fixed range at a chosen abbreviation, hashed the way the
/// gate hashes it.
fn diff_hash(abbrev: &str, full_index: bool) -> Option<String> {
    let mut args: Vec<String> = vec!["-c".into(), format!("core.abbrev={abbrev}"), "diff".into()];
    if full_index {
        args.push("--full-index".into());
    }
    // A range every clone of this repository has: the two commits are ancestors
    // of main, so this does not depend on the branch the test runs from.
    args.push("HEAD~1".into());
    args.push("HEAD".into());

    let out = Command::new("git")
        .current_dir(repo_root())
        .args(&args)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    if out.stdout.is_empty() {
        return None;
    }
    let hashed = Command::new("git")
        .current_dir(repo_root())
        .args(["hash-object", "--stdin"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .ok()
        .and_then(|mut c| {
            use std::io::Write;
            c.stdin.as_mut()?.write_all(&out.stdout).ok()?;
            c.wait_with_output().ok()
        })?;
    Some(String::from_utf8_lossy(&hashed.stdout).trim().to_string())
}

#[test]
fn abbrev_independence_is_what_makes_the_binding_portable() {
    let gate = std::fs::read_to_string(format!("{}/scripts/quorum-gate.sh", repo_root()))
        .expect("scripts/quorum-gate.sh must exist: it is the gate this test is about");

    // The gate's own diff must ask for full index lines.
    let line = gate
        .lines()
        .find(|l| l.trim_start().starts_with("diff_text="))
        .expect("quorum-gate.sh must assign diff_text -- the binding is computed there");
    assert!(
        line.contains("--full-index"),
        "quorum-gate.sh computes its binding hash WITHOUT --full-index, so the hash depends on \
         the object count of whichever clone ran it (forjar#573). The line was: {line}"
    );

    // And the behaviour that matters: two abbreviation settings, one answer.
    let (Some(short), Some(long)) = (diff_hash("7", true), diff_hash("32", true)) else {
        // No git, a shallow checkout with no HEAD~1, or an empty diff. UNMEASURED
        // is not a pass, but it is also not this test's failure to report.
        eprintln!(
            "SKIP: could not compute a diff over HEAD~1..HEAD -- no git, or a shallow checkout"
        );
        return;
    };
    assert_eq!(
        short, long,
        "with --full-index the diff text must not depend on core.abbrev, and it did: \
         abbrev=7 gave {short}, abbrev=32 gave {long}"
    );
}

#[test]
fn without_full_index_the_two_settings_disagree() {
    // ANTI-VACUITY. If this fails, the assertion above proves nothing, because
    // the bug it guards against could not be observed here in the first place.
    let (Some(short), Some(long)) = (diff_hash("7", false), diff_hash("32", false)) else {
        eprintln!("SKIP: could not compute a diff over HEAD~1..HEAD");
        return;
    };
    assert_ne!(
        short, long,
        "abbreviated index lines were expected to differ between core.abbrev=7 and 32, and did \
         not. Either this git no longer abbreviates index lines, or the range has no file whose \
         blob hashes differ in the first 7 characters -- in both cases the sibling test is \
         vacuous and must not be trusted as written."
    );
}
