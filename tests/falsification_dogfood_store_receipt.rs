//! forjar#876 (design note v6, PR A1): `scripts/dogfood/lib/store.sh` is the
//! head-bound store receipt check that gates E and R will AND on top of their
//! committed-file checks. Each case runs the REAL scripts against a throwaway
//! repo: a base commit, a PR head carrying a Co-Authored-By trailer, and a
//! squash-merge commit on top of the base, with a receipt store beside it.
#![cfg(unix)]

#[path = "dogfood_gates_harness/mod.rs"]
mod harness;

use harness::{git, stdout_of, write};
use std::path::PathBuf;
use std::process::Command;

const AUTHOR: &str = "Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>";

struct Repo {
    _dir: tempfile::TempDir,
    root: PathBuf,
    store: PathBuf,
    head: String,
    other: String,
    merge: String,
}

fn src(rel: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} must exist: {e}", p.display()))
}

fn receipt(head: &str, lanes: &str, judges: u32, author: &str) -> String {
    let v = |l: &str, m: &str| {
        format!(r#"{{"lane":"{l}","model":"{m}","verdict":"PASS","blockers":[]}}"#)
    };
    format!(
        r#"{{"repo":"paiml/forjar","pr":77,"sha":"{head}","author_model":"{author}","verdicts":[{},{},{}],"quorum":{{"lanes":{lanes},"judges":{judges},"refuters_per_claim":3,"claims_refuted":2}},"evidence":{{"files":["a.md"]}}}}"#,
        v("a", "claude-opus-5-5"),
        v("b", "claude-haiku-5-5"),
        v("c", "gemini-3")
    )
}

fn good(head: &str) -> String {
    receipt(head, r#"["a","b","c"]"#, 3, "claude-sonnet-5-5")
}

/// `trailer` is the PR head commit's trailer ("" = a commit carrying none).
fn repo(trailer: &str) -> Repo {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("repo");
    std::fs::create_dir_all(&root).expect("mkdir");
    git(&root, &["init", "-q", "-b", "main"]);
    git(&root, &["config", "core.hooksPath", "/dev/null"]);
    git(&root, &["config", "commit.gpgsign", "false"]);
    for rel in [
        "scripts/quorum-store-gate.sh",
        "scripts/quorum-models.json",
        "scripts/dogfood/lib/store.sh",
        "scripts/dogfood/lib/receipt.sh",
    ] {
        write(&root, rel, &src(rel));
    }
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "base"]);
    let base = stdout_of(&git(&root, &["rev-parse", "HEAD"]));
    let msg = format!("pr work\n\n{trailer}");
    git(&root, &["checkout", "-q", "-b", "pr", &base]);
    write(&root, "w1.txt", "one\n");
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", &msg]);
    let other = stdout_of(&git(&root, &["rev-parse", "HEAD"]));
    write(&root, "w2.txt", "two\n");
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", &msg]);
    let head = stdout_of(&git(&root, &["rev-parse", "HEAD"]));
    git(&root, &["checkout", "-q", "main"]);
    write(&root, "w.txt", "squashed\n");
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "squash merge (#77)"]);
    let merge = stdout_of(&git(&root, &["rev-parse", "HEAD"]));
    let store = dir.path().join("store");
    Repo {
        _dir: dir,
        root,
        store,
        head,
        other,
        merge,
    }
}

impl Repo {
    /// Commit `body` at `rel` on main and make that commit the merge under test.
    fn merge_edit(&mut self, rel: &str, body: &str) {
        write(&self.root, rel, body);
        git(&self.root, &["add", "-A"]);
        git(
            &self.root,
            &["commit", "-qm", "squash merge edits the judge (#77)"],
        );
        self.merge = stdout_of(&git(&self.root, &["rev-parse", "HEAD"]));
    }

    fn put(&self, name: &str, body: &str) {
        write(
            &self.store,
            &format!("quorum/paiml/forjar/77/{name}.json"),
            body,
        );
    }

    /// The token and the reason, or `FAIL ...` when the gate died.
    fn check(&self, head: &str, env: &[(&str, &str)]) -> String {
        let script = r#"fail(){ echo "FAIL $1"; exit 1; }; REPO=paiml/forjar
. scripts/dogfood/lib/store.sh; dogfood_store_require
dogfood_store_check 77 "$1" "$2"; echo "token=$DOGFOOD_STORE_TOKEN $DOGFOOD_STORE_WHY""#;
        let mut c = Command::new("bash");
        c.args(["-c", script, "x", head, &self.merge])
            .current_dir(&self.root);
        c.env_remove("QUORUM_STORE").env_remove("GITHUB_ACTIONS");
        if !env.iter().any(|(k, _)| *k == "QUORUM_STORE") {
            c.env("QUORUM_STORE", &self.store);
        }
        for (k, v) in env {
            c.env(k, v);
        }
        let o = c.output().expect("bash");
        format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        )
    }
}

