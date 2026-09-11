//! PMAT-225 (forjar#506): gate T — every tagged release is declared, every
//! ticket names the tag that shipped it, and the next cut is on time.
//!
//! The gate joins a DECLARED side (`docs/roadmaps/releases.yaml` and the
//! `release:<tag>` labels on `docs/roadmaps/roadmap.yaml` rows) against a
//! MEASURED one (git's tags and their creation instants, the PRs a stubbed
//! `gh` reports merged, placed by ancestry, and a clock the test pins with
//! `DOGFOOD_NOW`). Every case below drives the REAL script over a temp
//! repository with a bare `origin`, changes ONE thing, and asserts the gate is
//! red for that reason by name — or green. A gate that exited 1 without its
//! `GATE T FAIL` line is a death, not a verdict, and is a failure here too.

#[path = "release_goal_fixture/mod.rs"]
mod fx;
use fx::*;

#[test]
fn a_declared_ledger_that_matches_git_and_github_is_green_and_counts() {
    let fx = fixture(Case::default());
    let r = run(&fx, AN_HOUR);
    r.assert_green("the ledger, the labels and the window agree");
    r.assert_says("1 tagged release(s) since v0.0.1");
    r.assert_says("1 ticket(s) from 1 PR(s) merged since v0.0.1 carry release:v0.0.2");
    r.assert_says("47h left");
}

