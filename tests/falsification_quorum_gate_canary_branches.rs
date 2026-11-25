//! PMAT-1170 (Refs #1170, #990): the PUSH-TIME gate's canary branches, run for real.
//!
//! `scripts/quorum-gate.sh` decides in one python pass whether a receipt that
//! carries `quorum.canary` is a canary round or a waiver in disguise. A source
//! assertion cannot tell a gate that refuses from one that merely contains the
//! right words, so every case here builds a throwaway git repo (base, a head
//! that adds the lines the canaries are falsified by, an evidence commit, a
//! committed receipt) and runs the REAL gate on it, pushed from a checkout that
//! is not the pushed branch (so the gate never shells out to `cargo test`).
//! The mutants copy `scripts/`, change one branch of the copy, and must
//! flip the decision of their kill case (a changed refusal message does not count).
//!
//! Every run pins `QUORUM_ACTOR`: the fixture's `enforce.json` lists only that
//! actor, so a run inheriting the machine's email would fall to `advisory`.
#![cfg(unix)]

#[path = "canary_receipts_fixture/mod.rs"]
mod fx;

use serde_json::{json, Value};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const ACTOR: &str = "tester@example.com";
const BRANCH: &str = "PMAT-1170-gate-fixture";
const TEST_FILE: &str = "tests/falsification_canary_gate_fixture.rs";
const DIGEST: &str = ".quorum/evidence/digest.md";

struct Run {
    code: i32,
    text: String,
}

/// The real gate (or a mutated copy of `scripts/`), run from the fixture.
fn run_gate(repo: &Path, script: &Path, print_hash: bool, tip: &str, path: Option<&str>) -> Run {
    let mut cmd = Command::new("bash");
    cmd.arg(script)
        .args(["--remote-ref", &format!("refs/heads/{BRANCH}")])
        .args(["--local-sha", tip])
        .current_dir(repo)
        .env("QUORUM_ACTOR", ACTOR)
        .env("PRINT_HASH", if print_hash { "1" } else { "0" })
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("QUORUM_SKIP");
    if let Some(p) = path {
        cmd.env("PATH", p);
    }
    let out = cmd.output().expect("the gate script must run");
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    Run {
        code: out.status.code().unwrap_or(-1),
        text,
    }
}

/// One adjudicated claim: a headline over the 180 B item floor and exactly one
/// `- evidence:` subline carrying `cite`, never a round-number length.
fn claim(n: usize, headline: &str, cite: &str) -> String {
    let mut sub = format!(
        "{cite} is the line the panel read at the commit under review rather than \
         trusting the receipt's prose; the added function sits there and nowhere else."
    );
    while [
        200, 300, 400, 500, 600, 800, 1000, 1200, 1400, 1500, 2000, 4000,
    ]
    .contains(&sub.len())
    {
        sub.push('.');
    }
    format!(
        "{n}. [{headline}] The change adds a function to src/a.rs and the panel adjudicated \
         this claim against the pushed tree, which is the only tree a reviewer will see, \
         so the citation below is the whole of its evidence.\n- evidence: {sub}\n\n"
    )
}

fn digest(confirmed: usize, refuted: usize) -> String {
    let mut s = String::from("# Claims digest (fixture)\n\n## CONFIRMED\n\n");
    for i in 0..confirmed {
        s.push_str(&claim(
            i + 1,
            &format!("confirmed-{i}"),
            &format!("src/a.rs:{}", 1 + i % 2),
        ));
    }
    s.push_str("## REFUTED\n\n");
    for i in 0..refuted {
        s.push_str(&claim(i + 1, &format!("refuted-{i}"), "src/a.rs:3"));
    }
    s
}

struct Fix {
    _dir: tempfile::TempDir,
    repo: PathBuf,
}

/// Base on main, a head that adds the two functions, an evidence commit and a
/// committed receipt; the checkout is then left on main. `edit` may change the
/// receipt; it is handed a commit that is NOT an ancestor of the pushed branch.
fn fixture(refuted: usize, edit: &dyn Fn(&mut Value, &str)) -> (Fix, String) {
    fixture_with(refuted, edit, &|_| {})
}

