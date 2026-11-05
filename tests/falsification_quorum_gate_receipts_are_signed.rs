//! PMAT-5882 / R34-3: the quorum gate trusts NO file a runner can write.
//!
//! Root cause: forjar CI jobs run on a pool whose containers mount the receipt store
//! read-write, so any PR code (cargo test, build.rs) could write a receipt the gate
//! accepted. Receipts are now signed (`ssh-keygen -Y sign -n quorum-receipt` over
//! `jq -S -c .`) and the gate verifies against `quorum-allowed-signers`, staged from the
//! PR BASE. Each test below plants a forgery and demands RED; the control demands GREEN.

use std::path::Path;
use std::process::{Command, Output};

#[path = "common/quorum_sign.rs"]
mod quorum_sign;
use quorum_sign as qs;

const SHA: &str = "141132a621af2843efe9d6689b2b469ef34f7ce4";
const OTHER: &str = "acff20f1b5eb1991ab26fb03eb8076d1417d4d2c";

fn body(sha: &str) -> String {
    format!(
        r#"{{"repo":"paiml/forjar","pr":798,"sha":"{sha}","author_model":"claude-sonnet-5-5","verdicts":[{{"lane":"a","model":"claude-opus-5-5","verdict":"PASS","blockers":[]}},{{"lane":"b","model":"gemini-3","verdict":"PASS","blockers":[]}},{{"lane":"c","model":"gpt-5","verdict":"PASS","blockers":[]}}]}}"#
    )
}

/// Run `gate` with `store` as the (test-mode) receipt store.
fn run(gate: &Path, store: &Path) -> Output {
    Command::new("bash")
        .arg(gate)
        .env("QUORUM_STORE", store)
        .env("QUORUM_REPO", "paiml/forjar")
        .env("PR_NUMBER", "798")
        .env(
            "PR_AUTHOR_MODELS",
            "Claude Sonnet 5.5 <noreply@anthropic.com>",
        )
        .env("QUORUM_TEST_MODE", "1")
        .env_remove("GITHUB_ACTIONS")
        .env("PR_HEAD_SHA", SHA)
        .output()
        .unwrap()
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

fn store_dir(root: &Path) -> std::path::PathBuf {
    root.join("store/quorum/paiml/forjar/798")
}

/// (e) control: a properly signed 3-lane receipt passes.
#[test]
fn e_properly_signed_three_lane_receipt_passes() {
    let t = tempfile::tempdir().unwrap();
    qs::write_signed(&store_dir(t.path()), SHA, body(SHA).as_bytes());
    let o = run(&qs::trusted_gate(), &t.path().join("store"));
    assert!(o.status.success(), "{}", text(&o));
}

/// (a) the seat-o1 repro: an UNSIGNED receipt written straight into the store.
#[test]
fn a_unsigned_forged_receipt_fails() {
    let t = tempfile::tempdir().unwrap();
    let d = store_dir(t.path());
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join(format!("{SHA}.json")), body(SHA)).unwrap();
    let o = run(&qs::trusted_gate(), &t.path().join("store"));
    assert!(!o.status.success(), "{}", text(&o));
    assert!(text(&o).contains("unsigned"), "{}", text(&o));
}

/// (a') a garbage signature file is not a signature.
#[test]
fn a_garbage_signature_fails() {
    let t = tempfile::tempdir().unwrap();
    let d = store_dir(t.path());
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join(format!("{SHA}.json")), body(SHA)).unwrap();
    std::fs::write(d.join(format!("{SHA}.json.sig")), "not a signature\n").unwrap();
    let o = run(&qs::trusted_gate(), &t.path().join("store"));
    assert!(!o.status.success(), "{}", text(&o));
}

/// (b) validly signed, but by a key that is NOT in allowed_signers.
#[test]
fn b_valid_signature_by_an_untrusted_key_fails() {
    let t = tempfile::tempdir().unwrap();
    let rogue = qs::new_key();
    let d = store_dir(t.path());
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join(format!("{SHA}.json")), body(SHA)).unwrap();
    std::fs::write(
        d.join(format!("{SHA}.json.sig")),
        qs::sign_with(&rogue, body(SHA).as_bytes()),
    )
    .unwrap();
    let o = run(&qs::trusted_gate(), &t.path().join("store"));
    assert!(!o.status.success(), "{}", text(&o));
    assert!(
        text(&o).contains("signature verification failed"),
        "{}",
        text(&o)
    );
}

