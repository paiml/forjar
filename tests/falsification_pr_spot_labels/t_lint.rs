//! Falsifiers: the `lint.yml` `file-health` job shares the burst opt-in (forjar#1216).

use crate::*;

fn lint_text() -> String {
    fs::read_to_string(repo().join(".github/workflows/lint.yml")).unwrap()
}

fn lint_mut(from: &str, to: &str) -> String {
    let t = lint_text();
    let m = t.replacen(from, to, 1);
    assert_ne!(t, m, "mutation must apply: {from}");
    m
}

fn with_lint(name: &str, lint: &str) -> PathBuf {
    let d = scratch(name, &ci_text());
    fs::write(d.join("lint.yml"), lint).unwrap();
    d
}

fn lint_red(name: &str, lint: &str, needle: &str) {
    match judge(&with_lint(name, lint), &expected_canary()) {
        Ok(_) => panic!("mutant {name} was accepted"),
        Err(w) => assert!(
            w.iter().any(|x| x.contains(needle)),
            "mutant {name} refused for the wrong reason (want `{needle}`): {w:?}"
        ),
    }
}

#[test]
fn lint_file_health_opt_in_is_accepted() {
    assert!(lint_text().contains("vars.BURST_OPTIN"), "opt-in not wired");
    assert!(guard(&with_lint("lint-ok", &lint_text())));
}

#[test]
fn lint_file_health_off_canonical_value_fails() {
    let m = lint_mut(
        "fromJSON('[\"self-hosted\",\"clean-room\"]') }}",
        "fromJSON('[\"self-hosted\",\"clean-room\",\"X64\"]') }}",
    );
    lint_red("lint-fallback", &m, "canonical");
    let m = lint_mut("vars.BURST_OPTIN == '1'", "vars.PR_SPOT_OPTIN == '1'");
    lint_red("lint-var", &m, "canonical");
}

#[test]
fn lint_file_health_plain_spot_label_fails() {
    let t = lint_text();
    let at = t.find("  file-health:").unwrap();
    let (a, b) = t.split_at(at);
    let ro = b.find("    runs-on:").unwrap();
    let end = b[ro..].find('\n').unwrap() + ro;
    let m = format!(
        "{a}{}    runs-on: [self-hosted, burst-x64]{}",
        &b[..ro],
        &b[end..]
    );
    lint_red("lint-plain", &m, "canonical");
}

#[test]
fn lint_file_health_wider_permissions_or_secrets_fail() {
    let m = lint_mut(
        "      contents: read\n    runs-on: ${{",
        "      contents: write\n    runs-on: ${{",
    );
    lint_red("lint-write", &m, "permissions");
    let m = lint_mut(
        "PR_BASE: ${{ github.event.pull_request.base.sha }}",
        "PR_BASE: ${{ secrets.TOKEN }}",
    );
    lint_red("lint-secret", &m, "secrets");
}

#[test]
fn lint_opt_in_in_another_job_or_file_fails() {
    let m = lint_mut(
        "    runs-on: ${{ matrix.runner }}",
        "    runs-on: ${{ vars.BURST_OPTIN == '1' && 'x' || 'y' }}",
    );
    lint_red("lint-clippy", &m, "opt-in variable");
    let d = scratch("lint-elsewhere", &ci_text());
    fs::write(d.join("other.yml"), lint_text()).unwrap();
    assert!(
        !guard(&d),
        "file-health opt-in accepted in a file the allowlist does not name"
    );
}
