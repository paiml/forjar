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
        "pr_ticket_aliases_frozen_at: 0000000000000000000000000000000000000000\n",
    ] {
        let fx = fixture(Case {
            aliases: map(frozen, &good_entry()),
            ..Case::default()
        });
        let r = run(&fx, AN_HOUR);
        r.assert_red("a freeze point that names no commit is a ledger nobody can measure");
        r.assert_says("UNMEASURED");
    }
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
    git(&fx.root, &["checkout", "-q", "main"]);
    let r = run(&fx, AN_HOUR);
    r.assert_red("v0.7.7 is not an ancestor of HEAD");
    r.assert_says("pr_ticket_aliases_frozen_at");
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
    for pr in ["0", "-1", "10.5"] {
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

// ---------------------------------------------------------------------------
// The committed map, checked against git (#1238 lane 2, finding 6).

fn here(args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("git must run");
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn yaml(rel: &str) -> serde_yaml_ng::Value {
    let text = std::fs::read_to_string(format!("{}/{rel}", env!("CARGO_MANIFEST_DIR"))).expect(rel);
    serde_yaml_ng::from_str(&text).expect(rel)
}

/// Every entry cites evidence, targets a row, is a squash commit whose own
/// `Pmat-Ticket:` trailers say what the entry says, and the freeze point is the
/// newest of those commits, so it cannot be moved to cover a later PR without
/// a later PR's entry. A CI checkout is shallow (--depth 1) and cannot carry
/// the history; it is MEASURED as shallow rather than skipped.
///
/// Residual limit, stated rather than hidden: the freeze point is data in the
/// same ledger. A change that adds an entry for a PR merged after the freeze
/// AND moves the freeze to that PR's merge commit passes this test, because
/// that is also what a legitimate late entry looks like. What stops it is that
/// releases.yaml is a gate file (a change to it needs two non-author lanes).
#[test]
fn the_committed_alias_map_is_evidenced_row_backed_and_frozen() {
    let ledger = yaml("docs/roadmaps/releases.yaml");
    let roadmap = yaml("docs/roadmaps/roadmap.yaml");
    let rows = roadmap["roadmap"].as_sequence().expect("roadmap rows");
    let frozen = ledger["pr_ticket_aliases_frozen_at"]
        .as_str()
        .expect("the alias map declares pr_ticket_aliases_frozen_at");
    assert!(
        frozen.len() == 40 && frozen.bytes().all(|b| b.is_ascii_hexdigit()),
        "the freeze point is a full commit sha, got {frozen:?}"
    );
    let aliases = ledger["pr_ticket_aliases"].as_sequence().expect("aliases");
    assert!(!aliases.is_empty(), "the alias map is empty");
    let mut seen = std::collections::BTreeSet::new();
    for a in aliases {
        let pr = a["pr"].as_u64().expect("pr is a number");
        let ticket = a["ticket"].as_str().expect("ticket is a string");
        assert!(seen.insert(pr), "PR #{pr} is mapped twice");
        assert!(
            !a["evidence"].as_str().unwrap_or("").trim().is_empty(),
            "PR #{pr}: no evidence"
        );
        assert!(
            rows.iter().any(|r| r["id"].as_str() == Some(ticket)),
            "PR #{pr}: {ticket} is no roadmap row"
        );
    }
    if here(&["rev-parse", "--verify", &format!("{frozen}^{{commit}}")]).is_none() {
        // Exactly one explanation: this checkout does not carry the history.
        assert_eq!(
            here(&["rev-parse", "--is-shallow-repository"]).as_deref(),
            Some("true"),
            "the freeze point {frozen} names no commit in a full clone"
        );
        assert!(here(&["cat-file", "-e", frozen]).is_none());
        return;
    }
    assert!(
        here(&["merge-base", "--is-ancestor", frozen, "HEAD"]).is_some(),
        "the freeze point {frozen} is not history of HEAD"
    );
    // PR number -> squash commit, from the subject's `(#N)` suffix.
    let log = here(&["log", "--format=%H%x09%s", frozen]).expect("git log");
    let commit_of = |pr: u64| {
        log.lines()
            .find(|l| l.ends_with(&format!("(#{pr})")))
            .and_then(|l| l.split('\t').next())
            .unwrap_or_else(|| panic!("PR #{pr} has no squash commit under the freeze point"))
            .to_string()
    };
    let mut newest = false;
    for a in aliases {
        let pr = a["pr"].as_u64().expect("pr");
        let ticket = a["ticket"].as_str().expect("ticket");
        let evidence = a["evidence"].as_str().expect("evidence");
        let oid = commit_of(pr);
        newest |= oid == frozen;
        let msg = here(&["show", "-s", "--format=%B", &oid]).expect("git show");
        let trailers: Vec<&str> = msg
            .lines()
            .filter_map(|l| l.strip_prefix("Pmat-Ticket: "))
            .collect();
        let issue = rows
            .iter()
            .find(|r| r["id"].as_str() == Some(ticket))
            .and_then(|r| r["github_issue"].as_u64());
        // The trailer names the row, or the PMAT id of the row's issue.
        let names = [Some(ticket.to_string()), issue.map(|n| format!("PMAT-{n}"))];
        let hit = names.iter().flatten().find_map(|n| {
            let k = trailers.iter().filter(|t| **t == n.as_str()).count();
            (k > 0).then(|| format!("Pmat-Ticket: {n} ({k} of {})", trailers.len()))
        });
        let want = hit.unwrap_or_else(|| {
            panic!("PR #{pr}: no Pmat-Ticket trailer of {oid} names {ticket}: {trailers:?}")
        });
        assert!(
            evidence.contains(&want),
            "PR #{pr}: the evidence {evidence:?} does not state the measured {want:?}"
        );
        assert!(
            here(&["merge-base", "--is-ancestor", &oid, frozen]).is_some(),
            "PR #{pr} merged after the freeze point"
        );
    }
    assert!(
        newest,
        "the freeze point is not the merge commit of an aliased PR: it was moved"
    );
}