/// (c) a valid signed receipt, then ONE byte of the JSON flipped.
#[test]
fn c_one_flipped_byte_fails() {
    let t = tempfile::tempdir().unwrap();
    let d = store_dir(t.path());
    qs::write_signed(&d, SHA, body(SHA).as_bytes());
    // gemini-3 -> gemini-4 (still an allowlisted-looking shape; only the signature can tell)
    let f = d.join(format!("{SHA}.json"));
    let flipped = std::fs::read_to_string(&f)
        .unwrap()
        .replacen("gemini-3", "gemini-4", 1);
    std::fs::write(&f, flipped).unwrap();
    let o = run(&qs::trusted_gate(), &t.path().join("store"));
    assert!(!o.status.success(), "{}", text(&o));
    assert!(
        text(&o).contains("signature verification failed"),
        "{}",
        text(&o)
    );
}

/// (d) a valid signed receipt for ANOTHER sha, copied to the head's filename.
#[test]
fn d_signed_receipt_for_another_sha_fails() {
    let t = tempfile::tempdir().unwrap();
    let d = store_dir(t.path());
    qs::write_signed(&d, OTHER, body(OTHER).as_bytes());
    std::fs::copy(
        d.join(format!("{OTHER}.json")),
        d.join(format!("{SHA}.json")),
    )
    .unwrap();
    std::fs::copy(
        d.join(format!("{OTHER}.json.sig")),
        d.join(format!("{SHA}.json.sig")),
    )
    .unwrap();
    let o = run(&qs::trusted_gate(), &t.path().join("store"));
    assert!(!o.status.success(), "{}", text(&o));
    assert!(text(&o).contains("sha is not the head sha"), "{}", text(&o));
}

/// Same, for another PR number (signed, bound to pr 799, offered for 798).
#[test]
fn d2_signed_receipt_for_another_pr_fails() {
    let t = tempfile::tempdir().unwrap();
    let b = body(SHA).replace("\"pr\":798", "\"pr\":799");
    qs::write_signed(&store_dir(t.path()), SHA, b.as_bytes());
    let o = run(&qs::trusted_gate(), &t.path().join("store"));
    assert!(!o.status.success(), "{}", text(&o));
}

/// The committed allowed_signers is a placeholder that verifies nothing: RED, not a fake key.
#[test]
fn committed_signers_file_is_an_unprovisioned_placeholder_and_gate_is_red() {
    let t = tempfile::tempdir().unwrap();
    let gate = qs::stage_gate(t.path(), None);
    qs::write_signed(&store_dir(t.path()), SHA, body(SHA).as_bytes());
    let o = run(&gate, &t.path().join("store"));
    assert!(!o.status.success(), "{}", text(&o));
    assert!(text(&o).contains("no signer provisioned"), "{}", text(&o));
}

/// A signer file with the right key but another identity is not trusted.
#[test]
fn signer_identity_must_be_quorum_writer() {
    let t = tempfile::tempdir().unwrap();
    let k = qs::trusted();
    let gate = qs::stage_gate(t.path(), Some(k));
    let p = gate.with_file_name("quorum-allowed-signers");
    std::fs::write(&p, k.signers_line.replace("quorum-writer", "someone-else")).unwrap();
    qs::write_signed(&store_dir(t.path()), SHA, body(SHA).as_bytes());
    let o = run(&gate, &t.path().join("store"));
    assert!(!o.status.success(), "{}", text(&o));
}

/// The store path is a test fallback only: in CI (GITHUB_ACTIONS set) or without test mode
/// the gate ignores the store entirely, so a runner-written file cannot satisfy it.
#[test]
fn store_is_ignored_outside_test_mode_even_when_validly_signed() {
    let t = tempfile::tempdir().unwrap();
    qs::write_signed(&store_dir(t.path()), SHA, body(SHA).as_bytes());
    for (k, v) in [("GITHUB_ACTIONS", "true"), ("QUORUM_TEST_MODE", "0")] {
        let o = Command::new("bash")
            .arg(qs::trusted_gate())
            .env("QUORUM_STORE", t.path().join("store"))
            .env("QUORUM_REPO", "paiml/forjar")
            .env("PR_NUMBER", "798")
            .env("PR_BASE_SHA", SHA)
            .env("PR_HEAD_SHA", SHA)
            .env("QUORUM_TEST_MODE", "1")
            .env(k, v)
            .current_dir(t.path())
            .output()
            .unwrap();
        assert!(!o.status.success(), "{k}={v}: {}", text(&o));
    }
}

fn git(dir: &Path, args: &[&str]) {
    let o = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "{args:?} {}",
        String::from_utf8_lossy(&o.stderr)
    );
}

