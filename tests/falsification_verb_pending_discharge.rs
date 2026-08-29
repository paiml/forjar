//! Three capabilities left the debt ledger (paiml/forjar#356).
//!
//! `src/verb/partition.rs` accounts for all 193 CLI leaves. `Bucket::Pending`
//! is its debt ledger — "belongs on the unified surface, is not there yet" —
//! and the module says the ledger MAY ONLY SHRINK. Nothing enforced that
//! direction, and nothing asserted that a row leaving it arrived anywhere: a
//! leaf could be flipped to `Unified` while the verb registry stayed as it was,
//! and only `unified_bucket_matches_the_verb_registry` (a lib test comparing
//! two in-process tables) would notice.
//!
//! These three shipped as CLI leaves and nowhere else:
//!
//! | leaf | what it computes | who could read it |
//! |---|---|---|
//! | `audit` | the append-only provenance trail | a human at a terminal |
//! | `policy-coverage` | which resources policy rules cover | a human at a terminal |
//! | `workspace list`/`current` | which isolated state dir is selected | a human at a terminal |
//!
//! Each is a projection of a calculation that already existed, so none of this
//! is new behaviour — it is the same answer, reachable by the callers forjar
//! claims to serve. The tests below assert the reachability, not the arithmetic
//! (that is covered where the calculations live).
//!
//! The `workspace` verb's own tests live in
//! `falsification_verb_workspace_report.rs` — split off for the 500-line file
//! cap, and because what that verb reports needed claims of its own retracted.
//!
//! Usage: cargo test --test falsification_verb_pending_discharge

use forjar::verb::{find, partition, Bucket};

#[path = "common/verb_pending_fixtures.rs"]
mod fixtures;
use fixtures::{audited_project, call, policy_project};

// ── the ledger shrank, and the rows landed somewhere ────────────────

/// REJECTION CRITERION: a discharged leaf still sitting in `Bucket::Pending`.
#[test]
fn the_discharged_leaves_left_the_debt_ledger() {
    for leaf in [
        vec!["audit"],
        vec!["policy-coverage"],
        vec!["workspace", "current"],
        vec!["workspace", "list"],
    ] {
        let row = partition()
            .iter()
            .find(|l| l.path == leaf.as_slice())
            .unwrap_or_else(|| panic!("`{}` has no row in the partition", leaf.join(" ")));
        assert_eq!(
            row.bucket,
            Bucket::Unified,
            "`{}` is not on the unified surface: {:?}",
            leaf.join(" "),
            row.bucket
        );
    }
}

/// The mutating half of `workspace` deliberately did NOT move. This is the
/// guard against "discharge the ledger" being read as "move every row".
#[test]
fn the_mutating_workspace_leaves_did_not_move() {
    for leaf in [
        vec!["workspace", "new"],
        vec!["workspace", "select"],
        vec!["workspace", "delete"],
    ] {
        let row = partition()
            .iter()
            .find(|l| l.path == leaf.as_slice())
            .unwrap_or_else(|| panic!("`{}` has no row", leaf.join(" ")));
        assert!(
            matches!(row.bucket, Bucket::Pending(_)),
            "`{}` writes to disk; putting it on a surface that publishes \
             readOnlyHint: true would make the hint a lie: {:?}",
            leaf.join(" "),
            row.bucket
        );
    }
}

/// A hyphenated CLI leaf must still render a snake_case MCP tool name. Every
/// tool shipped before this is snake_case, and `forjar_policy-coverage` beside
/// `forjar_policy_install` would be a spelling a client has to special-case.
#[test]
fn a_hyphenated_verb_gets_a_snake_case_mcp_name() {
    let v = find("policy-coverage").expect("policy-coverage is a verb");
    assert_eq!(v.mcp_name(), "forjar_policy_coverage");
}

/// The fold is only safe while it cannot collide. `policy-coverage` and a
/// hypothetical `policy_coverage` would render ONE MCP name for two verbs, and
/// `tools/call` would dispatch whichever `register_all` wrote last — silently,
/// with `tools/list` still advertising both.
#[test]
fn the_hyphen_fold_cannot_collide() {
    let all = forjar::verb::verbs();
    let mut names: Vec<String> = all.iter().map(|v| v.mcp_name()).collect();
    names.sort();
    let before = names.len();
    names.dedup();
    assert_eq!(
        before,
        names.len(),
        "two verbs render the same MCP tool name"
    );

    // The property that makes the fold one-way: a verb name is a CLI leaf, so
    // it never contains the character the fold produces.
    for v in &all {
        assert!(
            v.name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "`{}` is not a CLI leaf name, so `forjar_{}` is not a derivation",
            v.name,
            v.name
        );
    }
}