#[test]
fn a_declared_window_that_disagrees_with_github_is_named_and_red() {
    let fx = fixture(Case {
        declared_prs: "[10, 12]",
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("the ledger declares PR #12, which GitHub does not report in v0.0.1's window");
    r.assert_says("v0.0.1");
    r.assert_says("declared window is not the measured one");
}

#[test]
fn a_merged_ticket_without_the_next_release_label_is_named_and_red() {
    let fx = fixture(Case {
        rows: vec![(SHIPPED, vec!["release:v0.0.1"]), (OPEN, vec![])],
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("PMAT-902 merged since v0.0.1 and carries no release:v0.0.2");
    r.assert_says(OPEN);
    r.assert_says("release:v0.0.2");
}

#[test]
fn a_shipped_ticket_without_its_release_label_is_named_and_red() {
    let fx = fixture(Case {
        rows: vec![(SHIPPED, vec![]), (OPEN, vec!["release:v0.0.2"])],
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("PMAT-901 shipped in v0.0.1 and carries no release:v0.0.1");
    r.assert_says(SHIPPED);
    r.assert_says("release:v0.0.1");
}

#[test]
fn a_ticket_claiming_a_release_it_was_not_in_is_red() {
    let fx = fixture(Case {
        rows: vec![
            (SHIPPED, vec!["release:v0.0.1"]),
            (OPEN, vec!["release:v0.0.2"]),
            ("PMAT-903", vec!["release:v0.0.1"]),
        ],
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("PMAT-903 claims v0.0.1 and no PR of that window names it");
    r.assert_says("PMAT-903");
    r.assert_says("fabricated");
}

#[test]
fn a_row_naming_a_tag_origin_does_not_carry_is_red() {
    let fx = fixture(Case {
        extra_row: "  - tag: v0.0.5\n    cut: 2026-01-01T00:00:00Z\n    prs: []\n    tickets: []\n",
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("the ledger declares v0.0.5, a release that does not exist");
    r.assert_says("v0.0.5");
}

#[test]
fn a_declared_due_that_is_not_cut_plus_cadence_is_red() {
    let fx = fixture(Case {
        due_skew: 1,
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("next.due is one second off the derived instant");
    r.assert_says("derived one disagree");
}

#[test]
fn an_overdue_cut_is_red_until_the_version_is_bumped() {
    let fx = fixture(Case::default());
    let r = run(&fx, 2 * 86400 + 3 * AN_HOUR);
    r.assert_red("three hours past due with a PR merged and the version unbumped");
    r.assert_says("OVERDUE by 3h");

    let bumped = fixture(Case {
        version: "0.0.2",
        ..Case::default()
    });
    let r = run(&bumped, 2 * 86400 + 3 * AN_HOUR);
    r.assert_green("Cargo.toml is at the next version: the cut is in flight");
    r.assert_says("cut in flight");
}

#[test]
fn a_tree_at_a_version_no_goal_names_is_red() {
    let fx = fixture(Case {
        version: "0.0.7",
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("0.0.7 is neither the newest tag nor the declared next");
    r.assert_says("0.0.7");
}

#[test]
fn a_release_at_the_dogfood_floor_without_its_receipt_is_red() {
    let fx = fixture(Case {
        receipts: false,
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("v0.0.1 is at the dogfood floor and its receipt is not at HEAD");
    r.assert_says("docs/audits/dogfood-0.0.1-receipt.md");

    let with = fixture(Case::default());
    with.assert_committed("docs/audits/dogfood-0.0.1-receipt.md");
    run(&with, AN_HOUR).assert_green("the receipt and the crux document are at HEAD");
}

#[test]
fn a_stray_id_in_a_tagged_window_is_red_until_a_row_declares_the_alias() {
    let fx = fixture(Case {
        shipped_branch: "PMAT-999-misnamed",
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("PR #10's branch names PMAT-999, which is no row and no declared alias");
    r.assert_says("PMAT-999");
    r.assert_says("alias");

    let declared = fixture(Case {
        shipped_branch: "PMAT-999-misnamed",
        rows: vec![
            (SHIPPED, vec!["release:v0.0.1", "alias:PMAT-999"]),
            (OPEN, vec!["release:v0.0.2"]),
        ],
        ..Case::default()
    });
    run(&declared, AN_HOUR).assert_green("PMAT-901 declares alias:PMAT-999 and carries the label");
}

#[test]
fn a_gh_that_cannot_answer_is_unmeasured_and_red() {
    let fx = fixture(Case::default());
    let r = run_at(&fx, AN_HOUR, "false");
    r.assert_red("gh exited 1: the window is unmeasured");
    r.assert_says("UNMEASURED");
}

/// PMAT-229: the status line renders what it can and says what it cannot.
///
/// `release-goal.sh show` shares gate T's window, and the window refuses when
/// commits reach HEAD that no merged PR contains — which is every feature
/// branch, and therefore every place an operator actually reads the cadence.
/// `make release-goal` printed no goal at all there: not the tag, not the due
/// instant, not the elapsed bar, none of which depend on the unmeasured
/// commits. Only the merged count does, and it is now printed as UNMEASURED.
///
/// The exit code stays non-zero, so nothing can read the degraded line as a
/// measured one, and `scripts/dogfood/tagged.sh` is untouched: a release gate
/// that rendered this would be a gate passing on an unmeasured window, which
/// is the case directly below this one in the same fixture.
#[test]
fn the_status_line_renders_the_goal_when_the_merged_count_is_unmeasurable() {
    // The state right after a tag: the previous release's own PR is all
    // GitHub reports, so the window is empty while the declaration commit
    // sits in it. That is what a branch looks like to this code, and it is
    // exactly the case that fired on PMAT-227's booking branch.
    let mut fx = fixture(Case::default());
    let shipped = stdout_of(&git(&fx.root, &["rev-parse", "v0.0.1^{commit}"]));
    fx.gh = stub_gh(
        fx._dir.path(),
        &format!(
            r#"[{{"number":10,"mergedAt":"2026-01-02T00:00:00Z","mergeCommit":{{"oid":"{shipped}"}},"headRefName":"PMAT-901-the-shipped-work","title":"the shipped work","body":""}}]"#
        ),
    );
    let t = tool(&fx, AN_HOUR, &["show"]);
    assert_ne!(
        t.code, 0,
        "PMAT-229: an unmeasurable window must still leave a non-zero exit, so \
         no script reads a degraded line as a pass:\n{}",
        t.text
    );
    assert!(
        t.text.contains(NEXT) && t.text.contains("due ") && t.text.contains("basis="),
        "PMAT-229: the goal line — the next tag, the due instant, the basis — \
         does not depend on the commits that could not be measured, and must be \
         printed:\n{}",
        t.text
    );
    assert!(
        t.text.contains("UNMEASURED"),
        "PMAT-229: the counts that could not be measured must say so rather than \
         print a number:\n{}",
        t.text
    );
    // The gate over the same window is unchanged and still refuses outright.
    run(&fx, AN_HOUR).assert_red("gate T never renders a degraded line");
}

/// PMAT-229: the soft switch cannot be set from outside the tool.
///
/// The first version read `${DOGFOOD_WINDOW_SOFT:-0}` from the environment,
/// which any operator could export and every gate would inherit — a release
/// gate softened from a shell profile. It is the third argument now, and this
/// case runs the gate with that name exported to prove the gate cannot see it.
#[test]
fn no_environment_variable_can_soften_the_gate() {
    let fx = fixture(Case::default());
    let out = std::process::Command::new("bash")
        .arg(fx.root.join("scripts/dogfood/tagged.sh"))
        .current_dir(&fx.root)
        .env("GH", &fx.gh)
        .env("DOGFOOD_NOW", (fx.cut + AN_HOUR).to_string())
        .env("DOGFOOD_WINDOW_SOFT", "1")
        .env("SOFT", "soft")
        .output()
        .expect("bash must run");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !text.contains("UNMEASURED counts") && !text.contains("merged=UNMEASURED"),
        "PMAT-229: the gate rendered a degraded line because the environment \
         asked it to:\n{text}"
    );
    assert!(
        text.contains("GATE T PASS") || text.contains("GATE T FAIL"),
        "the gate must still reach a verdict:\n{text}"
    );
}

/// PMAT-236: a ticket a release names must say it shipped.
///
/// T2 and T4 reconcile the ledger and the `release:<tag>` labels, and neither
/// looked at `status`. So the roadmap said none of the 1.28.0 work had started
/// on the day 1.28.0 shipped — sixteen tickets across five releases read
/// `planned` or `inprogress` while their labels were correct, and nothing went
/// red. The label is the link a release needs; the status is what a person
/// reads, and a record half true is the kind that is trusted until it matters.
#[test]
fn a_shipped_ticket_whose_row_says_planned_is_named_and_red() {
    let shipped_still_planned = fixture(Case {
        rows: vec![
            (concat!("planned:", "PMAT-901"), vec!["release:v0.0.1"]),
            (OPEN, vec!["release:v0.0.2"]),
        ],
        ..Case::default()
    });
    let r = run(&shipped_still_planned, AN_HOUR);
    r.assert_red("PMAT-901 shipped in v0.0.1 and its row still reads planned");
    r.assert_says("PMAT-901");
    r.assert_says("status: planned");
    r.assert_says("pmat work edit");
}

/// The same arm reaches the OPEN window: a ticket whose PR has merged has
/// landed, whether or not a tag has been cut over it. This is the case that
/// catches the drift as it happens rather than five releases later.
#[test]
fn a_merged_ticket_whose_row_says_planned_is_named_and_red() {
    let merged_still_planned = fixture(Case {
        rows: vec![
            (SHIPPED, vec!["release:v0.0.1"]),
            (concat!("planned:", "PMAT-902"), vec!["release:v0.0.2"]),
        ],
        ..Case::default()
    });
    let r = run(&merged_still_planned, AN_HOUR);
    r.assert_red("PMAT-902 merged since v0.0.1 and its row still reads planned");
    r.assert_says("PMAT-902");
    r.assert_says("merged since v0.0.1");
}

/// PMAT-239: the same tree, the same verdict, every time. A `printf … | grep
/// -q` had grep close the pipe at the first match, printf take SIGPIPE, and
/// `pipefail` report 141 — so the gate called a readable registry UNMEASURED
/// at random. Ten runs is not a proof of determinism, but one flake in three
/// was how this was found.
#[test]
fn the_same_tree_gives_the_same_verdict_every_time() {
    let fx = fixture(Case::default());
    let first = run(&fx, AN_HOUR);
    first.assert_green("the declared ledger matches");
    for _ in 0..9 {
        let again = run(&fx, AN_HOUR);
        assert_eq!(
            again.code, first.code,
            "PMAT-239: the gate gave a different exit code on the same tree:\n{}",
            again.text
        );
        assert!(
            !again.text.contains("exited 141"),
            "PMAT-239: a pipeline returned 141 (SIGPIPE) and the gate called a \
             readable registry UNMEASURED:\n{}",
            again.text
        );
    }
}

/// PMAT-239 / PMAT-240: the pattern, not the instance.
///
/// Ten runs of a gate cannot distinguish a fix from luck — a review lane put
/// the odds at 1.7% for a flake that showed one time in three, and it is right.
/// This is the deterministic half: the three files this branch owns carry no
/// pipeline whose right-hand side can exit before its left-hand side finishes,
/// so none of them can return 141 under `set -o pipefail`.
///
/// Eighteen more such pipelines exist elsewhere under `scripts/`; they are
/// filed as PMAT-240 with the census committed, and this rule is deliberately
/// scoped to what this branch fixed rather than made red on work it did not do.
#[test]
fn the_release_goal_scripts_carry_no_pipeline_that_can_take_sigpipe() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let owned = [
        "scripts/dogfood/lib/window.sh",
        "scripts/dogfood/tagged.sh",
        "scripts/release-goal.sh",
    ];
    let early = ["grep -q", "grep -m", "head", "jq -e"];
    for rel in owned {
        let text =
            std::fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"));
        for (n, line) in text.lines().enumerate() {
            if line.trim_start().starts_with('#') {
                continue;
            }
            let Some((_, rhs)) = line.split_once('|') else {
                continue;
            };
            // `||` is a shell operator, not a pipeline.
            if rhs.starts_with('|') {
                continue;
            }
            for e in early {
                assert!(
                    !rhs.trim_start().starts_with(e),
                    "PMAT-239: {rel}:{} pipes into `{e}`, which exits before its \
                     left-hand side finishes. Under `set -o pipefail` the left side \
                     takes SIGPIPE and the pipeline returns 141, so a gate reports \
                     UNMEASURED at random — measured on this very gate. Capture and \
                     use a here-string, or do it in one process:\n    {line}",
                    n + 1
                );
            }
        }
    }
}

/// PMAT-241: a release at or above `cookbook_floor` names the cookbook it was
/// qualified against.
///
/// The cookbook is where forjar is USED rather than described — gate D
/// validates every one of its configs against the built artifact, and
/// `make dogfood-published VERSION=x.y.z` does it against what crates.io
/// actually serves. Nothing recorded WHICH cookbook that was, and
/// paiml/forjar-cookbook's master had not moved since 2026-08-29 while four
/// tags went out claiming to be dogfooded against it.
#[test]
fn a_release_at_the_cookbook_floor_without_its_cookbook_commit_is_red() {
    let fx = fixture(Case {
        cookbook_floor: FLOOR,
        cookbook: "",
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("v0.0.1 is at the cookbook floor and its row names no cookbook commit");
    r.assert_says("cookbook");
    r.assert_says("ls-remote");
}

/// A branch name is not a commit: a branch moves, and a record of what a
/// release was qualified against must not.
#[test]
fn a_cookbook_field_that_is_not_a_commit_sha_is_red() {
    let fx = fixture(Case {
        cookbook_floor: FLOOR,
        cookbook: "master",
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("`master` is a branch, not the commit the release was qualified against");
    r.assert_says("not a commit sha");
}

/// A commit the cookbook does not carry is UNMEASURED, not a pass: the
/// fixture's stubbed `gh` cannot answer a contents request, which is exactly
/// what an unreachable GitHub looks like.
#[test]
fn a_cookbook_commit_that_cannot_be_read_is_unmeasured_and_red() {
    let fx = fixture(Case {
        cookbook_floor: FLOOR,
        cookbook: "0123456789abcdef0123456789abcdef01234567",
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("the cookbook commit cannot be read, so the claim is unmeasured");
    r.assert_says("UNMEASURED");
}

/// And nothing fires below the floor: the four releases whose cookbook commit
/// nobody recorded are left alone rather than retrofitted with a record that
/// was never taken.
#[test]
fn a_release_below_the_cookbook_floor_is_not_asked_for_one() {
    let fx = fixture(Case {
        cookbook_floor: "v9.9.9",
        cookbook: "",
        ..Case::default()
    });
    run(&fx, AN_HOUR).assert_green("v0.0.1 is below the cookbook floor");
}
