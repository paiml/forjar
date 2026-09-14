//! PMAT-531: a ratchet ceiling may not RISE without a written reason.
//!
//! `"MAY ONLY SHRINK"` is the rule `scripts/ratchets/cb21xx-baseline.json`
//! states in its own text, and nothing enforced it. The ceilings are a JSON
//! file: raising one is a one-character edit that turns every future regression
//! green. The schema has carried a `justification` field since it was written
//! and nothing read it.
//!
//! Measured the day it mattered. Correcting PMAT-240's false
//! `status: completed` — thirteen pipelines of its own shape were still in
//! `scripts/` and neither acceptance criterion had been met — turned a closed
//! roadmap row into an OPEN one, CB-2112 and CB-2114 each grew by one, and gate
//! B went red. Correctly: the growth was real. CB-2114 was fixed by real work;
//! CB-2112 was RAISED, because the row keeps its historical id and no sync can
//! rename it.
//!
//! That raise is legitimate, and **it is exactly the shape an illegitimate one
//! has**. The only thing separating them is a sentence, so the gate requires
//! the sentence — measured against the baseline as the BASE BRANCH has it,
//! which is a tree the author of a raise did not write.

#![cfg(unix)]

use std::path::PathBuf;
use std::process::Command;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// A ceiling may not RISE without a written reason (PMAT-531).
///
/// "MAY ONLY SHRINK" is the rule the baseline states and nothing enforced it:
/// the ceilings are a JSON file, and raising one is a one-character edit that
/// turns every future regression green. The schema has carried a
/// `justification` field since it was written; nothing read it.
///
/// Measured the day it mattered. Correcting PMAT-240's false
/// `status: completed` turned a closed roadmap row into an open one, CB-2112
/// and CB-2114 each grew by one, and gate B went red — correctly, because the
/// growth was real. CB-2114 was fixed by real work; CB-2112 was RAISED, because
/// the row keeps its historical id and no sync can rename it. That raise is
/// legitimate, and it is exactly the shape an illegitimate one has.
///
/// So the arm compares against the baseline as the BASE BRANCH has it — a tree
/// the author of a raise did not write — and this drives all four outcomes over
/// a temp repository: raised with a reason, raised without one, lowered, and a
/// baseline the base does not carry at all.
#[test]
fn a_ceiling_may_not_rise_without_a_written_reason() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .args(args)
            .current_dir(root)
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .output()
            .expect("git must run");
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    git(&["init", "-q", "-b", "main"]);
    git(&["config", "commit.gpgsign", "false"]);

    let base = r#"{"ceiling": {"CB-2112": 34, "CB-2114": 34}}"#;
    std::fs::create_dir_all(root.join("scripts/ratchets")).expect("mkdir");
    let rel = "scripts/ratchets/cb21xx-baseline.json";
    std::fs::write(root.join(rel), base).expect("write");
    git(&["add", "-A"]);
    git(&["commit", "-qm", "the ceiling as the base has it"]);

    // The arm, lifted out of the gate the same way the gate runs it.
    let arm = |body: &str| -> (i32, String) {
        std::fs::write(root.join(rel), body).expect("write");
        let script = format!(
            r#"set -euo pipefail
               src={src}
               arm="$(sed -n '/^cb21xx_raise_rc=0$/,/cb21xx_raise_rc=\$?$/p' "$src")"
               [ -n "$arm" ] || {{ echo "could not extract the raise arm from $src"; exit 98; }}
               CB21XX_BASE="{rel}"
               BASE_REF=main
               cb21xx_raise_rc=0
               eval "$arm"
               printf '%s
' "${{cb21xx_raise:-}}"
               exit "${{cb21xx_raise_rc}}""#,
            src = repo().join("scripts/dogfood/comply.sh").display(),
            rel = rel,
        );
        let out = Command::new("bash")
            .arg("-c")
            .arg(&script)
            .current_dir(root)
            .output()
            .expect("bash must run");
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        )
    };

    // Raised, with no reason: refused, naming the check and both numbers.
    let (code, text) = arm(r#"{"ceiling": {"CB-2112": 35, "CB-2114": 34}}"#);
    assert_eq!(
        code, 1,
        "an unjustified raise must be refused:
{text}"
    );
    assert!(
        text.contains("CB-2112 raised 34 -> 35") && text.contains("no justification.CB-2112"),
        "the refusal does not name the check and the numbers:
{text}"
    );

    // Raised, with a reason: allowed, and SAID so — a silent allow would make
    // the reason decorative.
    let (code, text) = arm(r#"{"ceiling": {"CB-2112": 35, "CB-2114": 34},
            "justification": {"CB-2112": "a false completed was corrected, so a closed row became an open one"}}"#);
    assert_eq!(
        code, 0,
        "a justified raise must pass:
{text}"
    );
    assert!(
        text.contains("raised with a written reason") && text.contains("CB-2112"),
        "a justified raise passes silently, which makes the reason decorative:
{text}"
    );

    // An EMPTY reason is not a reason.
    let (code, text) = arm(r#"{"ceiling": {"CB-2112": 35}, "justification": {"CB-2112": "   "}}"#);
    assert_eq!(
        code, 1,
        "whitespace is not a justification:
{text}"
    );

    // Lowering needs nothing, which is the whole point of a ratchet.
    let (code, text) = arm(r#"{"ceiling": {"CB-2112": 33, "CB-2114": 34}}"#);
    assert_eq!(
        code, 0,
        "lowering must never need a reason:
{text}"
    );
    assert!(
        text.contains("no ceiling raised"),
        "a lowering run does not say that nothing rose:
{text}"
    );

    // A justification for a check that did NOT rise does not license a
    // different one that did.
    let (code, text) = arm(r#"{"ceiling": {"CB-2112": 35, "CB-2114": 35},
            "justification": {"CB-2114": "a reason for the other one"}}"#);
    assert_eq!(
        code, 1,
        "a reason is per-check:
{text}"
    );
    assert!(
        text.contains("CB-2112 raised"),
        "the refusal does not name the check that rose without its own reason:
{text}"
    );
}
