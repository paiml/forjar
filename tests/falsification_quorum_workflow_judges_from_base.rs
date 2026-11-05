//! PMAT-5882 / paiml/forjar#876 — the "quorum receipt" check must measure the receipt
//! store, and must judge a PR with the gate on its BASE commit.
//!
//! Root cause: the old workflow ran `scripts/quorum-gate.sh`, which demands a committed
//! `.quorum/<branch>.json`. A receipt binds to the head sha and committing it moves the
//! head, so the check was red on every PR by construction (29 open PRs).
//!
//! These tests execute the workflow's own "Fetch gate from the PR base commit" shell
//! step against a synthetic repo, so a mutated step (always use the PR copy, or fall
//! back after a failed base read) turns them RED.
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workflow() -> String {
    std::fs::read_to_string(root().join(".github/workflows/quorum.yml")).expect("quorum.yml")
}

/// The `run: |` body of the named step, dedented.
fn step_run(name: &str) -> String {
    let wf = workflow();
    let at = wf.find(&format!("- name: {name}")).expect("step present");
    let rest = &wf[at..];
    let r = rest.find("        run: |\n").expect("run block") + "        run: |\n".len();
    rest[r..]
        .lines()
        .take_while(|l| l.starts_with("          ") || l.is_empty())
        .map(|l| l.strip_prefix("          ").unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

fn git(dir: &Path, args: &[&str]) -> String {
    let o = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .output()
        .expect("git");
    assert!(
        o.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8(o.stdout).unwrap().trim().to_string()
}

/// Repo whose base commit has `base_gate` (or none) and whose working tree has `pr_gate`.
/// Returns (repo dir, base sha, tmp dir).
fn fixture(tag: &str, base_gate: Option<&str>, pr_gate: &str) -> (PathBuf, String, PathBuf) {
    let d = std::env::temp_dir().join(format!("pmat5882-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("repo/scripts")).unwrap();
    let repo = d.join("repo");
    git(&repo, &["init", "-q"]);
    std::fs::write(repo.join("README"), "x").unwrap();
    if let Some(g) = base_gate {
        std::fs::write(repo.join("scripts/quorum-store-gate.sh"), g).unwrap();
        std::fs::write(
            repo.join("scripts/quorum-models.json"),
            "{\"from\":\"base\"}",
        )
        .unwrap();
        std::fs::write(
            repo.join("scripts/quorum-allowed-signers"),
            "# base signers\n",
        )
        .unwrap();
    }
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "base"]);
    let base = git(&repo, &["rev-parse", "HEAD"]);
    std::fs::write(repo.join("scripts/quorum-store-gate.sh"), pr_gate).unwrap();
    std::fs::write(repo.join("scripts/quorum-models.json"), "{\"from\":\"pr\"}").unwrap();
    std::fs::write(
        repo.join("scripts/quorum-allowed-signers"),
        "# PR-ADDED-SIGNER\n",
    )
    .unwrap();
    (repo, base, d)
}

fn run_step(repo: &Path, base: &str, tmp: &Path) -> (bool, String, String) {
    let envf = tmp.join("ghenv");
    let o = Command::new("bash")
        .args([
            "-euo",
            "pipefail",
            "-c",
            &step_run("Fetch gate from the PR base commit"),
        ])
        .current_dir(repo)
        .env("PR_BASE_SHA", base)
        .env("RUNNER_TEMP", tmp)
        .env("GITHUB_ENV", &envf)
        .output()
        .unwrap();
    let out = String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr);
    let gate = std::fs::read_to_string(tmp.join("quorum-base-gate/scripts/quorum-store-gate.sh"))
        .unwrap_or_default();
    (o.status.success(), out, gate)
}

#[test]
fn a_pr_cannot_weaken_the_judge_that_reads_it() {
    let (repo, base, d) = fixture("weak", Some("echo BASE-GATE\n"), "exit 0 # PR-WEAKENED\n");
    let (ok, out, gate) = run_step(&repo, &base, &d);
    assert!(ok, "{out}");
    assert!(
        gate.contains("BASE-GATE") && !gate.contains("PR-WEAKENED"),
        "judge must come from base: {gate}"
    );
    assert!(
        !out.contains("BOOTSTRAP"),
        "a base that has the gate never bootstraps: {out}"
    );
}

#[test]
fn bootstrap_uses_the_pr_copy_only_when_the_base_has_no_gate_and_says_so() {
    let (repo, base, d) = fixture("boot", None, "echo PR-GATE\n");
    let (ok, out, gate) = run_step(&repo, &base, &d);
    assert!(ok, "{out}");
    assert!(out.contains("BOOTSTRAP"), "bootstrap must be loud: {out}");
    assert!(gate.contains("PR-GATE"));
}

#[test]
fn a_base_with_the_script_but_no_allowlist_is_red_not_a_fallback() {
    let (repo, base, d) = fixture("half", Some("echo BASE-GATE\n"), "echo PR\n");
    git(
        &repo,
        &["rm", "-q", "--cached", "scripts/quorum-models.json"],
    );
    git(&repo, &["commit", "-qm", "drop allowlist from base"]);
    let base2 = git(&repo, &["rev-parse", "HEAD"]);
    let _ = base;
    let (ok, out, _) = run_step(&repo, &base2, &d);
    assert!(
        !ok,
        "missing allowlist on a base that has the gate must fail: {out}"
    );
}

#[test]
fn signers_come_from_the_base_never_the_pr() {
    let (repo, base, d) = fixture("sig", Some("echo BASE-GATE\n"), "echo PR\n");
    let (ok, out, _) = run_step(&repo, &base, &d);
    assert!(ok, "{out}");
    let signers =
        std::fs::read_to_string(d.join("quorum-base-gate/scripts/quorum-allowed-signers")).unwrap();
    assert!(
        signers.contains("base signers") && !signers.contains("PR-ADDED-SIGNER"),
        "a PR must not add its own signer: {signers}"
    );
}

#[test]
fn a_base_with_the_gate_but_no_signers_file_is_red() {
    let (repo, _b, d) = fixture("nosig", Some("echo BASE-GATE\n"), "echo PR\n");
    git(
        &repo,
        &["rm", "-q", "--cached", "scripts/quorum-allowed-signers"],
    );
    git(&repo, &["commit", "-qm", "drop signers from base"]);
    let base2 = git(&repo, &["rev-parse", "HEAD"]);
    let (ok, out, _) = run_step(&repo, &base2, &d);
    assert!(
        !ok,
        "missing signers on a base that has the gate must fail: {out}"
    );
}

#[test]
fn the_check_runs_the_store_gate_not_the_committed_file_gate() {
    let wf = workflow();
    assert!(
        wf.contains("bash \"$QUORUM_BASE_GATE\""),
        "must run the base-judged store gate"
    );
    let verify = &wf[wf.find("- name: Verify quorum receipt").unwrap()..];
    assert!(
        !verify.contains("quorum-gate.sh"),
        "the committed .quorum/<branch>.json gate is the root cause"
    );
    for bad in ["continue-on-error", "|| true"] {
        assert!(!wf.contains(bad), "no third state on the PR path: {bad}");
    }
}
