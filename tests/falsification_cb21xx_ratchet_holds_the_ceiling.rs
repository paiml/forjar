//! PMAT-243: the ratchet over the five comply checks pmat 3.40 added.
//!
//! # Why this exists
//!
//! Gate B — a release blocker — was GREEN when 1.28.0 was cut on 2026-09-10 and
//! RED on main on 2026-09-11, because pmat 3.40 put six checks in the comply
//! roster that this repository has never satisfied. Not one is a regression
//! from any ticket in that window: they are a new obligation arriving with a
//! tool upgrade, over a backlog that was always there.
//!
//! The doctrine forbids a skip. It does not forbid RECORDING what is true and
//! refusing to let it get worse, which is what CB-200 has done one arm above
//! since PMAT-201. So `scripts/ratchets/cb21xx-baseline.json` records a ceiling
//! per check and `scripts/dogfood/comply.sh` Arm 7 enforces it.
//!
//! # What a ratchet has to get right, and what this pins
//!
//! A ceiling that only ever looks upward greets a rotted predicate as
//! perfection. So the three directions are driven here, over synthetic comply
//! output rather than the live tool: one more finding than the ceiling is a
//! REGRESSION; a check that has left the roster is UNMEASURED and red, not zero
//! findings; and a check whose message carries no count is UNMEASURED too. The
//! fourth direction — fewer findings than the ceiling — is green, because that
//! is the improvement the ratchet exists to let through.
//!
//! Arm 7 is driven by extracting its python from the script, the same way the
//! arm runs it: a test that re-implemented the arithmetic in Rust would pass
//! over an arm that no longer exists.

#![cfg(unix)]

use std::path::PathBuf;
use std::process::Command;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The ids the baseline declares, read from the committed file.
fn declared_ids() -> Vec<String> {
    let body = std::fs::read_to_string(repo().join("scripts/ratchets/cb21xx-baseline.json"))
        .expect("the baseline must be committed");
    let v: serde_json::Value = serde_json::from_str(&body).expect("the baseline must be JSON");
    let mut ids: Vec<String> = v["ceiling"]
        .as_object()
        .expect("ceiling must be an object")
        .keys()
        .cloned()
        .collect();
    ids.sort();
    ids
}

fn ceiling_of(id: &str) -> i64 {
    let body = std::fs::read_to_string(repo().join("scripts/ratchets/cb21xx-baseline.json"))
        .expect("the baseline must be committed");
    let v: serde_json::Value = serde_json::from_str(&body).expect("the baseline must be JSON");
    v["ceiling"][id].as_i64().expect("a ceiling is an integer")
}