/// `tools/list` and `tools/call` must agree. Handler registration is generic
/// over each handler's type so it cannot be a loop over the verb table — which
/// makes "advertised but not dispatchable" a live failure mode, and the one a
/// client cannot recover from because it reads the list and believes it.
#[test]
fn every_discharged_verb_is_dispatchable() {
    let reg = forjar::mcp::build_registry();
    for name in ["audit", "policy-coverage", "workspace"] {
        let v = find(name).unwrap_or_else(|| panic!("`{name}` is not a verb"));
        assert!(
            reg.has_handler(&v.mcp_name()),
            "`{}` is advertised by tools/list but tools/call cannot dispatch it",
            v.mcp_name()
        );
    }
}

// ── policy-coverage ─────────────────────────────────────────────────

#[test]
fn policy_coverage_names_the_resources_no_policy_covers() {
    let d = tempfile::tempdir().unwrap();
    let cfg = policy_project(d.path());

    let out = call(
        "policy-coverage",
        serde_json::json!({ "path": cfg.display().to_string() }),
    )
    .expect("policy-coverage runs");

    assert_eq!(out["total_resources"], 2);
    assert_eq!(out["covered_resources"], 1);
    assert_eq!(out["fully_covered"], false);
    assert_eq!(
        out["uncovered"],
        serde_json::json!(["pkg"]),
        "the package resource is matched by no policy, and saying which one is \
         uncovered is the entire point of the report: {out}"
    );
    assert_eq!(out["by_type"]["require"], 1);
    assert_eq!(out["compliance_frameworks"]["soc2"], 1);

    // COVERED is not CLEAN. `conf` is the covered resource and it VIOLATES
    // (it has no owner); `pkg` is the clean one and no rule scopes to it. Both
    // halves report "1 of 2" and they mean opposite resources — which is the
    // divergence that shipped when the verb and the CLI leaf were two
    // calculations (#356). One document has to carry both or a reader cannot
    // tell them apart.
    assert_eq!(out["clean_resources"], 1, "{out}");
    assert_eq!(out["rules_triggered"], 1, "{out}");
    assert_eq!(out["untriggered_rules"], serde_json::json!([]), "{out}");
}

