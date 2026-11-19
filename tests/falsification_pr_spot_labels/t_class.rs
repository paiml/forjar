//! Round 8: the classes behind the findings, planted RED first. Case, hosted detection, relays.

use crate::t_yaml::plant;
use crate::*;

const MATRIX_HEAD: &str =
    "  publish:\n    runs-on: ${{ matrix.os }}\n    strategy:\n      matrix:\n";
const SPOT: &str = "[self-hosted, burst-x64, burst-public]";
const CLEAN: &str = "[self-hosted, clean-room, X64]";

fn matrix_job(runs_on: &str, matrix: &str) -> String {
    plant(&format!(
        "{}\n    steps: []\n",
        MATRIX_HEAD.replace("${{ matrix.os }}", runs_on) + matrix
    ))
}

#[test]
fn l1_1_matrix_and_context_names_compare_the_way_github_does() {
    let inc =
        |k: &str| format!("        include:\n          - os: {CLEAN}\n          - {k}: {SPOT}");
    red(
        "c1-include-upper",
        &matrix_job("${{ matrix.os }}", &inc("OS")),
    );
    red(
        "c1-include-mixed",
        &matrix_job("${{ matrix.os }}", &inc("Os")),
    );
    red("c1-ref-upper", &matrix_job("${{ MATRIX.OS }}", &inc("os")));
    red("c1-ref-mixed", &matrix_job("${{ Matrix.Os }}", &inc("oS")));
    red(
        "c1-top-level-key",
        &matrix_job("${{ matrix.os }}", &format!("        Os: [{SPOT}]\n")),
    );
    red(
        "c1-key-collision",
        &matrix_job(
            "${{ matrix.os }}",
            &format!("        os: [{CLEAN}]\n        OS: [{SPOT}]\n"),
        ),
    );
    red(
        "c1-include-key-upper",
        &matrix_job(
            "${{ matrix.os }}",
            &format!("        INCLUDE:\n          - OS: {SPOT}\n"),
        ),
    );
    red(
        "c1-exclude-decoy",
        &matrix_job(
            "${{ matrix.os }}",
            &format!(
                "        target: [decoy]\n        os:\n          - {CLEAN}\n        exclude:\n          - target: decoy\n        include:\n          - target: release\n            OS: {SPOT}\n"
            ),
        ),
    );
}

#[test]
fn l1_1_opt_in_variable_names_compare_case_insensitively() {
    for v in [
        "vars.pr_spot_optin",
        "vars.Burst_Optin",
        "vars['pr_spot_OPTIN']",
    ] {
        red(
            "c1-optin",
            &plant(&format!(
                "  a:\n    runs-on: {CLEAN}\n    steps:\n      - if: ${{{{ {v} == '1' }}}}\n        run: x\n"
            )),
        );
    }
    let ci = real_mut("vars.PR_SPOT_OPTIN", "VARS.pr_spot_optin");
    red("c1-canonical-respelled", &ci);
}

#[test]
fn l1_2_a_self_hosted_job_is_never_hosted_whatever_else_it_lists() {
    for ro in [
        "[self-hosted, ubuntu-24.04]",
        "[ubuntu-24.04, self-hosted]",
        "[self-hosted, ubuntu-latest]",
        "[self-hosted, windows-]",
        "[SELF-HOSTED, macos-14]",
        "ubuntu-public",
        "[ubuntu-latest, burstless]",
        "[ubuntu-x64]",
    ] {
        red(
            "c2-hosted-lookalike",
            &plant(&format!("  a:\n    runs-on: {ro}\n    steps: []\n")),
        );
    }
    for ro in [
        "ubuntu-latest",
        "ubuntu-22.04",
        "ubuntu-24.04-arm",
        "windows-latest",
        "[self-hosted, clean-room, X64]",
        "[self-hosted, clean-room, ubuntu-24.04]",
    ] {
        let ci = plant(&format!("  a:\n    runs-on: {ro}\n    steps: []\n"));
        assert!(
            guard(&scratch("c2-ok", &ci)),
            "a real route was refused: {ro}"
        );
    }
}

const MINT: &str = "\n  mint-creds:\n    runs-on: [self-hosted, clean-room, X64]\n    permissions:\n      contents: write\n      id-token: write\n    outputs:\n      tok: ${{ steps.s.outputs.tok }}\n    steps:\n      - id: s\n        env:\n          T: ${{ secrets.CARGO_REGISTRY_TOKEN }}\n        run: echo \"tok=$(printf %s \"$T\" | base64 -w0)\" >> \"$GITHUB_OUTPUT\"\n";

#[test]
fn l1_3_a_spot_job_takes_nothing_from_a_job_that_holds_secrets_or_write() {
    let perm = "    permissions:\n      contents: read\n";
    for need in ["mint-creds", "[classify, mint-creds]"] {
        let ci = job_mut("lockfile", perm, &format!("    needs: {need}\n{perm}")) + MINT;
        red_why("c3-needs-secret-job", &ci, "needs");
    }
    // a write-permission job with no secrets of its own is just as much a relay
    let write = MINT.replace(
        "          T: ${{ secrets.CARGO_REGISTRY_TOKEN }}\n",
        "          T: x\n",
    );
    let ci = job_mut("lockfile", perm, &format!("    needs: mint-creds\n{perm}")) + &write;
    red_why("c3-needs-write-job", &ci, "needs");
    // transitive: lockfile -> relay -> mint-creds
    let relay = "\n  relay:\n    needs: mint-creds\n    runs-on: [self-hosted, clean-room, X64]\n    steps: []\n";
    let ci = job_mut("lockfile", perm, &format!("    needs: relay\n{perm}")) + MINT + relay;
    red_why("c3-needs-transitive", &ci, "needs");
    // unknown need cannot be judged
    let ci = job_mut("lockfile", perm, &format!("    needs: nope\n{perm}"));
    red_why("c3-needs-unknown", &ci, "needs");
    // artifacts are a relay too
    let step = "      - uses: actions/download-artifact@3d3c42e5aac5ba805825da76410c181273ba90b1\n";
    let ci = job_mut("lockfile", "    steps:\n", &format!("    steps:\n{step}"));
    red_why("c3-artifact", &ci, "artifact");
}