/// Comply output naming every declared check with the given finding counts.
fn comply_json(counts: &[(String, Option<i64>)]) -> String {
    let checks: Vec<String> = counts
        .iter()
        .map(|(id, n)| match n {
            Some(n) => format!(
                r#"{{"name":"{id}: Fixture Check","status":"Fail","message":"{n} finding(s) — SOMETHING {n}: a sample line","severity":"Error"}}"#
            ),
            None => format!(
                r#"{{"name":"{id}: Fixture Check","status":"Fail","message":"something went wrong and no count was printed","severity":"Error"}}"#
            ),
        })
        .collect();
    format!(r#"{{"checks":[{}]}}"#, checks.join(","))
}

/// Run Arm 7 over `json`, returning (the arm's exit code, the message it left
/// in `$cb21xx`).
///
/// Everything the arm needs is taken OUT OF THE SCRIPT — the `CB21XX` array,
/// `CB21XX_BASE` and the arm itself — so a renamed baseline, a changed
/// exemption list or a moved arm is a failure here rather than a test quietly
/// measuring its own copy. A review lane refuted the first version of this
/// harness for defining `CB21XX_BASE` itself.
///
/// The message is `$cb21xx` ALONE, not stdout plus stderr. That is the whole
/// point of the arm's `2>&1`: three lanes found that a `sys.exit("…")` writes
/// to stderr while `$( )` takes stdout, so the gate went red with an empty
/// detail. A harness that merged the two streams would have hidden exactly
/// that, and the first one did.
fn arm7(json: &str) -> (i32, String) {
    let script = r#"
set -euo pipefail
src=scripts/dogfood/comply.sh
decl="$(sed -n '/^CB21XX=(/,/^)$/p' "$src")"
[ -n "$decl" ] || { echo "could not extract the CB21XX array from $src"; exit 98; }
base="$(sed -n 's/^CB21XX_BASE=//p' "$src")"
[ -n "$base" ] || { echo "could not extract CB21XX_BASE from $src"; exit 98; }
arm="$(sed -n '/^cb21xx_rc=0$/,/2>&1)" || cb21xx_rc=\$?$/p' "$src")"
[ -n "$arm" ] || { echo "could not extract Arm 7 from $src"; exit 99; }
eval "$decl"
eval "CB21XX_BASE=$base"
comply_json="$(cat)"
eval "$arm"
printf '%s
' "${cb21xx:-}"
exit "${cb21xx_rc}"
"#;
    let mut child = Command::new("bash")
        .arg("-c")
        .arg(script)
        .current_dir(repo())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("bash must run");
    {
        use std::io::Write;
        child
            .stdin
            .as_mut()
            .expect("stdin")
            .write_all(json.as_bytes())
            .expect("write");
    }
    let out = child.wait_with_output().expect("wait");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

/// Whatever the arm decides, it SAYS it — on stdout, where the gate reads it.
///
/// Refuted by all three lanes of the round: `sys.exit("message")` writes to
/// stderr, `cb21xx="$( … )"` captures stdout, and `fail "… ${cb21xx}"` then
/// printed a verdict with nothing after the colon. A gate that is red and mute
/// is worse than one that is green and wrong, because nobody can act on it.
#[test]
fn every_verdict_the_arm_reaches_arrives_with_words() {
    let ids = declared_ids();
    let at: Vec<(String, Option<i64>)> = ids
        .iter()
        .map(|id| (id.clone(), Some(ceiling_of(id))))
        .collect();
    let over: Vec<(String, Option<i64>)> = ids
        .iter()
        .enumerate()
        .map(|(n, id)| {
            (
                id.clone(),
                Some(ceiling_of(id) + if n == 0 { 1 } else { 0 }),
            )
        })
        .collect();
    let cases: Vec<(&str, String)> = vec![
        ("at the ceiling", comply_json(&at)),
        ("over the ceiling", comply_json(&over)),
        ("not JSON at all", "this is not json".to_string()),
        ("an empty roster", r#"{"checks":[]}"#.to_string()),
    ];
    for (what, body) in cases {
        let (_, text) = arm7(&body);
        assert!(
            !text.trim().is_empty(),
            "with {what} the arm left nothing in $cb21xx, so the gate's verdict would              read `the CB-2110..CB-2115 ratchet (Arm 7): ` and stop"
        );
    }
}

/// A check the gate exempts but the baseline does not own is WAIVED by both.
///
/// Refuted by a review lane, and it is the worst outcome this design has: Arm 1
/// skips it because the array names it, Arm 7 skips it because the ceiling map
/// does not, and two files that each look correct alone add up to a check
/// nobody measures. The arm now refuses the mismatch itself rather than leaving
/// it to a test that runs only in CI.
#[test]
fn a_check_exempted_but_not_ratcheted_is_refused_by_the_gate_itself() {
    let script = r#"
set -euo pipefail
src=scripts/dogfood/comply.sh
arm="$(sed -n '/^cb21xx_rc=0$/,/2>&1)" || cb21xx_rc=\$?$/p' "$src")"
[ -n "$arm" ] || { echo "could not extract Arm 7"; exit 99; }
# One more id in the exemption list than the baseline owns.
CB21XX=(CB-2110 CB-2111 CB-2112 CB-2114 CB-2115 CB-9999)
CB21XX_BASE="scripts/ratchets/cb21xx-baseline.json"
comply_json="$(cat)"
eval "$arm"
printf '%s
' "${cb21xx:-}"
exit "${cb21xx_rc}"
"#;
    let ids = declared_ids();
    let counts: Vec<(String, Option<i64>)> = ids
        .iter()
        .map(|id| (id.clone(), Some(ceiling_of(id))))
        .collect();
    let mut child = Command::new("bash")
        .arg("-c")
        .arg(script)
        .current_dir(repo())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("bash must run");
    {
        use std::io::Write;
        child
            .stdin
            .as_mut()
            .expect("stdin")
            .write_all(comply_json(&counts).as_bytes())
            .expect("write");
    }
    let out = child.wait_with_output().expect("wait");
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert_eq!(
        out.status.code().unwrap_or(-1),
        2,
        "an exemption the baseline does not own must be UNMEASURED:\n{text}"
    );
    assert!(
        text.contains("UNMEASURED") && text.contains("CB-9999"),
        "the verdict does not name the check that is exempt and unowned:\n{text}"
    );
}

/// At the ceiling exactly, the ratchet holds and prints what it measured.
/// At the ceiling exactly, the ratchet holds and prints what it measured.
#[test]
fn a_tree_at_the_ceiling_is_green_and_says_every_number() {
    let counts: Vec<(String, Option<i64>)> = declared_ids()
        .into_iter()
        .map(|id| {
            let c = ceiling_of(&id);
            (id, Some(c))
        })
        .collect();
    let (code, text) = arm7(&comply_json(&counts));
    assert_eq!(code, 0, "at the ceiling the ratchet holds:\n{text}");
    for id in declared_ids() {
        let c = ceiling_of(&id);
        assert!(
            text.contains(&format!("{id}={c}/{c}")),
            "the verdict does not report {id}, so a reader cannot see what held:\n{text}"
        );
    }
}

/// One finding more than the ceiling is a regression, named.
#[test]
fn one_finding_over_the_ceiling_is_a_regression() {
    let ids = declared_ids();
    for over in &ids {
        let counts: Vec<(String, Option<i64>)> = ids
            .iter()
            .map(|id| {
                let c = ceiling_of(id);
                (id.clone(), Some(if id == over { c + 1 } else { c }))
            })
            .collect();
        let (code, text) = arm7(&comply_json(&counts));
        assert_eq!(code, 1, "{over} one over its ceiling must be red:\n{text}");
        assert!(
            text.contains("REGRESSION") && text.contains(over.as_str()),
            "the verdict does not name {over} as the regression:\n{text}"
        );
    }
}

/// Below the ceiling is green — the improvement the ratchet exists to allow.
#[test]
fn fewer_findings_than_the_ceiling_is_green() {
    let counts: Vec<(String, Option<i64>)> = declared_ids()
        .into_iter()
        .map(|id| {
            let c = ceiling_of(&id);
            (id, Some(c - 1))
        })
        .collect();
    let (code, text) = arm7(&comply_json(&counts));
    assert_eq!(code, 0, "an improvement must pass:\n{text}");
}

/// A check that has left the roster reports NOTHING, which is spelled the same
/// as zero findings. It must be UNMEASURED and red, never the largest
/// improvement in the project's history.
#[test]
fn a_check_that_left_the_roster_is_unmeasured_and_never_zero() {
    let ids = declared_ids();
    let gone = ids[0].clone();
    let counts: Vec<(String, Option<i64>)> = ids
        .iter()
        .filter(|id| **id != gone)
        .map(|id| (id.clone(), Some(ceiling_of(id))))
        .collect();
    let (code, text) = arm7(&comply_json(&counts));
    assert_eq!(code, 2, "a missing check must be UNMEASURED:\n{text}");
    assert!(
        text.contains("UNMEASURED") && text.contains(gone.as_str()),
        "the verdict does not name {gone} as the one it could not measure:\n{text}"
    );
}

/// A check whose message carries no count is UNMEASURED too — the count is the
/// measurement, and a check reporting Fail with no number has not been read.
#[test]
fn a_check_with_no_finding_count_is_unmeasured() {
    let ids = declared_ids();
    let mute = ids[1].clone();
    let counts: Vec<(String, Option<i64>)> = ids
        .iter()
        .map(|id| {
            if *id == mute {
                (id.clone(), None)
            } else {
                (id.clone(), Some(ceiling_of(id)))
            }
        })
        .collect();
    let (code, text) = arm7(&comply_json(&counts));
    assert_eq!(code, 2, "a Fail with no count must be UNMEASURED:\n{text}");
    assert!(
        text.contains("UNMEASURED") && text.contains(mute.as_str()),
        "the verdict does not name {mute}:\n{text}"
    );
}

/// Arm 1's exemption list is the baseline's list, exactly.
///
/// The two are written in different files and a sixth check in the same family
/// must fail Arm 1 rather than inherit an exemption nobody granted it. If the
/// script's `CB21XX` array and the baseline's `ceiling` keys ever disagree, one
/// of them is silently wrong.
#[test]
fn the_exemption_list_and_the_ceilings_are_the_same_set() {
    let script = std::fs::read_to_string(repo().join("scripts/dogfood/comply.sh"))
        .expect("the gate must exist");
    let start = script
        .find("CB21XX=(")
        .expect("the gate must declare the exemption list");
    let end = script[start..]
        .find(")\n")
        .expect("the exemption list must close");
    let block = &script[start..start + end];
    let mut listed: Vec<String> = block
        .lines()
        .filter_map(|l| {
            let t = l.trim().trim_matches('"');
            if t.starts_with("CB-") {
                Some(t.to_string())
            } else {
                None
            }
        })
        .collect();
    listed.sort();
    assert_eq!(
        listed,
        declared_ids(),
        "scripts/dogfood/comply.sh exempts {listed:?} and the baseline records ceilings for {:?} \
         — a check in one list and not the other is either unowned or unmeasured",
        declared_ids()
    );
}

/// CB-2113 is neither exempt nor ratcheted, and that is deliberate.
///
/// It is branch-local: it reads the commits between the base and HEAD, so every
/// branch can satisfy it and this one does. Exempting a satisfiable check would
/// be the reduction this whole arm exists to avoid.
#[test]
fn commit_traceability_is_not_exempt() {
    let script = std::fs::read_to_string(repo().join("scripts/dogfood/comply.sh"))
        .expect("the gate must exist");
    let start = script.find("CB21XX=(").expect("the exemption list");
    let end = script[start..].find(")\n").expect("the list closes");
    assert!(
        !script[start..start + end].contains("CB-2113"),
        "CB-2113 is branch-local and satisfiable; exempting it would waive a check this \
         repository can pass"
    );
    assert!(
        !declared_ids().iter().any(|id| id == "CB-2113"),
        "CB-2113 has no ceiling: it is satisfied, not ratcheted"
    );
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