fn tok(out: &str, want: &str) {
    assert!(
        out.starts_with(&format!("token={want}")),
        "want token {want}, got: {out}"
    );
}

#[test]
fn a_head_bound_receipt_at_the_floor_is_ok() {
    let r = repo(AUTHOR);
    r.put(&r.head, &good(&r.head));
    tok(&r.check(&r.head, &[]), "ok");
}

#[test]
fn no_store_receipt_at_the_head_is_refused() {
    let r = repo(AUTHOR);
    tok(&r.check(&r.head, &[]), "missing");
}

#[test]
fn a_receipt_for_another_head_of_the_same_pr_is_refused() {
    let r = repo(AUTHOR);
    r.put(&r.other, &good(&r.other));
    tok(&r.check(&r.head, &[]), "missing");
}

#[test]
fn a_receipt_keyed_by_the_merge_commit_is_refused() {
    let r = repo(AUTHOR);
    r.put(&r.merge, &good(&r.merge));
    tok(&r.check(&r.head, &[]), "missing");
}

#[test]
fn a_symlinked_receipt_is_refused() {
    let r = repo(AUTHOR);
    r.put("real", &good(&r.head));
    let d = r.store.join("quorum/paiml/forjar/77");
    std::os::unix::fs::symlink(d.join("real.json"), d.join(format!("{}.json", r.head))).unwrap();
    tok(&r.check(&r.head, &[]), "missing");
}

