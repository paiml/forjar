//! FLAKE-001 C3 (paiml/infra#3479): the quorum gate must give the SAME verdict on every runner.
//! A host-local receipt store (/mnt/nvme-raid0/...) exists on one box only, so the verdict depended
//! on which host picked the job. With QUORUM_STORE unset the gate fetches the receipt by sha from
//! `refs/quorum/<repo>/<pr>/<sha>` on QUORUM_REMOTE (git transport: no REST quota, O(1)).
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SHA: &str = "141132a621af2843efe9d6689b2b469ef34f7ce4";
const OTHER: &str = "241132a621af2843efe9d6689b2b469ef34f7ce4";

fn gate() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/quorum-store-gate.sh")
}

fn git(dir: &Path, args: &[&str]) -> String {
    let o = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

fn receipt(sha: &str) -> String {
    let l = |n: &str, m: &str| {
        format!(r#"{{"lane":"{n}","model":"{m}","verdict":"PASS","blockers":[]}}"#)
    };
    format!(
        r#"{{"repo":"paiml/forjar","pr":798,"sha":"{sha}","author_model":"claude-sonnet-5-5","verdicts":[{},{},{}]}}"#,
        l("a", "claude-opus-5-5"),
        l("b", "claude-haiku-5-5"),
        l("c", "gemini-3")
    )
}

/// Plant `body` as a blob under refs/quorum/paiml/forjar/798/<ref_sha> in a bare remote, run the gate
/// for head SHA with QUORUM_STORE unset (the host-independent path).
fn run(body: Option<String>, ref_sha: &str) -> Output {
    let t = tempfile::tempdir().unwrap();
    let remote = t.path().join("remote.git");
    std::fs::create_dir(&remote).unwrap();
    git(&remote, &["init", "-q", "--bare"]);
    if let Some(b) = body {
        let f = t.path().join("r.json");
        std::fs::write(&f, b).unwrap();
        let blob = git(&remote, &["hash-object", "-w", f.to_str().unwrap()]);
        git(
            &remote,
            &[
                "update-ref",
                &format!("refs/quorum/paiml/forjar/798/{ref_sha}"),
                &blob,
            ],
        );
    }
    let work = t.path().join("work");
    std::fs::create_dir(&work).unwrap();
    git(&work, &["init", "-q"]);
    Command::new("bash")
        .arg(gate())
        .current_dir(&work)
        .env_remove("QUORUM_STORE")
        .env("QUORUM_REMOTE", &remote)
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
fn valid_receipt_by_ref_passes_without_any_host_path() {
    let o = run(Some(receipt(SHA)), SHA);
    assert!(o.status.success(), "ref receipt must pass: {}", text(&o));
}

#[test]
fn absent_ref_fails_closed() {
    let o = run(None, SHA);
    assert!(
        !o.status.success() && text(&o).contains("FAIL quorum receipt"),
        "{}",
        text(&o)
    );
}

#[test]
fn receipt_for_other_sha_planted_under_this_ref_fails() {
    let o = run(Some(receipt(OTHER)), SHA);
    assert!(
        !o.status.success(),
        "receipt claiming another sha must fail: {}",
        text(&o)
    );
}

#[test]
fn unreachable_remote_is_unmeasured_not_pass() {
    let o = Command::new("bash")
        .arg(gate())
        .env_remove("QUORUM_STORE")
        .env("QUORUM_REMOTE", "/nonexistent/remote.git")
        .env("QUORUM_REPO", "paiml/forjar")
        .env("PR_NUMBER", "798")
        .env("PR_HEAD_SHA", SHA)
        .env("QUORUM_TEST_MODE", "1")
        .env(
            "PR_AUTHOR_MODELS",
            "Claude Sonnet 5.5 <noreply@anthropic.com>",
        )
        .env_remove("GITHUB_ACTIONS")
        .output()
        .unwrap();
    assert!(
        !o.status.success() && text(&o).contains("UNMEASURED"),
        "{}",
        text(&o)
    );
}