/// Production delivery: a signed receipt on refs/quorum/pr-<N>, no store anywhere.
#[test]
fn signed_receipt_delivered_by_git_ref_passes_and_a_forged_ref_fails() {
    let t = tempfile::tempdir().unwrap();
    let w = t.path().join("work");
    std::fs::create_dir_all(&w).unwrap();
    git(&w, &["init", "-q", "-b", "main"]);
    git(&w, &["commit", "-q", "--allow-empty", "-m", "base"]);
    let base = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&w)
        .output()
        .unwrap();
    let base = String::from_utf8_lossy(&base.stdout).trim().to_string();
    let msg = "w\n\nCo-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>\n";
    git(&w, &["commit", "-q", "--allow-empty", "-m", msg]);
    let head = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&w)
        .output()
        .unwrap();
    let head = String::from_utf8_lossy(&head.stdout).trim().to_string();
    let b = body(&head);
    let go = || {
        Command::new("bash")
            .arg(qs::trusted_gate())
            .env_remove("QUORUM_TEST_MODE")
            .env_remove("QUORUM_STORE")
            .env_remove("PR_AUTHOR_MODELS")
            .env("GITHUB_ACTIONS", "true")
            .env("QUORUM_REPO", "paiml/forjar")
            .env("PR_NUMBER", "798")
            .env("PR_BASE_SHA", &base)
            .env("PR_HEAD_SHA", &head)
            .current_dir(&w)
            .output()
            .unwrap()
    };
    let none = go();
    assert!(!none.status.success(), "no ref yet: {}", text(&none));
    qs::publish_ref(&w, 798, &head, b.as_bytes(), &qs::sign(b.as_bytes()));
    let ok = go();
    assert!(ok.status.success(), "{}", text(&ok));
    // forged ref: same body, signature from a rogue key
    let rogue = qs::new_key();
    qs::publish_ref(
        &w,
        798,
        &head,
        b.as_bytes(),
        &qs::sign_with(&rogue, b.as_bytes()),
    );
    let bad = go();
    assert!(!bad.status.success(), "{}", text(&bad));
    // forged ref: unsigned (empty signature)
    qs::publish_ref(&w, 798, &head, b.as_bytes(), b"");
    assert!(!go().status.success());
    // author comes from the commits: a head with no trailer fails even with a valid signature
    git(
        &w,
        &[
            "commit",
            "-q",
            "--amend",
            "--allow-empty",
            "-m",
            "no trailer",
        ],
    );
    let h2 = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&w)
        .output()
        .unwrap();
    let h2 = String::from_utf8_lossy(&h2.stdout).trim().to_string();
    let b2 = body(&h2);
    qs::publish_ref(&w, 798, &h2, b2.as_bytes(), &qs::sign(b2.as_bytes()));
    let o = Command::new("bash")
        .arg(qs::trusted_gate())
        .env("GITHUB_ACTIONS", "true")
        .env("QUORUM_REPO", "paiml/forjar")
        .env("PR_NUMBER", "798")
        .env("PR_BASE_SHA", &base)
        .env("PR_HEAD_SHA", &h2)
        .current_dir(&w)
        .output()
        .unwrap();
    assert!(!o.status.success(), "{}", text(&o));
}

/// Mutation: with the verification line deleted, the (a) forgery goes GREEN, i.e. the
/// tests above are what stands between a runner-written file and a merge.
#[test]
fn mutation_gate_without_verification_accepts_the_unsigned_forgery() {
    let t = tempfile::tempdir().unwrap();
    let gate = qs::stage_gate(t.path(), Some(qs::trusted()));
    let src = std::fs::read_to_string(&gate).unwrap();
    let mutated: String = src
        .lines()
        .filter(|l| !l.contains("ssh-keygen -Y verify"))
        .map(|l| format!("{l}\n"))
        .collect();
    // removing the verify line leaves `|| fail "signature verification failed"` dangling
    // onto the previous command; neutralise just that continuation
    let mutated = mutated.replace(
        "  || fail \"signature verification failed for head $head (unsigned, wrong signer, or altered receipt)\"\n",
        "",
    );
    assert_ne!(src, mutated);
    std::fs::write(&gate, mutated).unwrap();
    let d = store_dir(t.path());
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join(format!("{SHA}.json")), body(SHA)).unwrap();
    std::fs::write(d.join(format!("{SHA}.json.sig")), "forged\n").unwrap();
    let o = run(&gate, &t.path().join("store"));
    assert!(
        o.status.success(),
        "mutant should accept the forgery: {}",
        text(&o)
    );
}
