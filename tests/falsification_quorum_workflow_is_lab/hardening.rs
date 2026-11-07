use crate::common::*;

// ---- C58-3 round 4: planted mutants for the remaining workflow invariants ----

/// Workflow lines with comments and blanks removed.
pub(crate) fn yaml_lines(w: &str) -> Vec<String> {
    w.lines()
        .map(strip_comment)
        .filter(|l| !l.trim().is_empty())
        .collect()
}

/// The zero-PR guard is exactly `[ "${#prs[@]}" -eq 0 ]`.
pub(crate) fn zero_pr_guard_is_exact(w: &str) -> bool {
    let guards: Vec<String> = code_lines(w)
        .into_iter()
        .filter(|l| l.contains("${#prs[@]}"))
        .collect();
    guards.len() == 1 && guards[0].contains(r#"[ "${#prs[@]}" -eq 0 ]"#)
}

/// Indented lines directly under a top-level `permissions:` key, or None if it is inline.
pub(crate) fn permission_entries(lines: &[String]) -> Option<Vec<String>> {
    let at = lines.iter().position(|l| l.starts_with("permissions:"))?;
    if lines[at].trim() != "permissions:" {
        return None;
    }
    let mut v: Vec<String> = lines[at + 1..]
        .iter()
        .take_while(|l| l.starts_with(' '))
        .map(|l| l.trim().to_string())
        .collect();
    v.sort();
    Some(v)
}

/// Only `contents: read` and `pull-requests: read` under `permissions:`; no write anywhere.
pub(crate) fn permissions_are_read_only(w: &str) -> bool {
    let lines = yaml_lines(w);
    !lines.iter().any(|l| l.contains("write"))
        && permission_entries(&lines)
            .is_some_and(|p| p == ["contents: read", "pull-requests: read"])
}

/// Every `secrets.` reference is `secrets.GITHUB_TOKEN`.
pub(crate) fn secrets_are_only_github_token(w: &str) -> bool {
    yaml_lines(w).iter().all(|l| {
        l.split("secrets.")
            .skip(1)
            .all(|r| r.starts_with("GITHUB_TOKEN"))
    })
}

pub(crate) fn names_pr_code(t: &str) -> bool {
    ["FETCH_HEAD", "refs/pull", "pull/"]
        .iter()
        .any(|k| t.contains(k))
}

pub(crate) fn changes_tree(t: &str) -> bool {
    [
        "git checkout",
        "git switch",
        "git reset",
        "git merge",
        "git worktree",
    ]
    .iter()
    .any(|k| t.contains(k))
}

/// PR code never becomes the working tree: no checkout/switch/reset of FETCH_HEAD or pull refs,
/// and no `ref:` input other than the default branch.
pub(crate) fn pr_code_never_checked_out(w: &str) -> bool {
    yaml_lines(w).iter().all(|l| {
        let t = l.trim();
        let ref_input = t.starts_with("ref:");
        !(names_pr_code(t) && (changes_tree(t) || ref_input))
            && (!ref_input || t.contains("default_branch"))
    })
}

/// Cron is a single daily schedule: 5 fields, numeric minute and hour, `*` for the rest.
pub(crate) fn cron_is_daily(w: &str) -> bool {
    let crons: Vec<String> = yaml_lines(w)
        .into_iter()
        .filter(|l| l.contains("cron:"))
        .collect();
    if crons.len() != 1 {
        return false;
    }
    let v = crons[0].split("cron:").nth(1).unwrap_or("").trim();
    let v = v.trim_matches('"').trim_matches('\'');
    let f: Vec<&str> = v.split_whitespace().collect();
    f.len() == 5
        && f[0].parse::<u8>().is_ok_and(|m| m < 60)
        && f[1].parse::<u8>().is_ok_and(|h| h < 24)
        && f[2..].iter().all(|x| *x == "*")
}

pub(crate) fn runs_on_is_clean_room(w: &str) -> bool {
    let r: Vec<String> = yaml_lines(w)
        .into_iter()
        .filter(|l| l.trim_start().starts_with("runs-on:"))
        .collect();
    r.len() == 1 && r[0].trim() == "runs-on: [self-hosted, clean-room, ARM64]"
}

/// Every `uses:` is pinned to a `vN...` tag or a 40-hex sha, never a branch.
pub(crate) fn uses_are_pinned(w: &str) -> bool {
    yaml_lines(w)
        .iter()
        .filter_map(|l| l.split("uses:").nth(1))
        .all(|u| {
            let u = u.trim();
            match u.split_once('@') {
                Some((_, r)) => {
                    let sha = r.len() == 40 && r.chars().all(|c| c.is_ascii_hexdigit());
                    let tag =
                        r.starts_with('v') && r[1..].starts_with(|c: char| c.is_ascii_digit());
                    sha || tag
                }
                None => false,
            }
        })
}

#[test]
pub(crate) fn live_workflow_passes_hardening_checks() {
    let w = workflow();
    assert!(zero_pr_guard_is_exact(&w), "zero-PR guard");
    assert!(permissions_are_read_only(&w), "permissions");
    assert!(secrets_are_only_github_token(&w), "secrets");
    assert!(pr_code_never_checked_out(&w), "PR code checked out");
    assert!(cron_is_daily(&w), "cron");
    assert!(runs_on_is_clean_room(&w), "runs-on");
    assert!(uses_are_pinned(&w), "uses pinning");
    assert!(w.contains("actions/checkout@v4"), "checkout still present");
}

#[test]
pub(crate) fn hardening_mutants_are_flagged() {
    let w = workflow();
    let guard = r#"[ "${#prs[@]}" -eq 0 ]"#;
    let cron = r#"cron: "17 3 * * *""#;
    let runs = "runs-on: [self-hosted, clean-room, ARM64]";
    let perm = "  pull-requests: read";
    for (what, m, check) in [
        (
            "guard -le 1",
            w.replace(guard, r#"[ "${#prs[@]}" -le 1 ]"#),
            zero_pr_guard_is_exact as fn(&str) -> bool,
        ),
        (
            "guard -ge 0",
            w.replace(guard, r#"[ "${#prs[@]}" -ge 0 ]"#),
            zero_pr_guard_is_exact,
        ),
        (
            "pr write",
            w.replace(perm, "  pull-requests: write"),
            permissions_are_read_only,
        ),
        (
            "contents write",
            w.replace("  contents: read", "  contents: write"),
            permissions_are_read_only,
        ),
        (
            "write-all",
            w.replace(
                "permissions:\n  contents: read\n  pull-requests: read",
                "permissions: write-all",
            ),
            permissions_are_read_only,
        ),
        (
            "extra perm",
            w.replace(perm, "  pull-requests: read\n  issues: read"),
            permissions_are_read_only,
        ),
        (
            "other secret",
            w.replace("${{ github.token }}", "${{ secrets.PAT }}"),
            secrets_are_only_github_token,
        ),
        (
            "secrets inherit",
            w.replace("INPUT_PR:", "X: ${{ secrets.GH_PAT }}\n          INPUT_PR:"),
            secrets_are_only_github_token,
        ),
        (
            "checkout FETCH_HEAD",
            w.replace(
                "          set -uo pipefail",
                "          set -uo pipefail\n          git checkout FETCH_HEAD",
            ),
            pr_code_never_checked_out,
        ),
        (
            "checkout refs/pull",
            w.replace(
                "          set -uo pipefail",
                "          set -uo pipefail\n          git checkout refs/pull/1/head",
            ),
            pr_code_never_checked_out,
        ),
        (
            "ref input",
            w.replace(
                "          fetch-depth: 0",
                "          fetch-depth: 0\n          ref: refs/pull/1/merge",
            ),
            pr_code_never_checked_out,
        ),
        (
            "cron every minute",
            w.replace(cron, r#"cron: "* * * * *""#),
            cron_is_daily,
        ),
        (
            "cron hourly",
            w.replace(cron, r#"cron: "17 * * * *""#),
            cron_is_daily,
        ),
        (
            "cron sub-daily",
            w.replace(cron, r#"cron: "*/5 3 * * *""#),
            cron_is_daily,
        ),
        (
            "cron weekday",
            w.replace(cron, r#"cron: "17 3 * * 1""#),
            cron_is_daily,
        ),
        (
            "cron 6 fields",
            w.replace(cron, r#"cron: "17 3 * * * *""#),
            cron_is_daily,
        ),
        (
            "second cron",
            w.replace(cron, &format!("{cron}\n    - cron: \"5 4 * * *\"")),
            cron_is_daily,
        ),
        (
            "hosted runner",
            w.replace(runs, "runs-on: ubuntu-latest"),
            runs_on_is_clean_room,
        ),
        (
            "no ARM64",
            w.replace(runs, "runs-on: [self-hosted, clean-room]"),
            runs_on_is_clean_room,
        ),
        (
            "uses @main",
            w.replace("actions/checkout@v4", "actions/checkout@main"),
            uses_are_pinned,
        ),
        (
            "uses @master",
            w.replace("actions/checkout@v4", "actions/checkout@master"),
            uses_are_pinned,
        ),
        (
            "uses unpinned",
            w.replace("actions/checkout@v4", "actions/checkout"),
            uses_are_pinned,
        ),
    ] {
        assert!(w.contains("actions/checkout@v4"));
        assert_ne!(m, w, "mutant did not apply: {what}");
        assert!(!check(&m), "mutant missed: {what}");
    }
}

// ---- exact-content pin: the whole file, so pattern-shaped tests cannot be slipped past ----
