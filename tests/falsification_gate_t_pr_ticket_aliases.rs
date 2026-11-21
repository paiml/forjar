//! forjar#1235: gate T and a PR that names no `PMAT-<n>` id at all.
//!
//! PR #944 named its work by GitHub issue (`GH-667`, `forjar#669`), which the
//! gate's id rule does not read, so no `alias:<id>` label can be declared for
//! it: an alias maps a STRAY id, and #944 names none. The ledger therefore
//! carries `pr_ticket_aliases`, a PR number mapped to the row that owns it.
//! The map is read ONLY for a PR that names no id; it never overrides a stray
//! and it never names a row the registry lacks. Once any alias is declared the
//! freeze point is required and must resolve (fail closed, never a default).

#[path = "release_goal_fixture/mod.rs"]
mod fx;
use fx::*;
use std::path::Path;

/// An alias map frozen at `frozen` (a line, or "" for none) with `entries`.
fn map(frozen: &str, entries: &str) -> &'static str {
    Box::leak(format!("{frozen}pr_ticket_aliases:\n{entries}").into_boxed_str())
}

fn entry(pr: &str, ticket: &str, evidence: &str) -> String {
    format!("  - pr: {pr}\n    ticket: {ticket}\n    evidence: {evidence}\n")
}

const FROZEN: &str = "pr_ticket_aliases_frozen_at: v0.0.1\n";

fn good_entry() -> String {
    entry("10", "PMAT-901", "\"squash trailer Pmat-Ticket: PMAT-901\"")
}

fn unticketed(aliases: &'static str) -> Fixture {
    fixture(Case {
        shipped_branch: "the-shipped-work",
        aliases,
        ..Case::default()
    })
}

/// A ledger the shape check refuses: red, and the verdict names the map.
fn assert_shape_error(aliases: &'static str, why: &str) {
    let r = run(&unticketed(aliases), AN_HOUR);
    r.assert_red(why);
    r.assert_says("pr_ticket_aliases");
}

