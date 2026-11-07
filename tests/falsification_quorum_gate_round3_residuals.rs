//! PMAT-554 round 3 falsifiers: judge-from-base, per-commit trailers, strict
//! trailer parse, duplicate keys at any depth, `pr` integer literal.
//!
//! IRREDUCIBLE RESIDUAL (not closed here): Co-Authored-By trailers and receipt
//! contents are written by the author/seat, so author-identity spoofing (omit
//! or replace one's own trailer) needs a harness-stamped attestation.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}
fn gate() -> PathBuf {
    root().join("scripts/quorum-store-gate.sh")
}
const SON: &str = "Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>";

fn lane(n: &str, m: &str) -> String {
    format!(r#"{{"lane":"{n}","model":"{m}","verdict":"PASS","blockers":[]}}"#)
}
fn lanes() -> String {
    [
        lane("a", "claude-opus-5-5"),
        lane("b", "claude-haiku-5-5"),
        lane("c", "gemini-3"),
    ]
    .join(",")
}
fn body(head: &str, top: &str, verdicts: &str) -> String {
    format!(
        r#"{{"repo":"paiml/forjar","pr":874,"sha":"{head}","author_model":"claude-sonnet-5-5"{top},"verdicts":[{verdicts}]}}"#
    )
}

struct Repo {
    dir: tempfile::TempDir,
    base: String,
}
impl Repo {
    fn git(&self, args: &[&str]) -> String {
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
            .current_dir(self.dir.path())
            .output()
            .unwrap();
        assert!(
            o.status.success(),
            "{args:?} {}",
            String::from_utf8_lossy(&o.stderr)
        );
        String::from_utf8_lossy(&o.stdout).trim().to_string()
    }
    fn new() -> Repo {
        let mut r = Repo {
            dir: tempfile::tempdir().unwrap(),
            base: String::new(),
        };
        r.git(&["init", "-q", "-b", "main"]);
        r.git(&["commit", "-q", "--allow-empty", "-m", "base"]);
        r.base = r.git(&["rev-parse", "HEAD"]);
        r
    }
    fn commit(&self, msg: &str) -> String {
        self.git(&["commit", "-q", "--allow-empty", "-m", msg]);
        self.git(&["rev-parse", "HEAD"])
    }
    /// Run the gate on the current head with the given receipt body (sha substituted).
    fn run(&self, receipt: &str, envs: &[(&str, &str)]) -> Output {
        let head = self.git(&["rev-parse", "HEAD"]);
        let store = self.dir.path().join("store");
        let d = store.join("quorum/paiml/forjar/874");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(
            d.join(format!("{head}.json")),
            receipt.replace("HEAD", &head),
        )
        .unwrap();
        let mut c = Command::new("bash");
        c.arg(gate())
            .env_remove("PR_AUTHOR_MODELS")
            .env_remove("QUORUM_TEST_MODE")
            .env_remove("GITHUB_ACTIONS")
            .env("PR_BASE_SHA", &self.base)
            .env("QUORUM_STORE", &store)
            .env("QUORUM_REPO", "paiml/forjar")
            .env("PR_NUMBER", "874")
            .env("PR_HEAD_SHA", &head)
            .current_dir(self.dir.path());
        for (k, v) in envs {
            c.env(k, v);
        }
        c.output().unwrap()
    }
    fn ok(&self, receipt: &str) -> bool {
        self.run(receipt, &[]).status.success()
    }
}
fn good() -> String {
    body("HEAD", "", &lanes())
}
fn msg(extra: &str) -> String {
    format!("work\n\nPmat-Ticket: PMAT-554\n\n{extra}\n")
}

#[test]
fn control_one_commit_with_trailer_passes() {
    let r = Repo::new();
    r.commit(&msg(SON));
    let o = r.run(&good(), &[]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}

#[test]
fn every_commit_needs_a_mapped_trailer() {
    let r = Repo::new();
    r.commit(&msg(SON));
    r.commit("second commit, no trailer");
    assert!(!r.ok(&good()), "commit without trailer must fail");
}

#[test]
fn merge_commits_from_main_are_excluded() {
    let r = Repo::new();
    r.git(&["checkout", "-q", "-b", "pr"]);
    r.commit(&msg(SON));
    r.git(&["checkout", "-q", "main"]);
    r.commit("main moved, no trailer");
    r.git(&["checkout", "-q", "pr"]);
    r.git(&["merge", "-q", "--no-ff", "-m", "Merge main", "main"]);
    // base = original base: main's own commit is in the range but a non-merge w/o trailer.
    // PR_BASE_SHA is the main tip at PR time in practice; model that.
    let tip = r.git(&["rev-parse", "main"]);
    let mut r2 = r;
    r2.base = tip;
    assert!(r2.ok(&good()), "merge commit alone must not need a trailer");
}

#[test]
fn author_env_is_ignored_in_ci_and_without_test_mode() {
    let r = Repo::new();
    r.commit("no trailer here");
    let a = "Claude Sonnet 5.5 <noreply@anthropic.com>";
    // no QUORUM_TEST_MODE: env var is not an author source
    assert!(!r.run(&good(), &[("PR_AUTHOR_MODELS", a)]).status.success());
    // GITHUB_ACTIONS set: ignored even with test mode
    assert!(!r
        .run(
            &good(),
            &[
                ("PR_AUTHOR_MODELS", a),
                ("QUORUM_TEST_MODE", "1"),
                ("GITHUB_ACTIONS", "true")
            ]
        )
        .status
        .success());
}

#[test]
fn trailer_parse_is_anchored_and_single_angle() {
    for t in [
        "Co-Authored-By: Claude Sonnet 5.5 <a@b.c> <d@e.f>",
        "Co-Authored-By: Claude Sonnet 5.5 <a@b.c> trailing",
        "Co-Authored-By: Claude Sonnet 5.5 <<a@b.c>",
        "Co-Authored-By: Claude Sonnet 5.5",
        "Co-Authored-By: Claude Sonnet 5.5 <>",
        "Co-Authored-By: Some Human <a@b.c>",
        "Co-Authored-By: Claude Sonnet 5.5 <a@b.c>\nCo-Authored-By: Some Human <a@b.c>",
    ] {
        let r = Repo::new();
        r.commit(&msg(t));
        assert!(!r.ok(&good()), "{t}");
    }
}

#[test]
fn gemini_author_is_in_the_authors_allowlist() {
    let r = Repo::new();
    r.commit(&msg("Co-Authored-By: gemini-4-argon-high <a@b.c>"));
    let b = good().replace("claude-sonnet-5-5", "gemini-4-argon-high");
    assert!(r.ok(&b), "gemini-4-argon-high must map as an author");
}

#[test]
fn duplicate_keys_at_any_depth_fail() {
    let r = Repo::new();
    r.commit(&msg(SON));
    let fail_lane = r#"{"lane":"a","model":"claude-opus-5-5","verdict":"FAIL","blockers":["x"]}"#;
    let cases = [
        // top-level verdicts twice: FAIL decoy then PASS lanes
        format!(
            r#"{{"repo":"paiml/forjar","pr":874,"sha":"HEAD","author_model":"claude-sonnet-5-5","verdicts":[{fail_lane}],"verdicts":[{}]}}"#,
            lanes()
        ),
        body("HEAD", r#","meta":{"a":1},"meta":{"b":2}"#, &lanes()),
        body("HEAD", r#","meta":[1],"meta":[1,2]"#, &lanes()),
        body("HEAD", r#","meta":{},"meta":{"a":1}"#, &lanes()),
        body(
            "HEAD",
            r#","meta":{"a":{"x":1}},"meta":{"a":{"y":1}}"#,
            &lanes(),
        ),
    ];
    for c in cases {
        assert!(!r.ok(&c), "{c}");
    }
    // control: distinct nested keys are fine
    assert!(r.ok(&body(
        "HEAD",
        r#","meta":{"a":{"x":1},"b":{"y":1}}"#,
        &lanes()
    )));
}

#[test]
fn pr_is_the_exact_top_level_integer_token() {
    let r = Repo::new();
    r.commit(&msg(SON));
    let decoy = format!(
        r#"{{"repo":"paiml/forjar","meta":{{"pr":874}},"pr":874.0,"sha":"HEAD","author_model":"claude-sonnet-5-5","verdicts":[{}]}}"#,
        lanes()
    );
    assert!(!r.ok(&decoy), "nested pr decoy with float top-level pr");
    let nested_only = format!(
        r#"{{"repo":"paiml/forjar","meta":{{"pr":874}},"pr":"874","sha":"HEAD","author_model":"claude-sonnet-5-5","verdicts":[{}]}}"#,
        lanes()
    );
    assert!(!r.ok(&nested_only));
    // pretty-printed, comma on the next line, must PASS
    let pretty = format!(
        "{{\n  \"repo\": \"paiml/forjar\",\n  \"pr\": 874\n  ,\n  \"sha\": \"HEAD\",\n  \"author_model\": \"claude-sonnet-5-5\",\n  \"verdicts\": [{}]\n}}\n",
        lanes()
    );
    let o = r.run(&pretty, &[]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}

#[test]
fn workflow_judges_from_the_pr_base_not_the_pr_copy() {
    let wf = std::fs::read_to_string(root().join(".github/workflows/quorum.yml")).unwrap();
    assert!(wf.contains(r#"git show "$PR_BASE_SHA:scripts/quorum-store-gate.sh""#));
    assert!(wf.contains(r#"git show "$PR_BASE_SHA:scripts/quorum-models.json""#));
    assert!(!wf.contains("run: |\n          ./scripts/quorum-store-gate.sh"));
    assert!(
        !wf.contains("PR_AUTHOR_MODELS"),
        "PR must not be able to set it"
    );
    assert!(
        wf.to_lowercase().contains("bootstrap"),
        "bootstrap case documented"
    );
}