/// `extra` writes files into the receipt commit (e.g. a regenerated `.pmat/`).
fn fixture_with(
    refuted: usize,
    edit: &dyn Fn(&mut Value, &str),
    extra: &dyn Fn(&Path),
) -> (Fix, String) {
    fixture_build(refuted, false, edit, extra)
}

fn fixture_build(
    refuted: usize,
    nul_in_claims: bool,
    edit: &dyn Fn(&mut Value, &str),
    extra: &dyn Fn(&Path),
) -> (Fix, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = dir.path().to_path_buf();
    let root = repo.as_path();
    fx::git(root, &["init", "-q", "-b", "main"]);
    for kv in [
        ("core.hooksPath", "/dev/null"),
        ("commit.gpgsign", "false"),
        ("user.email", ACTOR),
        ("user.name", "tester"),
    ] {
        fx::git(root, &["config", kv.0, kv.1]);
    }
    fx::put(root, "README.md", "fixture\n");
    fx::put(
        root,
        ".quorum/enforce.json",
        &format!("{{\"enforced_for\": [\"{ACTOR}\"]}}\n"),
    );
    fx::put(root, "src/a.rs", "l1\nl2\nl3\n");
    fx::git(root, &["add", "-A"]);
    fx::git(root, &["commit", "-qm", "base"]);
    fx::git(root, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    let base = fx::git(root, &["rev-parse", "HEAD"]);

    fx::git(root, &["checkout", "-q", "-b", "side"]);
    fx::put(root, "side.txt", "not on the pushed branch\n");
    fx::git(root, &["add", "-A"]);
    fx::git(root, &["commit", "-qm", "side"]);
    let side = fx::git(root, &["rev-parse", "HEAD"]);
    fx::git(root, &["checkout", "-q", "main"]);

    fx::git(root, &["checkout", "-q", "-b", BRANCH]);
    fx::put(
        root,
        "src/a.rs",
        "l1\nl2\nl3\nfn added_one() {}\nfn added_two() {}\n",
    );
    fx::put(root, TEST_FILE, "// fixture falsification test\n");
    fx::git(root, &["add", "-A"]);
    fx::git(root, &["commit", "-qm", "head"]);
    let head = fx::git(root, &["rev-parse", "HEAD"]);

    let f = fx::base_fx();
    let mut files: Vec<(String, String, Vec<&str>)> = vec![(
        DIGEST.into(),
        digest(2, refuted),
        vec!["claims", "lanes", "judges", "agy"],
    )];
    for (name, v, role) in [
        ("claims.json", &f.claims, "claims-blind"),
        ("plan.json", &f.plan, "canary-plan"),
        ("verdicts.json", &f.verdicts, "judge-verdicts"),
    ] {
        let mut body = serde_json::to_string_pretty(v).expect("json");
        if nul_in_claims && name == "claims.json" {
            body = body.replacen('[', "[\u{0}", 1);
        }
        files.push((format!(".quorum/evidence/{name}"), body, vec![role]));
    }
    for (p, body, _) in &files {
        fx::put(root, p, body);
    }
    // `extra` runs twice: here, while the receipt file does not exist yet (so
    // its files land in the evidence commit, AFTER reviewed_head and BEFORE the
    // diff hash is taken), and again in the receipt commit.
    extra(root);
    fx::git(root, &["add", "-A"]);
    fx::git(root, &["commit", "-qm", "evidence"]);
    let entries: Vec<Value> = files
        .iter()
        .map(|(p, body, roles)| {
            json!({"path": p, "roles": roles, "blob": fx::git(root, &["rev-parse", &format!("HEAD:{p}")]),
                   "sha256": fx::sha256(body), "bytes": body.len()})
        })
        .collect();
    let total: usize = files.iter().map(|(_, b, _)| b.len()).sum();

    let tip = fx::git(root, &["rev-parse", "HEAD"]);
    let gate = fx::manifest().join("scripts/quorum-gate.sh");
    let hash = run_gate(root, &gate, true, &tip, None);
    assert_eq!(hash.code, 0, "PRINT_HASH must succeed:\n{}", hash.text);

    let mut q = f.receipt["quorum"].clone();
    q["claims_refuted"] = json!(refuted);
    if refuted > 0 {
        q["refuted_claims"] = json!(["the panel killed a claim the diff does not support"]);
    }
    q["canary"]["reviewed_base"] = json!(base);
    q["canary"]["reviewed_head"] = json!(head);
    let mut r = json!({
        "issue": "PMAT-1170 (fixture) canary gate branches",
        "diff_sha256": hash.text.trim(),
        "base_commit": base,
        "quorum": q,
        "crux": {"systems": ["Bugzilla", "Jira", "GitHub triage"], "verdict": "accept"},
        "agy_teamwork": {"ran": true, "verdict": "pass"},
        "pmat": {"tools": ["analyze_vacuous_tests"], "vacuous_tests_in_touched_paths": 0, "accepted": []},
        "falsification": {
            "test": "fixture", "test_file": TEST_FILE, "cargo_test_target": "falsification_canary_gate_fixture",
            "reverted": "the added functions", "observed_failure": "the fixture test went red",
        },
        "evidence": {"claims_digest": DIGEST, "total_bytes": total, "files": entries},
    });
    edit(&mut r, &side);
    fx::put(
        root,
        &format!(".quorum/{BRANCH}.json"),
        &serde_json::to_string_pretty(&r).expect("json"),
    );
    extra(root);
    fx::git(root, &["add", "-A"]);
    fx::git(root, &["commit", "-qm", "quorum receipt"]);
    let tip = fx::git(root, &["rev-parse", "HEAD"]);
    // The checkout is NOT the pushed branch: the gate must not run `cargo test`.
    fx::git(root, &["checkout", "-q", "main"]);
    (Fix { _dir: dir, repo }, tip)
}

type Edit = fn(&mut Value, &str);

/// (name, claims_refuted, receipt edit, extra files in the receipt commit,
/// expected: None = accepted, else a refusal substring)
type Case = (&'static str, usize, Edit, fn(&Path), Option<&'static str>);

fn nothing(_: &Path) {}

const WAIVER_SENTENCE: &str = "quorum.canary is present next to a waiver or override key";

const CASES: [Case; 17] = [
    (
        "refuted_ge_1_with_a_malformed_canary_block_is_refused",
        1,
        |r, _| r["quorum"]["canary"]["planted_by"] = json!("refuter-1"),
        nothing,
        Some("quorum.canary block is refused: quorum.canary.planted_by names a refuter or a judge"),
    ),
    (
        "refuted_0_with_a_valid_canary_block_and_an_empty_waived_reason_is_refused",
        0,
        |r, _| r["waived"] = json!({"reason": ""}),
        nothing,
        Some(WAIVER_SENTENCE),
    ),
    (
        "refuted_0_with_a_valid_canary_block_is_accepted",
        0,
        |_, _| {},
        nothing,
        None,
    ),
    (
        "a_reviewed_head_that_is_not_an_ancestor_of_the_push_is_refused",
        0,
        |r, side| r["quorum"]["canary"]["reviewed_head"] = json!(side),
        nothing,
        Some("quorum.canary.reviewed_head is not an ancestor of the pushed commit"),
    ),
    (
        "a_non_empty_waived_reason_next_to_a_canary_block_is_refused_not_waived",
        0,
        |r, _| r["waived"] = json!({"reason": "because"}),
        nothing,
        Some(WAIVER_SENTENCE),
    ),
    (
        "an_override_key_next_to_a_canary_block_is_refused",
        0,
        |r, _| r["override"] = json!("sneaky"),
        nothing,
        Some(WAIVER_SENTENCE),
    ),
    (
        "an_upper_case_WAIVED_key_next_to_a_canary_block_is_refused",
        0,
        |r, _| r["WAIVED"] = json!({"reason": "because"}),
        nothing,
        Some(WAIVER_SENTENCE),
    ),
    (
        "a_capitalised_Override_key_next_to_a_canary_block_is_refused",
        0,
        |r, _| r["Override"] = json!(true),
        nothing,
        Some(WAIVER_SENTENCE),
    ),
    (
        "a_nested_waiver_key_next_to_a_canary_block_is_refused",
        0,
        |r, _| r["pmat"]["accepted_waivers"] = json!([]),
        nothing,
        Some(WAIVER_SENTENCE),
    ),
    (
        "refutation_waived_false_next_to_a_canary_block_is_refused",
        0,
        |r, _| r["quorum"]["refutation_waived"] = json!(false),
        nothing,
        Some(WAIVER_SENTENCE),
    ),
    (
        "a_reviewed_base_that_is_not_the_merge_base_is_refused",
        0,
        |r, side| r["quorum"]["canary"]["reviewed_base"] = json!(side),
        nothing,
        Some("quorum.canary.reviewed_base is not the merge base of this push"),
    ),
    (
        "a_regenerated_pmat_baseline_after_reviewed_head_is_accepted",
        0,
        |_, _| {},
        |root| fx::put(root, ".pmat/baseline.json", "{\"regenerated\": true}\n"),
        None,
    ),
    (
        "a_canary_whose_falsifier_is_the_receipts_own_evidence_is_refused",
        0,
        |r, _| {
            for (i, ln) in [(0, 1), (1, 2)] {
                let p = format!(".quorum/evidence/claims.json:{ln}");
                r["quorum"]["canary"]["planted"][i]["falsified_by"] = json!(p);
            }
        },
        nothing,
        Some("lies under .quorum/"),
    ),
    (
        "a_canary_receipt_with_a_blank_lane_name_is_refused",
        0,
        |r, _| r["quorum"]["lanes"] = json!(["a", "b", " "]),
        nothing,
        Some("blank or non-string lane name"),
    ),
    (
        "a_code_file_added_after_reviewed_head_is_refused_though_the_diff_hash_covers_it",
        0,
        |_, _| {},
        |root| {
            if !root.join(format!(".quorum/{BRANCH}.json")).exists() {
                fx::put(root, "src/backdoor.rs", "pub fn evil() {}\n");
            }
        },
        Some("change files outside .quorum/ and .pmat/"),
    ),
    (
        "an_unparsable_receipt_is_refused_by_the_json_pass_not_waived",
        0,
        |_, _| {},
        |root| {
            if root.join(format!(".quorum/{BRANCH}.json")).exists() {
                fx::put(root, &format!(".quorum/{BRANCH}.json"), "{ not json");
            }
        },
        Some("not valid JSON"),
    ),
    (
        "a_pack_is_judged_on_its_own_receipt_not_on_the_backfill_receipts_it_carries",
        0,
        |_, _| {},
        |root| {
            // A backfill receipt for an already merged PR: a canary block next
            // to a waiver, which gate E and Arm 5 refuse at the tag. The push
            // gate reads `.quorum/<pushed branch>.json` only.
            fx::put(
                root,
                ".quorum/feat-merged-pr.json",
                "{\"waived\": {\"reason\": \"x\"}, \"quorum\": {\"canary\": {}}}",
            )
        },
        None,
    ),
];

/// Run one case and return the gate's answer and what it was designed to say
/// (None = accepted, else a substring of the refusal).
fn case_run(c: &Case, script: &Path, path: Option<&str>) -> (Run, Option<&'static str>) {
    let (f, tip) = fixture_with(c.1, &c.2, &c.3);
    (run_gate(&f.repo, script, false, &tip, path), c.4)
}

/// True when the answer is the designed one. An acceptance needs the gate's own
/// pass line or its WAIVED line, so a death is never an acceptance.
fn as_designed(r: &Run, want: Option<&str>) -> bool {
    match want {
        None => r.code == 0 && r.text.contains("quorum gate passed"),
        Some(w) => r.code != 0 && r.text.contains(w),
    }
}

/// True when a mutant flipped the decision: an accepted case is now refused, or
/// a refused case is now accepted (exit 0). A mutant that only changes the
/// words of a refusal does not count.
fn flipped(r: &Run, want: Option<&str>) -> bool {
    match want {
        None => r.code != 0,
        Some(_) => r.code == 0,
    }
}

/// None when the case behaves as designed, else what went wrong.
fn verdict(c: &Case, script: &Path, path: Option<&str>) -> Option<String> {
    let (r, want) = case_run(c, script, path);
    if as_designed(&r, want) {
        None
    } else {
        Some(format!(
            "{}: exit {} (wanted {:?}):\n{}",
            c.0, r.code, c.4, r.text
        ))
    }
}

#[test]
fn the_gate_drives_each_canary_branch_end_to_end() {
    let gate = fx::manifest().join("scripts/quorum-gate.sh");
    let bad: Vec<String> = CASES
        .iter()
        .filter_map(|c| verdict(c, &gate, None))
        .collect();
    eprintln!("gate cases {}", CASES.len());
    assert!(
        bad.is_empty(),
        "{} case(s) failed:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

/// `dogfood_canary_files` reads the evidence with `jq ... || return 1`. No
/// RECEIPT can fail that jq: stage 1 answers -1 for an `evidence.files` that is
/// not an array and refuses it, and stage 2a wraps every `.path` read in a
/// `try`, so a malformed entry is refused by NAME before the file read runs.
/// What can fail it is the tool itself, so a PATH shim makes exactly that one
/// jq call exit 7, and the gate must refuse with the UNMEASURED sentence.
#[test]
fn a_failing_jq_in_the_evidence_read_is_refused_not_passed() {
    let real = Command::new("sh")
        .args(["-c", "command -v jq"])
        .output()
        .expect("sh");
    let real = String::from_utf8_lossy(&real.stdout).trim().to_string();
    assert!(
        !real.is_empty(),
        "jq must be installed to run the predicate"
    );
    let bin = tempfile::tempdir().expect("tempdir");
    let shim = bin.path().join("jq");
    std::fs::write(
        &shim,
        format!(
            "#!/bin/sh\ncase \"$*\" in *'.evidence.files[] | select(.path == $p)'*) exit 7;; esac\nexec {real} \"$@\"\n"
        ),
    )
    .expect("write the shim");
    std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let path = format!(
        "{}:{}",
        bin.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let gate = fx::manifest().join("scripts/quorum-gate.sh");
    let (f, tip) = fixture(0, &|_, _| {});
    let r = run_gate(&f.repo, &gate, false, &tip, Some(&path));
    assert!(
        r.code != 0
            && r.text.contains("quorum.canary block is refused")
            && r.text
                .contains("could not be verified against its evidence files"),
        "a jq that fails inside dogfood_canary_files must refuse, not pass: exit {}\n{}",
        r.code,
        r.text
    );
}

/// A PATH whose `jq` exits `code` for every call whose arguments contain
/// `needle` (empty needle: every call) and is the real jq otherwise.
fn jq_shim(needle: &str, code: i32) -> (tempfile::TempDir, String) {
    let real = Command::new("sh")
        .args(["-c", "command -v jq"])
        .output()
        .expect("sh");
    let real = String::from_utf8_lossy(&real.stdout).trim().to_string();
    assert!(!real.is_empty(), "jq must be installed");
    let bin = tempfile::tempdir().expect("tempdir");
    let shim = bin.path().join("jq");
    let body = if needle.is_empty() {
        format!("#!/bin/sh\nexit {code}\n")
    } else {
        format!("#!/bin/sh\ncase \"$*\" in *'{needle}'*) exit {code};; esac\nexec {real} \"$@\"\n")
    };
    std::fs::write(&shim, body).expect("write the shim");
    std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let path = format!(
        "{}:{}",
        bin.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    (bin, path)
}

const BYPASS: Edit = |r, _| r["waived"] = json!({"reason": "bypass"});
const NO_CANARY_WAIVED: Edit = |r, _| {
    r["quorum"]
        .as_object_mut()
        .expect("quorum")
        .remove("canary");
    r["quorum"]["refutation_waived"] = json!("fixture: a waived non-canary round");
    r["waived"] = json!({"reason": "emergency"});
};

fn probe_case(name: &'static str, edit: Edit, want: Option<&'static str>) -> Case {
    (name, 0, edit, nothing, want)
}

/// The waiver check under a jq that fails with `code`: (name of the probe, the
/// answer, what it was designed to be).
fn jq_fails(code: i32, gate: &Path) -> (Run, Option<&'static str>) {
    let (_bin, path) = jq_shim("waiv|override", code);
    let c = probe_case(
        "canary block next to a waiver, jq broken",
        BYPASS,
        Some("could not run (jq exit"),
    );
    case_run(&c, gate, Some(&path))
}

fn non_canary_waiver_without_jq(gate: &Path) -> (Run, Option<&'static str>) {
    let (_bin, path) = jq_shim("", 2);
    let (r, _) = case_run(
        &probe_case(
            "waived non-canary round, no jq at all",
            NO_CANARY_WAIVED,
            None,
        ),
        gate,
        Some(&path),
    );
    // Designed: the WAIVED line and exit 0, exactly as before the canary change.
    let waived = r.code == 0 && r.text.contains("quorum gate WAIVED");
    (
        Run {
            code: if waived { 0 } else { 1 },
            text: r.text,
        },
        None,
    )
}

/// Finding 1 of the third review: `jq -e` exit 0 is a hit (refuse), 1 is no hit,
/// anything else (2 usage or system, 5 parse error, 127 missing) is UNMEASURED.
/// It must never fall through to the WAIVED exit.
#[test]
fn a_jq_that_fails_next_to_a_canary_block_is_unmeasured_never_waived() {
    let gate = fx::manifest().join("scripts/quorum-gate.sh");
    for code in [2, 4, 5, 127] {
        let (r, _) = jq_fails(code, &gate);
        assert!(
            r.code != 0
                && r.text.contains(&format!("(jq exit {code}): UNMEASURED"))
                && !r.text.contains("WAIVED"),
            "jq exit {code} must refuse as UNMEASURED:\n{}",
            r.text
        );
    }
}

#[test]
fn a_push_with_no_canary_block_never_runs_jq_for_the_waiver_check() {
    let gate = fx::manifest().join("scripts/quorum-gate.sh");
    let (r, _) = non_canary_waiver_without_jq(&gate);
    assert!(
        r.code == 0,
        "a non-canary waived receipt must decide as it did on main, with no jq:\n{}",
        r.text
    );
}

/// A python3 that says nothing (or exits `code`) for the canary detection pass
/// and is the real one for every other pass: the detection must die UNMEASURED,
/// never read the silence as "no canary".
fn python_shim(code: i32) -> (tempfile::TempDir, String) {
    let real = Command::new("sh")
        .args(["-c", "command -v python3"])
        .output()
        .expect("sh");
    let real = String::from_utf8_lossy(&real.stdout).trim().to_string();
    assert!(!real.is_empty(), "python3 must be installed");
    let bin = tempfile::tempdir().expect("tempdir");
    let shim = bin.path().join("python3");
    let body = format!(
        "#!/bin/bash\ns=\"$(cat)\"\ncase \"$s\" in *'isinstance(q, dict) and \"canary\" in q'*) exit {code};; esac\nprintf '%s\\n' \"$s\" | exec {real} \"$@\"\n"
    );
    std::fs::write(&shim, body).expect("write the shim");
    std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let path = format!(
        "{}:{}",
        bin.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    (bin, path)
}

#[test]
fn a_python3_that_says_nothing_for_the_canary_detection_is_unmeasured_never_no_canary() {
    let gate = fx::manifest().join("scripts/quorum-gate.sh");
    for code in [0, 1, 2, 127] {
        let (_bin, path) = python_shim(code);
        let (r, _) = case_run(&CASES[0], &gate, Some(&path));
        assert!(
            r.code != 0
                && r.text.contains("the canary detection could not run")
                && r.text.contains("UNMEASURED"),
            "python3 exit {code} must refuse as UNMEASURED:\n{}",
            r.text
        );
    }
}

fn nul_receipt(root: &Path) {
    let p = root.join(format!(".quorum/{BRANCH}.json"));
    if let Ok(t) = std::fs::read_to_string(&p) {
        std::fs::write(&p, t.replacen('{', "{\u{0}", 1)).expect("write");
    }
}

/// A NUL byte in the receipt, or in an evidence file the receipt hashes, is
/// refused by the push gate (the receipt is judged on the bytes git holds).
#[test]
fn a_nul_byte_in_the_receipt_or_a_canary_evidence_file_is_refused_by_the_push_gate() {
    let gate = fx::manifest().join("scripts/quorum-gate.sh");
    let (f0, tip0) = fixture_with(0, &|_, _| {}, &nothing);
    let r0 = run_gate(&f0.repo, &gate, false, &tip0, None);
    assert!(
        r0.code == 0 && r0.text.contains("quorum gate passed"),
        "control fixture without NUL must pass:\n{}",
        r0.text
    );
    for (name, nul_in_claims, extra, want) in [
        (
            "receipt",
            false,
            nul_receipt as fn(&Path),
            "is not valid JSON",
        ),
        (
            "claims.json",
            true,
            nothing as fn(&Path),
            "holds a NUL byte",
        ),
    ] {
        let (f, tip) = fixture_build(0, nul_in_claims, &|_, _| {}, &extra);
        let r = run_gate(&f.repo, &gate, false, &tip, None);
        assert!(
            r.code != 0 && !r.text.contains("quorum gate passed") && r.text.contains(want),
            "a NUL in the {name} must be refused with {want:?}:\n{}",
            r.text
        );
    }
}

#[path = "canary_branches/mutants.rs"]
mod mutants;
use mutants::{MUTANTS, NO_JQ_MUTANT};

fn mutated_gate(from: &str, to: &str) -> (tempfile::TempDir, PathBuf) {
    let copy = tempfile::tempdir().expect("tempdir");
    let ok = Command::new("cp")
        .arg("-r")
        .arg(fx::manifest().join("scripts"))
        .arg(copy.path())
        .status()
        .expect("cp");
    assert!(ok.success(), "cp -r scripts failed");
    let gate = copy.path().join("scripts/quorum-gate.sh");
    let src = std::fs::read_to_string(&gate).expect("read the gate copy");
    assert_eq!(
        src.matches(from).count(),
        1,
        "mutant '{from}' must match once"
    );
    std::fs::write(&gate, src.replace(from, to)).expect("write the mutant");
    (copy, gate)
}

#[test]
fn every_mutant_of_the_gate_flips_its_kill_case() {
    eprintln!("gate mutants {}", MUTANTS.len() + 1);
    let real = fx::manifest().join("scripts/quorum-gate.sh");
    for (name, from, to, probe) in MUTANTS {
        let (_keep, gate) = mutated_gate(from, to);
        let (r, want) = probe(&gate);
        assert!(
            flipped(&r, want),
            "mutant '{name}' survived: the decision did not flip:\n{}",
            r.text
        );
        // Control: the unmutated gate gives the probe its designed answer.
        let (r0, want0) = probe(&real);
        assert!(
            want0.is_none() && r0.code == 0
                || want0.is_some_and(|w| r0.code != 0 && r0.text.contains(w)),
            "the unmutated gate must give the designed answer for '{name}':\n{}",
            r0.text
        );
    }
    let (name, from, to) = NO_JQ_MUTANT;
    let (_keep, gate) = mutated_gate(from, to);
    let (r, _) = non_canary_waiver_without_jq(&gate);
    assert!(r.code != 0, "mutant '{name}' survived:\n{}", r.text);
}