#[test]
fn a_pr_naming_no_ticket_is_red_without_a_declared_alias() {
    let fx = fixture(Case {
        shipped_branch: "the-shipped-work",
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("PR #10 names no roadmap ticket");
    r.assert_says("#10");
    r.assert_says("name no roadmap ticket");
}

#[test]
fn a_declared_alias_to_a_row_tickets_the_pr_and_is_green() {
    let fx = unticketed(map(FROZEN, &good_entry()));
    run(&fx, AN_HOUR).assert_green("PR #10 is mapped to PMAT-901, a row labelled release:v0.0.1");
}

#[test]
fn an_alias_to_a_row_the_registry_lacks_is_still_red() {
    let e = entry("10", "PMAT-777", "\"squash trailer Pmat-Ticket: PMAT-777\"");
    let r = run(&unticketed(map(FROZEN, &e)), AN_HOUR);
    r.assert_red("PMAT-777 is no row, so PR #10 stays unticketed");
    r.assert_says("name no roadmap ticket");
}

#[test]
fn an_alias_never_overrides_a_stray_id_the_pr_names() {
    let fx = fixture(Case {
        shipped_branch: "PMAT-999-misnamed",
        aliases: map(FROZEN, &good_entry()),
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("the branch names PMAT-999, a stray; the alias map does not paper over it");
    r.assert_says("PMAT-999");
}

#[test]
fn a_malformed_alias_entry_is_a_ledger_shape_error() {
    let e = entry("ten", "GH-669", "\"x\"");
    assert_shape_error(
        map(FROZEN, &e),
        "the alias entry is not {pr: number, ticket}",
    );
}

// The alias map is a frozen history, not an open door: a PR merged after the
// freeze point names its ticket or the gate stays red (FORJAR-T-HISTORY).
#[test]
fn an_alias_for_a_pr_merged_after_the_freeze_is_still_red() {
    let fx = unticketed(map("pr_ticket_aliases_frozen_at: v0.0.0\n", &good_entry()));
    let r = run(&fx, AN_HOUR);
    r.assert_red("PR #10 merged after the freeze v0.0.0, so its alias is not read");
    r.assert_says("name no roadmap ticket");
}

#[test]
fn a_freeze_point_the_repository_cannot_resolve_is_unmeasured() {
    let fx = unticketed(map("pr_ticket_aliases_frozen_at: v9.9.9\n", &good_entry()));
    let r = run(&fx, AN_HOUR);
    r.assert_red("the freeze point resolves to no commit");
    r.assert_says("UNMEASURED");
}

// #1238 lane FAIL 1: a missing, null or false freeze point fell back to
// "v0.0.0" in the shape check and to "" in the reader, which skipped the
// freeze for every alias.
#[test]
fn an_alias_map_without_a_freeze_point_fails_closed() {
    for (frozen, what) in [
        ("", "absent"),
        ("pr_ticket_aliases_frozen_at:\n", "null"),
        ("pr_ticket_aliases_frozen_at: false\n", "false"),
        ("pr_ticket_aliases_frozen_at: 7\n", "a number"),
    ] {
        let r = run(&unticketed(map(frozen, &good_entry())), AN_HOUR);
        r.assert_red(&format!(
            "an alias map whose freeze point is {what} reads every PR"
        ));
        r.assert_says("pr_ticket_aliases_frozen_at");
    }
}

// #1238 lane FAIL 2: the freeze point was resolved only when an alias lookup
// reached it, so an unresolvable one passed whenever every PR named a ticket.
#[test]
fn an_unresolvable_freeze_point_fails_even_when_no_alias_is_read() {
    // The default branch names PMAT-901: no PR reaches the alias lookup.
    for frozen in [
        "pr_ticket_aliases_frozen_at: v9.9.9\n",
        "pr_ticket_aliases_frozen_at: deadbeefdeadbeefdeadbeefdeadbeefdeadbeef\n",
    ] {
        let fx = fixture(Case {
            aliases: map(frozen, &good_entry()),
            ..Case::default()
        });
        let r = run(&fx, AN_HOUR);
        r.assert_red("a freeze point that names no commit is a ledger nobody can measure");
        r.assert_says("resolves to no commit");
        r.assert_says("UNMEASURED");
    }
}

#[test]
fn an_unresolvable_freeze_point_fails_even_when_alias_list_is_empty_or_absent() {
    for aliases in [
        "pr_ticket_aliases_frozen_at: deadbeefdeadbeefdeadbeefdeadbeefdeadbeef\npr_ticket_aliases: []\n",
        "pr_ticket_aliases_frozen_at: deadbeefdeadbeefdeadbeefdeadbeefdeadbeef\n",
        "pr_ticket_aliases_frozen_at: v9.9.9\npr_ticket_aliases: []\n",
    ] {
        let fx = fixture(Case {
            aliases,
            ..Case::default()
        });
        let r = run(&fx, AN_HOUR);
        r.assert_red("an unresolvable freeze point is unmeasured even without aliases");
        r.assert_says("resolves to no commit");
        r.assert_says("UNMEASURED");
    }
}

#[test]
fn a_real_40_hex_commit_sha_freeze_point_works_green_and_red() {
    let fx = unticketed(map(FROZEN, &good_entry()));
    let v001_sha = stdout_of(&git(&fx.root, &["rev-parse", "v0.0.1^{commit}"]));
    let v000_sha = stdout_of(&git(&fx.root, &["rev-parse", "v0.0.0^{commit}"]));
    let ledger = read(&fx, "docs/roadmaps/releases.yaml");
    std::fs::write(
        fx.root.join("docs/roadmaps/releases.yaml"),
        ledger.replace(
            "pr_ticket_aliases_frozen_at: v0.0.1",
            &format!("pr_ticket_aliases_frozen_at: {v001_sha}"),
        ),
    )
    .expect("rewrite");
    git(&fx.root, &["commit", "-qam", "freeze at 40-hex v0.0.1 sha"]);
    run(&fx, AN_HOUR).assert_green("PR #10 is an ancestor of the 40-hex freeze sha");

    std::fs::write(
        fx.root.join("docs/roadmaps/releases.yaml"),
        ledger.replace(
            "pr_ticket_aliases_frozen_at: v0.0.1",
            &format!("pr_ticket_aliases_frozen_at: {v000_sha}"),
        ),
    )
    .expect("rewrite");
    git(&fx.root, &["commit", "-qam", "freeze at 40-hex v0.0.0 sha"]);
    let r = run(&fx, AN_HOUR);
    r.assert_red("PR #10 merged after the 40-hex freeze sha");
    r.assert_says("name no roadmap ticket");
}

// #1238 finding 6: the freeze point must be history of HEAD, not a tag or sha
// parked on a side branch.
#[test]
fn a_freeze_point_that_is_not_history_of_head_fails() {
    let fx = unticketed(map("pr_ticket_aliases_frozen_at: v0.7.7\n", &good_entry()));
    git(&fx.root, &["checkout", "-q", "--detach", "v0.0.1"]);
    git(
        &fx.root,
        &["commit", "-q", "--allow-empty", "-m", "a side commit"],
    );
    git(&fx.root, &["tag", "v0.7.7"]);
    git(&fx.root, &["push", "-q", "origin", "v0.7.7"]);
    git(&fx.root, &["checkout", "-q", "main"]);
    let r = run(&fx, AN_HOUR);
    r.assert_red("v0.7.7 is not an ancestor of HEAD");
    r.assert_says("pr_ticket_aliases_frozen_at");
}

#[test]
fn a_local_branch_or_unpushed_tag_named_like_a_release_cannot_freeze() {
    let fx = unticketed(map("pr_ticket_aliases_frozen_at: v9.9.9\n", &good_entry()));
    git(&fx.root, &["branch", "v9.9.9", "HEAD"]);
    let r = run(&fx, AN_HOUR);
    r.assert_red("a local branch named v9.9.9 is not a tag");
    r.assert_says("resolves to no commit");

    git(&fx.root, &["branch", "-D", "v9.9.9"]);
    git(&fx.root, &["tag", "-a", "-m", "local", "v9.9.9", "HEAD"]);
    let r = run(&fx, AN_HOUR);
    r.assert_red("a local tag v9.9.9 not on origin cannot freeze aliases");
    r.assert_says("origin");
}

#[test]
fn an_alias_entry_with_a_blank_evidence_is_a_ledger_shape_error() {
    for ev in ["\"\"", "\"   \"", "' '"] {
        let e = entry("10", "PMAT-901", ev);
        assert_shape_error(map(FROZEN, &e), &format!("evidence {ev} cites nothing"));
    }
}

// #1238 lane FAIL 4: `// []` turned false and null into an empty map, the pr
// check took 0, -1 and 10.5, and a duplicate pr silently let the first win.
#[test]
fn a_malformed_alias_map_is_a_ledger_shape_error() {
    for text in [
        "pr_ticket_aliases: false\n",
        "pr_ticket_aliases:\n",
        "pr_ticket_aliases: 3\n",
    ] {
        let t: &'static str = Box::leak(format!("{FROZEN}{text}").into_boxed_str());
        assert_shape_error(t, &format!("{text:?} is not a list of entries"));
    }
    for pr in ["0", "-1", "10.0", "10.5"] {
        let e = entry(pr, "PMAT-901", "\"squash trailer Pmat-Ticket: PMAT-901\"");
        assert_shape_error(map(FROZEN, &e), &format!("pr: {pr} is not a PR number"));
    }
    let dup = format!(
        "{}{}",
        good_entry(),
        entry("10", "PMAT-902", "\"squash trailer Pmat-Ticket: PMAT-902\"")
    );
    assert_shape_error(
        map(FROZEN, &dup),
        "PR #10 is mapped twice: first-wins hides it",
    );
}

// #1238 lane FAIL 5: window.sh read only `PMAT-` row ids, so a GH-<n> alias
// target could never resolve and the test of it asserted only "not a shape
// error". Here the ledger itself names GH-881, so green means it resolved.
#[test]
fn an_alias_may_name_a_gh_row() {
    let fx = fixture(Case {
        shipped_branch: "the-shipped-work",
        rows: vec![
            ("GH-881", vec!["release:v0.0.1"]),
            (OPEN, vec!["release:v0.0.2"]),
        ],
        aliases: map(
            FROZEN,
            &entry(
                "10",
                "GH-881",
                "\"squash trailer Pmat-Ticket: GH-881 (1 of 1)\"",
            ),
        ),
        ..Case::default()
    });
    let ledger =
        std::fs::read_to_string(fx.root.join("docs/roadmaps/releases.yaml")).expect("ledger");
    std::fs::write(
        fx.root.join("docs/roadmaps/releases.yaml"),
        ledger.replace("tickets: [PMAT-901]", "tickets: [GH-881]"),
    )
    .expect("rewrite");
    git(&fx.root, &["commit", "-qam", "the floor ships GH-881"]);
    run(&fx, AN_HOUR)
        .assert_green("PR #10 resolves to GH-881, the row the ledger declares for v0.0.1");
}

#[test]
fn window_sh_without_releases_sh_does_not_abort_on_unticketed_pr() {
    let fx = fixture(Case {
        shipped_branch: "the-shipped-work",
        ..Case::default()
    });
    let oid = stdout_of(&git(&fx.root, &["rev-parse", "v0.0.1^{commit}"]));
    let cmd = format!(
        "set -euo pipefail; fail() {{ echo \"FAIL $1\"; exit 1; }}; \
         . scripts/dogfood/lib/window.sh; \
         DOGFOOD_PR_JSON='[{{\"number\":10,\"mergedAt\":\"2026-01-02T00:00:00Z\",\"mergeCommit\":{{\"oid\":\"{oid}\"}},\"headRefName\":\"no-ticket\",\"title\":\"no ticket\",\"body\":\"\"}}]'; \
         DOGFOOD_PR_COUNT=1; \
         dogfood_window_tickets; \
         echo \"UNTICKETED=$DOGFOOD_WINDOW_UNTICKETED\""
    );
    let out = std::process::Command::new("bash")
        .args(["-c", &cmd])
        .current_dir(&fx.root)
        .output()
        .expect("bash must run");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.success(),
        "window.sh without releases.sh must not abort:\n{text}"
    );
    assert_eq!(text.trim(), "UNTICKETED=#10");
}

// ---------------------------------------------------------------------------
// The committed map, checked against git (#1238 lane 2, finding 6).

/// Every entry cites evidence, targets a row, has plurality among its squash
/// commit's `Pmat-Ticket:` trailers, and the freeze point is the newest of
/// those commits. Fails closed when history is missing (CI runs with fetch-depth: 0).
#[test]
fn the_committed_alias_map_is_evidenced_row_backed_and_frozen() {
    check_alias_map(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn the_alias_map_checker_rejects_missing_freeze_post_freeze_ambiguous_and_minority() {
    let dir = tempfile::tempdir().expect("tempdir");
    let r = dir.path().join("repo");
    let nohooks = dir.path().join("nohooks");
    std::fs::create_dir_all(&r).expect("mkdir repo");
    std::fs::create_dir_all(&nohooks).expect("mkdir nohooks");
    git(&r, &["init", "-q", "-b", "main"]);
    git(
        &r,
        &["config", "core.hooksPath", &nohooks.to_string_lossy()],
    );
    git(&r, &["config", "commit.gpgsign", "false"]);
    write(
        &r,
        "docs/roadmaps/roadmap.yaml",
        &roadmap(&[("PMAT-901", &[]), ("PMAT-902", &[])]),
    );
    git(&r, &["add", "-A"]);
    git(
        &r,
        &[
            "commit",
            "-qm",
            "work (#10)\n\nPmat-Ticket: PMAT-901\nPmat-Ticket: PMAT-902\nPmat-Ticket: PMAT-902\n",
        ],
    );
    let c1 = stdout_of(&git(&r, &["rev-parse", "HEAD"]));
    git(
        &r,
        &[
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "later (#11)\n\nPmat-Ticket: PMAT-902\n",
        ],
    );
    let c2 = stdout_of(&git(&r, &["rev-parse", "HEAD"]));

    write(
        &r,
        "docs/roadmaps/releases.yaml",
        "pr_ticket_aliases_frozen_at: deadbeefdeadbeefdeadbeefdeadbeefdeadbeef\npr_ticket_aliases:\n  - pr: 10\n    ticket: PMAT-902\n    evidence: \"Pmat-Ticket: PMAT-902 (2 of 3)\"\n",
    );
    assert!(check_alias_map(&r)
        .unwrap_err()
        .contains("resolves to no commit"));

    write(
        &r,
        "docs/roadmaps/releases.yaml",
        &format!(
            "pr_ticket_aliases_frozen_at: {c1}\npr_ticket_aliases:\n  - pr: 10\n    ticket: PMAT-901\n    evidence: \"Pmat-Ticket: PMAT-901 (1 of 3)\"\n"
        ),
    );
    assert!(check_alias_map(&r)
        .unwrap_err()
        .contains("less than plurality"));

    write(
        &r,
        "docs/roadmaps/releases.yaml",
        &format!(
            "pr_ticket_aliases_frozen_at: {c1}\npr_ticket_aliases:\n  - pr: 10\n    ticket: PMAT-902\n    evidence: \"Pmat-Ticket: PMAT-902 (2 of 3)\"\n  - pr: 11\n    ticket: PMAT-902\n    evidence: \"Pmat-Ticket: PMAT-902 (1 of 1)\"\n"
        ),
    );
    assert!(check_alias_map(&r)
        .unwrap_err()
        .contains("merged after the freeze point"));

    git(
        &r,
        &[
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "dup (#10)\n\nPmat-Ticket: PMAT-902\n",
        ],
    );
    write(
        &r,
        "docs/roadmaps/releases.yaml",
        &format!(
            "pr_ticket_aliases_frozen_at: {c2}\npr_ticket_aliases:\n  - pr: 10\n    ticket: PMAT-902\n    evidence: \"Pmat-Ticket: PMAT-902 (2 of 3)\"\n"
        ),
    );
    assert!(check_alias_map(&r)
        .unwrap_err()
        .contains("must match one first-parent commit"));
}
