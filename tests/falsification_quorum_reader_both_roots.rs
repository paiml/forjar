//! The CI quorum gate reads the receipt store from BOTH roots
//! (`/mnt/nvme-raid0/cop-inbox/promote-state`, `/mnt/nvme-raid0/promote-state`):
//! first root holding the file wins, two roots that disagree on bytes FAIL,
//! neither root = UNMEASURED = RED. QUORUM_STORE here is a colon list (test mode only).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SHA: &str = "141132a621af2843efe9d6689b2b469ef34f7ce4";
const OTHER: &str = "2222222222222222222222222222222222222222";

fn gate() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/quorum-store-gate.sh")
}

fn body(sha: &str, pad: &str) -> String {
    let l = |n: &str, m: &str| {
        format!(r#"{{"lane":"{n}","model":"{m}","verdict":"PASS","blockers":[]}}"#)
    };
    format!(
        r#"{{"repo":"paiml/forjar","pr":798,"sha":"{sha}","author_model":"claude-sonnet-5-5"{pad},"verdicts":[{},{},{}]}}"#,
        l("a", "claude-opus-5-5"),
        l("b", "claude-haiku-5-5"),
        l("c", "gemini-3")
    )
}

fn put(root: &Path, file_sha: &str, content: &str) {
    let d = root.join("quorum/paiml/forjar/798");
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join(format!("{file_sha}.json")), content).unwrap();
}

/// `r1`/`r2`: Some((file_sha, content)) to plant in that root.
fn run(r1: Option<(&str, String)>, r2: Option<(&str, String)>) -> Output {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (dir.path().join("r1"), dir.path().join("r2"));
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    if let Some((s, c)) = r1 {
        put(&a, s, &c);
    }
    if let Some((s, c)) = r2 {
        put(&b, s, &c);
    }
    Command::new("bash")
        .arg(gate())
        .env("QUORUM_STORE", format!("{}:{}", a.display(), b.display()))
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

#[test]
fn receipt_only_in_second_root_passes() {
    let o = run(None, Some((SHA, body(SHA, ""))));
    assert!(o.status.success(), "{}", text(&o));
}

#[test]
fn receipt_only_in_first_root_passes() {
    let o = run(Some((SHA, body(SHA, ""))), None);
    assert!(o.status.success(), "{}", text(&o));
}

#[test]
fn receipt_in_both_roots_identical_passes() {
    let o = run(Some((SHA, body(SHA, ""))), Some((SHA, body(SHA, ""))));
    assert!(o.status.success(), "{}", text(&o));
}

#[test]
fn receipt_in_neither_root_fails() {
    let o = run(None, None);
    assert!(!o.status.success(), "{}", text(&o));
    assert!(text(&o).contains("UNMEASURED"), "{}", text(&o));
}

#[test]
fn roots_that_disagree_fail() {
    let o = run(
        Some((SHA, body(SHA, ""))),
        Some((SHA, body(SHA, r#","note":"x""#))),
    );
    assert!(!o.status.success(), "{}", text(&o));
    assert!(text(&o).contains("disagree"), "{}", text(&o));
}

#[test]
fn receipt_for_another_sha_fails_in_either_root() {
    let o = run(None, Some((OTHER, body(OTHER, ""))));
    assert!(!o.status.success(), "{}", text(&o));
    let o = run(Some((OTHER, body(OTHER, ""))), None);
    assert!(!o.status.success(), "{}", text(&o));
}

#[test]
fn receipt_naming_another_sha_in_second_root_fails() {
    let o = run(None, Some((SHA, body(OTHER, ""))));
    assert!(!o.status.success(), "{}", text(&o));
}

#[test]
fn store_override_is_ignored_under_github_actions() {
    let dir = tempfile::tempdir().unwrap();
    put(dir.path(), SHA, &body(SHA, ""));
    let o = Command::new("bash")
        .arg(gate())
        .env("QUORUM_STORE", dir.path())
        .env("QUORUM_REPO", "paiml/forjar")
        .env("PR_NUMBER", "798")
        .env(
            "PR_AUTHOR_MODELS",
            "Claude Sonnet 5.5 <noreply@anthropic.com>",
        )
        .env("QUORUM_TEST_MODE", "1")
        .env("GITHUB_ACTIONS", "true")
        .env("PR_HEAD_SHA", SHA)
        .output()
        .unwrap();
    assert!(!o.status.success(), "{}", text(&o));
}

#[test]
fn fable_is_not_an_accepted_model() {
    let m = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/quorum-models.json"),
    )
    .unwrap();
    assert!(!m.to_lowercase().contains("fable"));
}