/// REJECTION CRITERION: the verb and the CLI leaf answering differently.
///
/// `src/mcp/types_ops.rs` claims the verb returns "the same projection `forjar
/// policy-coverage --json` prints". Both clauses were false when this branch
/// was reviewed: the verb was wired to `core::policy_coverage`, which had NO
/// production caller, while the leaf routed to a different implementation in
/// `src/cli/policy_coverage.rs`. Nothing compared them, so nothing noticed.
///
/// This compares them — against the REAL BINARY, not against a second
/// in-process call, because a shared library function proves the library agrees
/// with itself and says nothing about what the command prints.
#[test]
fn the_verb_and_the_cli_leaf_print_the_same_document() {
    let d = tempfile::tempdir().unwrap();
    let cfg = policy_project(d.path());

    let from_verb = call(
        "policy-coverage",
        serde_json::json!({ "path": cfg.display().to_string() }),
    )
    .expect("policy-coverage runs");

    let run = std::process::Command::new(env!("CARGO_BIN_EXE_forjar"))
        .args(["policy-coverage", "--file"])
        .arg(&cfg)
        .arg("--json")
        .output()
        .expect("forjar policy-coverage runs");
    assert!(
        run.status.success(),
        "forjar policy-coverage --json failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let from_cli: serde_json::Value = serde_json::from_slice(&run.stdout)
        .unwrap_or_else(|e| panic!("--json did not print JSON ({e}): {:?}", run.stdout));

    assert_eq!(
        from_verb,
        from_cli,
        "the `policy-coverage` verb and `forjar policy-coverage --json` \
         returned DIFFERENT documents. They are supposed to be two renderers \
         over one calculation; a field that appears on one and not the other, \
         or the same field with a different value, means there are two \
         calculations again.\n\nverb: {}\n\ncli:  {}",
        serde_json::to_string_pretty(&from_verb).unwrap_or_default(),
        serde_json::to_string_pretty(&from_cli).unwrap_or_default(),
    );

    // Vacuity guard: an empty object equals an empty object.
    assert!(
        from_verb.as_object().is_some_and(|o| o.len() >= 10),
        "the document has {} fields — the equality above is close to vacuous",
        from_verb.as_object().map(serde_json::Map::len).unwrap_or(0)
    );
}

/// The guard against "covered" being a constant.
#[test]
fn policy_coverage_reports_a_config_with_no_policies_as_uncovered() {
    let d = tempfile::tempdir().unwrap();
    let cfg = d.path().join("forjar.yaml");
    std::fs::write(
        &cfg,
        "version: \"1.0\"\nname: bare\nmachines:\n  local:\n    hostname: localhost\n    \
         addr: localhost\nresources:\n  conf:\n    type: file\n    machine: local\n    \
         path: /etc/a.conf\n    content: x\n",
    )
    .unwrap();

    let out = call(
        "policy-coverage",
        serde_json::json!({ "path": cfg.display().to_string() }),
    )
    .expect("policy-coverage runs");

    assert_eq!(out["covered_resources"], 0);
    assert_eq!(out["coverage_percent"], 0.0);
    assert_eq!(out["uncovered"], serde_json::json!(["conf"]));
    // ...and it is clean, because nothing looked at it. A report that printed
    // only `clean_resources` would call this config compliant.
    assert_eq!(out["clean_resources"], 1, "{out}");
}

// ── audit ───────────────────────────────────────────────────────────

#[test]
fn audit_returns_the_trail_as_structured_events() {
    let d = tempfile::tempdir().unwrap();
    let cfg = audited_project(d.path());

    let out = call(
        "audit",
        serde_json::json!({ "path": cfg.display().to_string() }),
    )
    .expect("audit runs");

    assert_eq!(out["event_count"], 2, "{out}");
    let events = out["events"].as_array().expect("events array");
    assert_eq!(
        events[0]["timestamp"], "2026-08-01T10:00:05Z",
        "newest first: {out}"
    );
    assert_eq!(events[0]["machine"], "local");

    // The regression this shape exists to prevent: `forjar audit --json` used
    // to emit `"event": "ApplyStarted { machine: \"local\", ... }"` — Rust
    // Debug syntax inside a JSON string, in a document that exists to be
    // machine-read. A consumer must be able to reach a field by name.
    assert_eq!(events[1]["event"]["event"], "apply_started");
    assert_eq!(events[1]["event"]["run_id"], "r-000000000001");
    assert_eq!(events[1]["event"]["operator"], "ng@box");
    assert!(
        events[1]["event"].is_object(),
        "the event must be an object, not a Debug-printed string: {}",
        events[1]["event"]
    );
}

/// GH-208's lesson, applied to the audit trail: "I could not read the log" must
/// not be reported as "nothing happened". That substitution is what let
/// `forjar_drift` certify a tampered machine as clean.
#[test]
fn audit_refuses_to_report_an_unreadable_trail_as_an_empty_one() {
    let d = tempfile::tempdir().unwrap();
    let cfg = d.path().join("forjar.yaml");
    std::fs::write(
        &cfg,
        "version: \"1.0\"\nname: nostate\nmachines:\n  local:\n    hostname: localhost\n    \
         addr: localhost\nresources: {}\n",
    )
    .unwrap();

    let err = call(
        "audit",
        serde_json::json!({ "path": cfg.display().to_string() }),
    )
    .expect_err("a state dir that does not exist is an error, not an empty trail");
    assert!(
        err.contains("cannot read state dir"),
        "unhelpful error: {err}"
    );
}

/// `limit` narrows the answer rather than being accepted and ignored.
#[test]
fn audit_honours_the_limit() {
    let d = tempfile::tempdir().unwrap();
    let cfg = audited_project(d.path());

    let out = call(
        "audit",
        serde_json::json!({ "path": cfg.display().to_string(), "limit": 1 }),
    )
    .expect("audit runs");

    assert_eq!(out["event_count"], 1, "{out}");
    assert_eq!(out["events"][0]["timestamp"], "2026-08-01T10:00:05Z");
}
