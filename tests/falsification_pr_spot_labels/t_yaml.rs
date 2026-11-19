//! Falsifiers (yaml).

use crate::*;

// ---- r4: YAML spellings that resolve to the same value, planted RED first ----

pub(crate) fn plant(job_body: &str) -> String {
    format!("on: pull_request\njobs:\n{job_body}")
}

#[test]
fn r4_quoted_key_and_escape_fail() {
    red(
        "r4-quoted-key",
        &plant("  evil:\n    \"runs-on\": [\"self-hosted\", \"burst-x64\"]\n    steps: []\n"),
    );
    red(
        "r4-escape",
        &plant("  evil:\n    runs-on: [\"self-hosted\", \"bur\\x73t-x64\"]\n    steps: []\n"),
    );
    red(
        "r4-quoted-key-escape-secret",
        &plant("  evil:\n    \"runs-on\": [\"self-hosted\", \"bur\\x73t-x64\"]\n    env:\n      T: \"${{ secrets.TOK }}\"\n    steps: []\n"),
    );
}

#[test]
fn r4_flow_style_and_explicit_key_fail() {
    red(
        "r4-flow-job",
        &plant("  evil: {\"runs-on\": [\"self-hosted\", \"bur\\x73t-x64\"]}\n"),
    );
    red(
        "r4-explicit-key",
        &plant("  evil:\n    ? runs-on\n    : [self-hosted, bur\\x73t-x64]\n    steps: []\n")
            .replace("bur\\x73t-x64]\n", "burst-x64]\n"),
    );
    red(
        "r4-explicit-key-escaped",
        &plant(
            "  evil:\n    ? runs-on\n    : [\"self-hosted\", \"bur\\x73t-x64\"]\n    steps: []\n",
        ),
    );
    red(
        "r4-flow-all",
        "{on: pull_request, jobs: {evil: {runs-on: [self-hosted, burst-x64]}}}\n",
    );
}

#[test]
fn r4_expression_runs_on_fails() {
    // fed from a needs output
    red(
        "r4-needs",
        &plant("  pick:\n    runs-on: [self-hosted, clean-room]\n    outputs:\n      l: ${{ steps.s.outputs.l }}\n    steps: []\n  evil:\n    needs: pick\n    runs-on: ${{ fromJSON(needs.pick.outputs.l) }}\n    steps: []\n"),
    );
    // matrix fed from needs output or vars
    red(
        "r4-matrix-needs",
        &plant("  evil:\n    runs-on: ${{ matrix.os }}\n    strategy:\n      matrix:\n        os: ${{ fromJSON(needs.pick.outputs.l) }}\n    steps: []\n"),
    );
    red(
        "r4-matrix-vars",
        &plant("  evil:\n    runs-on: ${{ matrix.os }}\n    strategy:\n      matrix:\n        os: ${{ fromJSON(vars.LBL) }}\n    steps: []\n"),
    );
    red(
        "r4-vars",
        &plant("  evil:\n    runs-on: ${{ vars.LBL }}\n    steps: []\n"),
    );
    red(
        "r4-matrix-include-burst",
        &plant("  evil:\n    runs-on: ${{ matrix.os }}\n    strategy:\n      matrix:\n        os: [a]\n        include:\n          - os: bur\\x73t-x64\n    steps: []\n").replace("bur\\x73t", "burst"),
    );
    red(
        "r4-matrix-no-strategy",
        &plant("  evil:\n    runs-on: ${{ matrix.os }}\n    steps: []\n"),
    );
    // a literal matrix is fine
    assert!(guard(&scratch(
        "r4-matrix-literal-ok",
        &plant("  fine:\n    runs-on: ${{ matrix.os }}\n    strategy:\n      matrix:\n        os: [ubuntu-latest, macos-latest]\n    steps: []\n"),
    )));
}

