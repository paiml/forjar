//! Falsifiers (core).

use crate::*;

#[test]
fn real_tree_passes() {
    let n = judge(&repo().join(".github/workflows"), &expected_canary())
        .unwrap_or_else(|w| panic!("{w:#?}"));
    assert!(n >= 20, "guard measured only {n} workflow files");
}

#[test]
fn empty_and_missing_dir_fail() {
    let d = unique_dir("empty");
    fs::create_dir_all(&d).unwrap();
    assert!(!guard(&d));
    assert!(!guard(&d.join("absent")));
}

#[test]
fn reusable_runs_on_with_spot_label_fails() {
    let m = real_mut(
        "runs_on: '[\"self-hosted\",\"clean-room\",\"X64\"]'",
        "runs_on: '[\"self-hosted\",\"burst-x64\",\"burst-public\"]'",
    );
    red("runs-on", &m);
}

#[test]
fn signing_job_on_spot_label_fails() {
    let ci = "on: pull_request\njobs:\n  sign:\n    permissions:\n      id-token: write\n    runs-on: [self-hosted, burst-x64]\n    steps: []\n";
    red("sign", ci);
    assert!(guard(&scratch(
        "sign-ok",
        &ci.replace("burst-x64", "clean-room")
    )));
}

// ---- pool-canary.yml: parsed equal to the expected copy, dispatch-only ----

pub(crate) fn canary() -> String {
    fs::read_to_string(repo().join(".github/workflows/pool-canary.yml")).unwrap()
}

pub(crate) fn canary_dir(name: &str, text: &str) -> PathBuf {
    let d = scratch(name, "on: push\njobs: {}\n");
    fs::write(d.join("pool-canary.yml"), text).unwrap();
    d
}

pub(crate) fn red_canary(name: &str, text: &str) {
    assert!(
        !guard(&canary_dir(name, text)),
        "canary mutant {name} accepted"
    );
}

#[test]
fn canary_real_file_passes() {
    assert!(guard(&canary_dir("canary-ok", &canary())));
}

#[test]
fn canary_mutants_fail() {
    let c = canary();
    let cases: [(&str, String); 8] = [
        (
            "secret",
            c.replace("IID: ${{ inputs.iid }}", "IID: ${{ secrets.TOKEN }}"),
        ),
        (
            "idtoken",
            c.replace("contents: read", "contents: read\n  id-token: write"),
        ),
        (
            "trigger",
            c.replace(
                "on:\n  workflow_dispatch:",
                "on:\n  pull_request:\n  workflow_dispatch:",
            ),
        ),
        (
            "runs-on",
            c.replace("[pool-canary]", "[self-hosted, clean-room, X64]"),
        ),
        ("write", c.replace("contents: read", "contents: write")),
        (
            "step",
            c.replace("jq --version", "jq --version\n          echo hi"),
        ),
        (
            "extra-job",
            format!("{c}  extra:\n    runs-on: [self-hosted, pool-canary]\n    steps: []\n"),
        ),
        ("unparseable", format!("{c}\n  : : [\n")),
    ];
    for (n, m) in cases {
        assert_ne!(c, m, "{n}");
        red_canary(n, &m);
    }
    // a comment is not a change to the parsed document
    assert!(guard(&canary_dir("canary-comment", &format!("{}# x\n", c))));
}

#[test]
fn canary_label_in_another_file_or_name_fails() {
    let d = scratch("canary-elsewhere", "on: push\njobs: {}\n");
    fs::write(d.join("copy.yml"), canary()).unwrap();
    assert!(!guard(&d));
}

// ---- spot job template ----

pub(crate) const SPOT_JOB: &str = "on: pull_request\njobs:\n  lockfile:\n    runs-on: ${{ (github.event_name == 'pull_request' && vars.PR_SPOT_OPTIN == '1') && fromJSON('[\"self-hosted\",\"burst-x64\",\"burst-public\"]') || fromJSON('[\"self-hosted\",\"clean-room\",\"X64\"]') }}\n    permissions:\n      contents: read\n    steps: []\n";

#[test]
fn spot_job_template_passes() {
    assert!(guard(&scratch("spot-ok", SPOT_JOB)));
    // comments after a job header are not a change to the parsed document
    assert!(guard(&scratch(
        "spot-comment",
        &SPOT_JOB.replace("  lockfile:\n", "  lockfile: # c\n")
    )));
}

