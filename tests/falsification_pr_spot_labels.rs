//! A job with `id-token: write` must never resolve to a spot (burst) label. The shell guard
//! `tests/pr-spot-labels-guard.sh` decides; this runs it on the real tree (must pass) and on
//! mutants (must fail), so the guard is shown capable of failing.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn guard(dir: &std::path::Path) -> bool {
    Command::new("bash")
        .arg(repo().join("tests/pr-spot-labels-guard.sh"))
        .arg(dir)
        .output()
        .expect("bash runs the guard")
        .status
        .success()
}

fn scratch(name: &str, ci: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("forjar-spot-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    fs::write(d.join("ci.yml"), ci).unwrap();
    d
}

#[test]
fn real_tree_passes() {
    assert!(guard(&repo().join(".github/workflows")));
}

#[test]
fn reusable_runs_on_with_spot_label_fails() {
    let ci = fs::read_to_string(repo().join(".github/workflows/ci.yml")).unwrap();
    let mutant = ci.replace(
        "runs_on: '[\"self-hosted\",\"clean-room\",\"X64\"]'",
        "runs_on: '[\"self-hosted\",\"burst-x64\",\"burst-public\"]'",
    );
    assert_ne!(ci, mutant, "mutation must apply");
    assert!(!guard(&scratch("runs-on", &mutant)));
}

#[test]
fn signing_job_on_spot_label_fails() {
    let ci = "on: pull_request\njobs:\n  sign:\n    permissions:\n      id-token: write\n    runs-on: [self-hosted, burst-x64]\n    steps: []\n";
    assert!(!guard(&scratch("sign", ci)));
    let ok = ci.replace("burst-x64", "clean-room");
    assert!(guard(&scratch("sign-ok", &ok)));
}

#[test]
fn empty_dir_fails() {
    let d = std::env::temp_dir().join(format!("forjar-spot-empty-{}", std::process::id()));
    fs::create_dir_all(&d).unwrap();
    assert!(!guard(&d));
}

fn canary() -> String {
    fs::read_to_string(repo().join(".github/workflows/pool-canary.yml")).unwrap()
}

fn canary_dir(name: &str, text: &str) -> PathBuf {
    let d = scratch(name, "on: push\njobs: {}\n");
    fs::write(d.join("pool-canary.yml"), text).unwrap();
    d
}

#[test]
fn canary_real_file_passes() {
    assert!(guard(&canary_dir("canary-ok", &canary())));
}

#[test]
fn canary_with_secret_fails() {
    let m = canary().replace("IID: ${{ inputs.iid }}", "IID: ${{ secrets.TOKEN }}");
    assert_ne!(canary(), m);
    assert!(!guard(&canary_dir("canary-secret", &m)));
}

#[test]
fn canary_with_id_token_fails() {
    let m = canary().replace("contents: read", "contents: read\n  id-token: write");
    assert_ne!(canary(), m);
    assert!(!guard(&canary_dir("canary-idtoken", &m)));
}

#[test]
fn canary_with_extra_trigger_fails() {
    let m = canary().replace(
        "on:\n  workflow_dispatch:",
        "on:\n  pull_request:\n  workflow_dispatch:",
    );
    assert_ne!(canary(), m);
    assert!(!guard(&canary_dir("canary-trigger", &m)));
}

#[test]
fn canary_with_other_runs_on_fails() {
    let m = canary().replace(
        "[self-hosted, pool-canary]",
        "[self-hosted, clean-room, X64]",
    );
    assert_ne!(canary(), m);
    assert!(!guard(&canary_dir("canary-runs-on", &m)));
}

fn ci_text() -> String {
    fs::read_to_string(repo().join(".github/workflows/ci.yml")).unwrap()
}

const SPOT_JOB: &str = "on: pull_request\njobs:\n  lockfile:\n    runs-on: ${{ (github.event_name == 'pull_request' && vars.PR_SPOT_OPTIN == '1') && fromJSON('[\"self-hosted\",\"burst-x64\",\"burst-public\"]') || fromJSON('[\"self-hosted\",\"clean-room\",\"X64\"]') }}\n    permissions:\n      contents: read\n    steps: []\n";

#[test]
fn spot_job_template_passes() {
    assert!(guard(&scratch("spot-ok", SPOT_JOB)));
}

#[test]
fn spot_job_with_secret_fails() {
    let m = SPOT_JOB.replace(
        "steps: []",
        "steps:\n      - run: x\n        env:\n          T: ${{ secrets.X }}",
    );
    assert_ne!(SPOT_JOB, m);
    assert!(!guard(&scratch("spot-secret", &m)));
}

#[test]
fn spot_job_with_write_all_fails() {
    let m = SPOT_JOB.replace(
        "permissions:\n      contents: read",
        "permissions: write-all",
    );
    assert_ne!(SPOT_JOB, m);
    assert!(!guard(&scratch("spot-write-all", &m)));
}

#[test]
fn spot_job_with_write_scope_fails() {
    for extra in ["pull-requests: write", "id-token: write", "contents: write"] {
        let base = if extra.starts_with("contents") {
            SPOT_JOB.replace("contents: read", extra)
        } else {
            SPOT_JOB.replace("contents: read", &format!("contents: read\n      {extra}"))
        };
        assert_ne!(SPOT_JOB, base);
        assert!(!guard(&scratch("spot-scope", &base)), "{extra}");
    }
}

#[test]
fn spot_job_without_permissions_fails() {
    let m = SPOT_JOB.replace("    permissions:\n      contents: read\n", "");
    assert_ne!(SPOT_JOB, m);
    assert!(!guard(&scratch("spot-noperm", &m)));
}

#[test]
fn spot_job_on_non_pull_request_event_fails() {
    for ev in [
        "github.event_name != 'pull_request'",
        "github.event_name == 'push'",
        "true",
    ] {
        let m = SPOT_JOB.replace("github.event_name == 'pull_request'", ev);
        assert_ne!(SPOT_JOB, m);
        assert!(!guard(&scratch("spot-event", &m)), "{ev}");
    }
}

#[test]
fn canary_with_extra_job_fails() {
    let m = format!(
        "{}  extra:\n    runs-on: [self-hosted, pool-canary]\n    steps: []\n",
        canary()
    );
    assert!(!guard(&canary_dir("canary-extra-job", &m)));
}

#[test]
fn real_ci_spot_jobs_have_explicit_read_permissions() {
    // every job on the PR_SPOT_OPTIN expression is named here; dropping its permissions block fails
    let ci = ci_text();
    let m = ci.replacen("    permissions:\n      contents: read\n", "", 1);
    assert_ne!(ci, m);
    assert!(!guard(&scratch("real-noperm", &m)));
}

#[test]
fn ci_wires_the_guard_into_the_gate() {
    let ci = ci_text();
    assert!(
        ci.contains("tests/pr-spot-labels-guard.sh"),
        "ci.yml never runs the guard"
    );
    assert!(
        ci.contains("falsification_pr_spot_labels"),
        "ci.yml never runs the Rust test"
    );
    assert!(ci.contains("needs: [classify, ci, lockfile, examples-validate, no-default-features, doctests, dogfood-surface, dogfood-guards, spot-labels-guard]"), "gate does not need the guard job");
}

// ---- r3 allowlist mutants: the guard is an ALLOWLIST (exact job ids, exact runs-on text) ----

const PS: &str = "runs-on: ${{ (github.event_name == 'pull_request' && vars.PR_SPOT_OPTIN == '1') && fromJSON('[\"self-hosted\",\"burst-x64\",\"burst-public\"]') || fromJSON('[\"self-hosted\",\"clean-room\",\"X64\"]') }}";

fn red(name: &str, ci: &str) {
    assert!(!guard(&scratch(name, ci)), "mutant {name} was accepted");
}

fn real_mut(from: &str, to: &str) -> String {
    let ci = ci_text();
    let m = ci.replacen(from, to, 1);
    assert_ne!(ci, m, "mutation must apply: {from}");
    m
}

#[test]
fn r3_workflow_level_env_secret_fails() {
    red(
        "r3-top-env",
        &real_mut("\njobs:\n", "\nenv:\n  TOK: ${{ secrets.FOO }}\njobs:\n"),
    );
}

#[test]
fn r3_workflow_level_write_permissions_fails() {
    red(
        "r3-top-perm",
        &real_mut("\njobs:\n", "\npermissions:\n  contents: write\njobs:\n"),
    );
    red(
        "r3-top-perm-all",
        &real_mut("\njobs:\n", "\npermissions: write-all\njobs:\n"),
    );
}

#[test]
fn r3_block_list_runs_on_with_secrets_env_fails() {
    let m = SPOT_JOB.replace(
        &format!("    {PS}\n"),
        "    env:\n      T: ${{ secrets.FOO }}\n    runs-on:\n      - self-hosted\n      - burst-x64\n",
    );
    assert_ne!(SPOT_JOB, m);
    red("r3-block-list", &m);
}

#[test]
fn r3_matrix_label_fails() {
    let ci = "on: pull_request\njobs:\n  lockfile:\n    runs-on: ${{ matrix.os }}\n    strategy:\n      matrix:\n        os: [burst-x64]\n    permissions:\n      contents: read\n    steps: []\n";
    red("r3-matrix", ci);
}

#[test]
fn r3_or_true_fails() {
    let m = real_mut(
        "vars.PR_SPOT_OPTIN == '1')",
        "vars.PR_SPOT_OPTIN == '1' || true)",
    );
    red("r3-or-true", &m);
    red(
        "r3-or-true2",
        &real_mut(
            "github.event_name == 'pull_request' && vars.PR_SPOT_OPTIN",
            "github.event_name == 'pull_request' || vars.PR_SPOT_OPTIN",
        ),
    );
}

#[test]
fn r3_reusable_runs_on_variants_fail() {
    for v in [
        "runs_on: ${{ vars.X }}",
        "runs_on: >-\n        ${{ vars.X }}",
        "runs_on: ${{ inputs.x }}",
        "runs_on: '[\"self-hosted\",\"burst-x64\"]'",
    ] {
        red(
            "r3-reusable",
            &real_mut("runs_on: '[\"self-hosted\",\"clean-room\",\"X64\"]'", v),
        );
    }
}

#[test]
fn r3_burst_in_non_allowlisted_job_fails() {
    // the canonical text on a job that is not in the allowlist
    let ci = format!("on: pull_request\njobs:\n  other:\n    {PS}\n    permissions:\n      contents: read\n    steps: []\n");
    red("r3-other-job", &ci);
    // the PR_SPOT text on a BURST_OPTIN job (wrong canonical line for that job id)
    let m = real_mut("  doctests:\n", "  doctests:\n    x: y\n").replacen(
        "vars.BURST_OPTIN == '1') && fromJSON('[\"self-hosted\",\"burst-x64\"]')",
        "vars.PR_SPOT_OPTIN == '1') && fromJSON('[\"self-hosted\",\"burst-x64\",\"burst-public\"]')",
        1,
    );
    red("r3-wrong-canon", &m);
}

#[test]
fn r3_opt_in_variable_anywhere_else_fails() {
    red("r3-var-step", &real_mut("    steps:\n      - uses: actions/checkout", "    steps:\n      - if: ${{ vars.PR_SPOT_OPTIN == '1' }}\n        uses: actions/checkout"));
    red(
        "r3-ro-var",
        &real_mut(
            "runs-on: [self-hosted, clean-room, X64]",
            "runs-on: ${{ vars.POOL }}",
        ),
    );
}

#[test]
fn r3_new_workflow_with_burst_fails_in_either_extension() {
    for ext in ["yml", "yaml"] {
        let d = scratch("r3-new", "on: push\njobs: {}\n");
        fs::write(
            d.join(format!("x.{ext}")),
            "on: push\njobs:\n  a:\n    runs-on: [self-hosted, burst-x64]\n    steps: []\n",
        )
        .unwrap();
        assert!(!guard(&d), "{ext}");
    }
}

#[test]
fn r3_duplicate_job_or_runs_on_fails() {
    red(
        "r3-dup-runs-on",
        &SPOT_JOB.replace(
            &format!("    {PS}\n"),
            &format!("    {PS}\n    runs-on: [self-hosted, clean-room]\n"),
        ),
    );
    red(
        "r3-dup-job",
        &format!("{SPOT_JOB}  lockfile:\n    runs-on: [self-hosted, clean-room]\n    steps: []\n"),
    );
}

#[test]
fn r3_unparseable_spot_file_fails() {
    // cannot judge = red
    red(
        "r3-flow-jobs",
        "on: pull_request\njobs: {lockfile: {runs-on: burst-x64}}\n",
    );
    red(
        "r3-job-header-comment",
        &SPOT_JOB.replace("  lockfile:\n", "  lockfile: # c\n"),
    );
}

#[test]
fn r3_canary_must_be_byte_identical() {
    let c = canary();
    red_canary(
        "r3-canary-write",
        &c.replace("contents: read", "contents: write"),
    );
    red_canary("r3-canary-comment", &format!("{c}# x\n"));
    red_canary(
        "r3-canary-step",
        &c.replace("jq --version", "jq --version\n          echo hi"),
    );
}

fn red_canary(name: &str, text: &str) {
    assert!(
        !guard(&canary_dir(name, text)),
        "canary mutant {name} accepted"
    );
}

// ---- the CI step must go red when the guard is neutered ----

fn step_body(ci: &str, name: &str) -> String {
    let mut lines = ci.lines();
    let hdr = format!("      - name: {name}");
    lines.find(|l| *l == hdr).expect("step present");
    let mut body = Vec::new();
    let mut in_run = false;
    for l in lines {
        if l.starts_with("      - ") || (!l.is_empty() && !l.starts_with("        ")) {
            break;
        }
        if in_run {
            body.push(l.strip_prefix("          ").unwrap_or(l).to_string());
        } else if l.trim() == "run: |" {
            in_run = true;
        }
    }
    body.join("\n")
}

#[test]
fn ci_mutant_step_goes_red_when_the_guard_is_neutered() {
    if std::env::var_os("FORJAR_SPOT_INNER").is_some() {
        return; // the inner run (neutered guard) must not recurse
    }
    let t = std::env::temp_dir().join(format!("forjar-spot-step-{}", std::process::id()));
    let _ = fs::remove_dir_all(&t);
    fs::create_dir_all(t.join("tests")).unwrap();
    fs::create_dir_all(t.join(".github")).unwrap();
    let r = repo();
    fs::copy(
        r.join("tests/falsification_pr_spot_labels.rs"),
        t.join("tests/falsification_pr_spot_labels.rs"),
    )
    .unwrap();
    fs::copy(
        r.join("tests/pool-canary.expected.yml"),
        t.join("tests/pool-canary.expected.yml"),
    )
    .unwrap();
    fs::write(
        t.join("tests/pr-spot-labels-guard.sh"),
        "#!/usr/bin/env bash\nexit 0\n",
    )
    .unwrap();
    let wf = t.join(".github/workflows");
    fs::create_dir_all(&wf).unwrap();
    for e in fs::read_dir(r.join(".github/workflows")).unwrap() {
        let p = e.unwrap().path();
        fs::copy(&p, wf.join(p.file_name().unwrap())).unwrap();
    }
    let ci = ci_text();
    let body = step_body(&ci, "Guard mutants (each must be refused)");
    assert!(body.contains("rustc"), "step body not found: {body}");
    let script = t.join("step.sh");
    fs::write(&script, &body).unwrap();
    // GitHub's `shell: bash` runs `bash --noprofile --norc -eo pipefail {0}`
    let out = Command::new("bash")
        .args(["--noprofile", "--norc", "-eo", "pipefail"])
        .arg(&script)
        .current_dir(&t)
        .env("FORJAR_SPOT_INNER", "1")
        .output()
        .unwrap();
    let _ = fs::remove_dir_all(&t);
    assert!(
        !out.status.success(),
        "step stayed green with a neutered guard"
    );
    assert!(!body.contains("| tail"), "the step pipes into tail");
}
