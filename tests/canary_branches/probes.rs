//! The gate's jq, python3 and NUL-byte branches, and the mutant sweep (the
//! helpers live in the parent, `falsification_quorum_gate_canary_branches.rs`).
use super::mutants::{MUTANTS, NO_JQ_MUTANT};
use super::*;
use std::os::unix::fs::PermissionsExt;

/// Base on main, a head that adds the two functions, an evidence commit and a
/// committed receipt; the checkout is then left on main. `edit` may change the
/// receipt; it is handed a commit that is NOT an ancestor of the pushed branch.
fn fixture(refuted: usize, edit: &dyn Fn(&mut Value, &str)) -> (Fix, String) {
    fixture_with(refuted, edit, &|_| {})
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
pub(super) fn jq_shim(needle: &str, code: i32) -> (tempfile::TempDir, String) {
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
pub(super) fn jq_fails(code: i32, gate: &Path) -> (Run, Option<&'static str>) {
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
pub(super) fn python_shim(code: i32) -> (tempfile::TempDir, String) {
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

/// A rename of a tracked source file into `.quorum/` after `reviewed_head` is a
/// deletion of that source; with rename detection the name-only diff shows only
/// the destination, so the gate must read the deletion (`--no-renames`).
pub(super) fn rename_src_into_quorum(root: &Path) {
    if root.join("src/a.rs").exists() && !root.join(".quorum/moved.rs").exists() {
        fx::git(root, &["mv", "src/a.rs", ".quorum/moved.rs"]);
    }
}