#[test]
fn a_receipt_naming_a_different_sha_or_author_is_refused() {
    let r = repo(AUTHOR);
    r.put(&r.head, &good(&r.other));
    tok(&r.check(&r.head, &[]), "missing");
    r.put(&r.head, &receipt(&r.head, r#"["a","b","c"]"#, 3, "gpt-5"));
    tok(&r.check(&r.head, &[]), "missing");
}

#[test]
fn a_store_receipt_under_each_floor_is_refused() {
    let r = repo(AUTHOR);
    r.put(
        &r.head,
        &receipt(&r.head, r#"["a","b"]"#, 3, "claude-sonnet-5-5"),
    );
    tok(&r.check(&r.head, &[]), "thin");
    r.put(
        &r.head,
        &receipt(&r.head, r#"["a","b","c"]"#, 2, "claude-sonnet-5-5"),
    );
    tok(&r.check(&r.head, &[]), "thin");
    r.put(
        &r.head,
        &good(&r.head).replace(r#""files":["a.md"]"#, r#""files":[]"#),
    );
    tok(&r.check(&r.head, &[]), "thin");
    r.put(
        &r.head,
        &good(&r.head).replace(r#""claims_refuted":2"#, r#""claims_refuted":0"#),
    );
    tok(&r.check(&r.head, &[]), "thin");
}

#[test]
fn a_waiver_in_the_store_receipt_is_refused() {
    let r = repo(AUTHOR);
    r.put(
        &r.head,
        &good(&r.head).replace(r#""pr":77"#, r#""pr":77,"waiver":"x""#),
    );
    tok(&r.check(&r.head, &[]), "missing");
}

#[test]
fn an_absent_malformed_or_unknown_head_oid_is_refused_with_the_fetch_command() {
    let r = repo(AUTHOR);
    let upper = r.head.to_uppercase();
    let short = r.head[..7].to_string();
    for bad in [
        "",
        "abc123",
        short.as_str(),
        upper.as_str(),
        &"0".repeat(40),
    ] {
        let out = r.check(bad, &[]);
        tok(&out, "missing");
        assert!(out.contains("git fetch origin"), "{out}");
    }
}

#[test]
fn an_unset_or_empty_store_variable_fails_and_names_it() {
    let r = repo(AUTHOR);
    r.put(&r.head, &good(&r.head));
    let empty = r.check(&r.head, &[("QUORUM_STORE", "")]);
    assert!(
        empty.starts_with("FAIL") && empty.contains("QUORUM_STORE"),
        "{empty}"
    );
    let c = Command::new("bash")
        .args(["-c", "fail(){ echo \"FAIL $1\"; exit 1; }; REPO=x; . scripts/dogfood/lib/store.sh; dogfood_store_require"])
        .current_dir(&r.root)
        .env_remove("QUORUM_STORE")
        .output()
        .unwrap();
    assert_eq!(c.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&c.stdout).contains("QUORUM_STORE"));
}

#[test]
fn a_commit_with_no_author_trailer_is_refused_and_test_overrides_do_not_rescue_it() {
    let r = repo("");
    r.put(&r.head, &good(&r.head));
    tok(&r.check(&r.head, &[]), "missing");
    let spoof = [("QUORUM_TEST_MODE", "1"), ("PR_AUTHOR_MODELS", AUTHOR)];
    tok(&r.check(&r.head, &spoof), "missing");
}

#[test]
fn the_store_gate_is_read_from_the_base_not_the_working_tree() {
    let r = repo(AUTHOR);
    write(
        &r.root,
        "scripts/quorum-store-gate.sh",
        "#!/usr/bin/env bash\nexit 0\n",
    );
    tok(&r.check(&r.head, &[]), "missing");
}

#[test]
fn a_merge_that_adds_its_own_author_to_the_allowlist_is_refused() {
    let mut r = repo("Co-Authored-By: Evil Bot <evil@example.com>");
    let models = src("scripts/quorum-models.json")
        .replace(r#""authors": {"#, r#""authors": {"Evil Bot": "evil-1","#);
    assert!(models.contains("Evil Bot"), "fixture edit must apply");
    r.merge_edit("scripts/quorum-models.json", &models);
    r.put(&r.head, &receipt(&r.head, r#"["a","b","c"]"#, 3, "evil-1"));
    let out = r.check(&r.head, &[]);
    tok(&out, "missing");
    assert!(out.contains("Evil Bot"), "{out}");
}

#[test]
fn a_merge_that_rewrites_the_gate_to_always_pass_is_refused() {
    let mut r = repo(AUTHOR);
    r.merge_edit(
        "scripts/quorum-store-gate.sh",
        "#!/usr/bin/env bash\nexit 0\n",
    );
    tok(&r.check(&r.head, &[]), "missing");
}

#[test]
fn a_base_lacking_the_gate_or_the_allowlist_is_refused() {
    for rel in ["scripts/quorum-store-gate.sh", "scripts/quorum-models.json"] {
        let mut r = repo(AUTHOR);
        r.put(&r.head, &good(&r.head));
        git(&r.root, &["rm", "-q", rel]);
        git(&r.root, &["commit", "-qm", "base without the judge"]);
        r.merge_edit(rel, &src(rel)); // the merge restores it; HEAD has it, merge^1 does not
        let out = r.check(&r.head, &[]);
        tok(&out, "missing");
        assert!(out.contains("not committed at"), "{rel}: {out}");
    }
}

#[test]
fn an_unresolvable_merge_parent_is_refused() {
    let r = repo(AUTHOR);
    r.put(&r.head, &good(&r.head));
    let root = git(&r.root, &["rev-list", "--max-parents=0", "HEAD"]);
    let first = stdout_of(&root);
    let script = r#"fail(){ echo "FAIL $1"; exit 1; }; REPO=paiml/forjar
. scripts/dogfood/lib/store.sh
dogfood_store_check 77 "$1" "$2"; echo "token=$DOGFOOD_STORE_TOKEN $DOGFOOD_STORE_WHY""#;
    let o = Command::new("bash")
        .args(["-c", script, "x", &r.head, &first])
        .current_dir(&r.root)
        .env("QUORUM_STORE", &r.store)
        .output()
        .unwrap();
    tok(&String::from_utf8_lossy(&o.stdout), "missing");
}

#[test]
fn the_check_itself_refuses_an_empty_store_variable_without_the_require_call() {
    let r = repo(AUTHOR);
    let script = r#"REPO=paiml/forjar
. scripts/dogfood/lib/store.sh
dogfood_store_check 77 "$1" "$2"; echo "token=$DOGFOOD_STORE_TOKEN $DOGFOOD_STORE_WHY""#;
    let o = Command::new("bash")
        .args(["-c", script, "x", &r.head, &r.merge])
        .current_dir(&r.root)
        .env("QUORUM_STORE", "")
        .output()
        .unwrap();
    let out = String::from_utf8_lossy(&o.stdout);
    tok(&out, "missing");
    assert!(out.contains("QUORUM_STORE"), "{out}");
}

#[test]
fn a_receipt_under_the_refuters_per_claim_floor_is_refused() {
    let r = repo(AUTHOR);
    r.put(
        &r.head,
        &good(&r.head).replace(r#""refuters_per_claim":3"#, r#""refuters_per_claim":1"#),
    );
    tok(&r.check(&r.head, &[]), "thin");
}
