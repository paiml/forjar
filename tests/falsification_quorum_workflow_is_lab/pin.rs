use crate::common::*;
use crate::hardening::*;
use std::path::Path;

pub(crate) const PIN_HELP: &str =
    "changing the LAB workflow .github/workflows/quorum.yml requires updating \
tests/fixtures/quorum-lab-workflow.pinned.yml in the same PR, with a non-author quorum round";

/// Normalise only line endings and trailing whitespace (and trailing blank lines).
pub(crate) fn normalise(w: &str) -> String {
    let mut s: String = w
        .replace("\r\n", "\n")
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");
    s.push('\n');
    while s.ends_with("\n\n") {
        s.pop();
    }
    s
}

pub(crate) fn pinned() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/quorum-lab-workflow.pinned.yml"),
    )
    .expect("pinned fixture")
}

pub(crate) fn matches_pin(w: &str) -> bool {
    normalise(w) == normalise(&pinned())
}

/// Cheap explicit checks so a failure names its cause. Returns the first cause found.
pub(crate) fn explicit_cause(w: &str) -> Option<&'static str> {
    let y = yaml_lines(w);
    let has = |f: &dyn Fn(&str) -> bool| y.iter().any(|l| f(l));
    if has(&|l| l.contains("continue-on-error")) {
        return Some("continue-on-error");
    }
    if has(&|l| {
        let t = l.trim_start().trim_start_matches("- ");
        t.starts_with("if:")
    }) {
        return Some("job/step-level if:");
    }
    if has(&|l| {
        let t = l.trim();
        t == "gate:" || t == "name: gate" || t == "name: 'gate'" || t == "name: \"gate\""
    }) {
        return Some("job id/name gate");
    }
    if has(&|l| l.contains("toJSON(secrets") || l.contains("toJson(secrets")) {
        return Some("toJSON(secrets");
    }
    if !secrets_are_only_github_token(w) {
        return Some("secret other than GITHUB_TOKEN");
    }
    if has(&|l| {
        l.split(|c: char| !c.is_alphanumeric() && c != '_')
            .any(|x| x == "eval")
    }) {
        return Some("eval");
    }
    None
}

#[test]
pub(crate) fn workflow_equals_its_pin_byte_for_byte() {
    let w = workflow();
    assert_eq!(explicit_cause(&w), None, "explicit LAB check failed");
    assert!(matches_pin(&w), "{PIN_HELP}");
}

#[test]
pub(crate) fn pin_and_explicit_checks_flag_planted_mutants() {
    let w = workflow();
    // (name, mutant, explicit cause that must name it, if any)
    let cases: Vec<(&str, String, Option<&str>)> = vec![
        (
            "continue-on-error job",
            w.replace(
                "    name: quorum receipt\n",
                "    name: quorum receipt\n    continue-on-error: true\n",
            ),
            Some("continue-on-error"),
        ),
        (
            "continue-on-error step",
            w.replace(
                "      - name: Verify quorum receipt for each PR\n",
                "      - name: Verify quorum receipt for each PR\n        continue-on-error: true\n",
            ),
            Some("continue-on-error"),
        ),
        (
            "job if",
            w.replace(
                "    name: quorum receipt\n",
                "    name: quorum receipt\n    if: false\n",
            ),
            Some("job/step-level if:"),
        ),
        (
            "step if",
            w.replace(
                "      - name: Verify quorum receipt for each PR\n",
                "      - name: Verify quorum receipt for each PR\n        if: always()\n",
            ),
            Some("job/step-level if:"),
        ),
        (
            "exit 0 after rc",
            w.replace(r#"exit "$rc""#, r#"exit "$rc"; exit 0"#),
            None,
        ),
        (
            "state closed",
            w.replace("--state open", "--state closed"),
            None,
        ),
        (
            "trailing limit 1",
            w.replace("--limit 1000", "--limit 1000 --limit 1"),
            None,
        ),
        (
            "out emptied",
            w.replace("            prs=()\n", "            out=\"\"\n            prs=()\n"),
            None,
        ),
        (
            "QUORUM_STORE env",
            w.replace(
                "          INPUT_PR: ${{ inputs.pr_number }}\n",
                "          INPUT_PR: ${{ inputs.pr_number }}\n          QUORUM_STORE: /tmp/x\n",
            ),
            None,
        ),
        (
            "toJSON(secrets)",
            w.replace("${{ github.token }}", "${{ toJSON(secrets) }}"),
            Some("toJSON(secrets"),
        ),
        (
            "eval gh pr view",
            w.replace(
                "read -r head base < <(gh pr view",
                "eval \"$(gh pr view",
            ),
            Some("eval"),
        ),
        (
            "base=head",
            w.replace(
                "            echo \"== PR #$n head $head\"\n",
                "            base=$head\n            echo \"== PR #$n head $head\"\n",
            ),
            None,
        ),
        (
            "jq headRefOid twice",
            w.replace(
                r#"\(.headRefOid) \(.baseRefOid)"#,
                r#"\(.headRefOid) \(.headRefOid)"#,
            ),
            None,
        ),
        (
            "job name gate",
            w.replace("    name: quorum receipt\n", "    name: gate\n"),
            Some("job id/name gate"),
        ),
        (
            "job id gate",
            w.replace("  receipt:\n", "  gate:\n"),
            Some("job id/name gate"),
        ),
        (
            "other secret",
            w.replace("${{ github.token }}", "${{ secrets.PAT }}"),
            Some("secret other than GITHUB_TOKEN"),
        ),
    ];
    assert!(cases.len() >= 12);
    for (what, m, cause) in cases {
        assert_ne!(m, w, "mutant did not apply: {what}");
        assert!(!matches_pin(&m), "pin missed mutant: {what}");
        if let Some(c) = cause {
            assert_eq!(
                explicit_cause(&m),
                Some(c),
                "explicit check wrong for: {what}"
            );
        }
    }
    // normalisation only forgives line endings and trailing whitespace
    assert!(matches_pin(&w.replace('\n', "  \r\n")));
    assert!(!matches_pin(&w.replace("exit 0", "exit  0")));
}