#[test]
fn spot_job_with_secret_fails() {
    red(
        "spot-secret",
        &SPOT_JOB.replace(
            "steps: []",
            "steps:\n      - run: x\n        env:\n          T: ${{ secrets.X }}",
        ),
    );
}

#[test]
fn spot_job_permission_variants_fail() {
    red(
        "write-all",
        &SPOT_JOB.replace(
            "permissions:\n      contents: read",
            "permissions: write-all",
        ),
    );
    for extra in ["pull-requests: write", "id-token: write", "contents: write"] {
        let base = if extra.starts_with("contents") {
            SPOT_JOB.replace("contents: read", extra)
        } else {
            SPOT_JOB.replace("contents: read", &format!("contents: read\n      {extra}"))
        };
        red("scope", &base);
    }
    red(
        "noperm",
        &SPOT_JOB.replace("    permissions:\n      contents: read\n", ""),
    );
    red(
        "flow-perm-write",
        &SPOT_JOB.replace(
            "permissions:\n      contents: read",
            "permissions: {contents: write}",
        ),
    );
}

#[test]
fn spot_job_on_non_pull_request_event_fails() {
    for ev in [
        "github.event_name != 'pull_request'",
        "github.event_name == 'push'",
        "true",
    ] {
        red(
            "event",
            &SPOT_JOB.replace("github.event_name == 'pull_request'", ev),
        );
    }
}

#[test]
fn real_ci_spot_jobs_need_explicit_read_permissions() {
    red(
        "real-noperm",
        &real_mut("    permissions:\n      contents: read\n", ""),
    );
}

// ---- r3 mutants, kept red ----

#[test]
fn r3_workflow_level_mutants_fail() {
    red_why(
        "r3-top-env",
        &real_mut("\nenv:\n", "\nenv:\n  TOK: ${{ secrets.FOO }}\n"),
        "workflow level names secrets",
    );
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
    let m = SPOT_JOB
        .lines()
        .map(|l| {
            if l.starts_with("    runs-on:") {
                "    env:\n      T: ${{ secrets.FOO }}\n    runs-on:\n      - self-hosted\n      - burst-x64".to_string()
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    red("r3-block-list", &m);
}

#[test]
fn r3_matrix_label_fails() {
    red(
        "r3-matrix",
        "on: pull_request\njobs:\n  lockfile:\n    runs-on: ${{ matrix.os }}\n    strategy:\n      matrix:\n        os: [burst-x64]\n    permissions:\n      contents: read\n    steps: []\n",
    );
}

#[test]
fn r3_or_true_fails() {
    red(
        "r3-or-true",
        &real_mut(
            "vars.PR_SPOT_OPTIN == '1')",
            "vars.PR_SPOT_OPTIN == '1' || true)",
        ),
    );
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
    red("r3-other-job", &SPOT_JOB.replace("  lockfile:", "  other:"));
    let m = real_mut(
        "vars.BURST_OPTIN == '1') && fromJSON('[\"self-hosted\",\"burst-x64\"]')",
        "vars.PR_SPOT_OPTIN == '1') && fromJSON('[\"self-hosted\",\"burst-x64\",\"burst-public\"]')",
    );
    red("r3-wrong-canon", &m);
}

#[test]
fn r3_opt_in_variable_anywhere_else_fails() {
    red(
        "r3-var-step",
        &real_mut(
            "    steps:\n      - uses: actions/checkout",
            "    steps:\n      - if: ${{ vars.PR_SPOT_OPTIN == '1' }}\n        uses: actions/checkout",
        ),
    );
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
            "    permissions:",
            "    runs-on: [self-hosted, clean-room]\n    permissions:",
        ),
    );
    red(
        "r3-dup-job",
        &format!("{SPOT_JOB}  lockfile:\n    runs-on: [self-hosted, clean-room]\n    steps: []\n"),
    );
}

#[test]
fn r3_flow_jobs_burst_fails() {
    red(
        "r3-flow-jobs",
        "on: pull_request\njobs: {lockfile: {runs-on: burst-x64}}\n",
    );
}