#[test]
fn r4_map_form_runs_on_fails() {
    red(
        "r4-group-labels",
        &plant("  evil:\n    runs-on:\n      group: g\n      labels: [self-hosted, \"burst-x64\"]\n    steps: []\n"),
    );
    red(
        "r4-labels-escaped",
        &plant("  evil:\n    runs-on:\n      labels: \"bur\\x73t-x64\"\n    steps: []\n"),
    );
    red(
        "r4-unmodeled-shape",
        &plant("  evil:\n    runs-on: 5\n    steps: []\n"),
    );
    red(
        "r4-unmodeled-map-key",
        &plant("  evil:\n    runs-on:\n      other: x\n    steps: []\n"),
    );
}

#[test]
fn r4_reusable_call_passing_pool_value_fails() {
    red(
        "r4-reusable-quoted",
        &plant("  call:\n    uses: ./.github/workflows/x.yml\n    with:\n      \"runs_on\": '[\"self-hosted\",\"bur\\x73t-x64\"]'\n"),
    );
    red(
        "r4-reusable-vars",
        &plant("  call:\n    uses: ./.github/workflows/x.yml\n    with:\n      runs_on: ${{ vars.X }}\n"),
    );
    red(
        "r4-reusable-other-key",
        &plant("  call:\n    uses: ./.github/workflows/x.yml\n    with:\n      pool: burst-x64\n"),
    );
}

#[test]
fn r4_escaped_secrets_fail() {
    red_why(
        "r4-job-secret",
        &SPOT_JOB.replace(
            "steps: []",
            "env:\n      T: \"${{ s\\x65crets.GH_TOKEN }}\"\n    steps: []",
        ),
        "names secrets",
    );
    red_why(
        "r4-top-secret",
        &real_mut("\nenv:\n", "\nenv:\n  T: \"${{ s\\x65crets.GH_TOKEN }}\"\n"),
        "workflow level names secrets",
    );
    red_why(
        "r4-lockfile-secret",
        &job_mut(
            "lockfile",
            "    env:\n",
            "    env:\n      T: \"${{ s\\x65crets.GH_TOKEN }}\"\n",
        ),
        "names secrets",
    );
}

#[test]
fn r4_anchor_and_merge_and_tags_fail() {
    red(
        "r4-anchor",
        &plant("  a:\n    runs-on: &r [self-hosted, burst-x64]\n    steps: []\n  b:\n    runs-on: *r\n    steps: []\n"),
    );
    red(
        "r4-merge-key",
        &plant("  a:\n    <<: {runs-on: [self-hosted, burst-x64]}\n    steps: []\n"),
    );
    red(
        "r4-core-tag",
        &plant("  a:\n    runs-on: !!seq [self-hosted, burst-x64]\n    steps: []\n"),
    );
    red(
        "r4-custom-tag",
        &plant("  a:\n    runs-on: !pool [self-hosted, x]\n    steps: []\n"),
    );
    red(
        "r4-anchor-secret-into-lockfile",
        &SPOT_JOB
            .replace(
                "on: pull_request\n",
                "on: pull_request\nx: &t ${{ secrets.X }}\n",
            )
            .replace("steps: []", "env:\n      T: *t\n    steps: []"),
    );
}

#[test]
fn r4_cannot_judge_is_red() {
    red("r4-unparseable", "on: pull_request\njobs: [\n");
    red(
        "r4-dup-key",
        &plant("  a:\n    runs-on: [self-hosted]\n    runs-on: [self-hosted]\n    steps: []\n"),
    );
    red("r4-dup-job", &plant("  a:\n    runs-on: [self-hosted]\n    steps: []\n  a:\n    runs-on: [self-hosted]\n    steps: []\n"));
    red("r4-not-mapping", "- a\n- b\n");
    red("r4-jobs-not-mapping", "on: push\njobs: x\n");
    red("r4-no-jobs", "on: push\n");
    red("r4-job-not-mapping", "on: push\njobs:\n  a: x\n");
    red("r4-no-runs-on", &plant("  a:\n    steps: []\n"));
}
