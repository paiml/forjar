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
//! Usage: cargo test --test falsification_verb_pending_discharge

use forjar::verb::{find, partition, Bucket};

// ── fixtures ────────────────────────────────────────────────────────

fn call(verb: &str, params: serde_json::Value) -> Result<serde_json::Value, String> {
    let v = find(verb).unwrap_or_else(|| {
        panic!(
            "verb `{verb}` is not on the unified surface — every transport \
             derives from this one table, so a missing row means the capability \
             is reachable from the CLI and nowhere else"
        )
    });
    (v.invoke)(params)
}

/// A project whose policies cover the file resource and not the package one.
fn policy_project(dir: &std::path::Path) -> std::path::PathBuf {
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        r#"
version: "1.0"
name: coverage-fixture
machines:
  local:
    hostname: localhost
    addr: localhost
resources:
  conf:
    type: file
    machine: local
    path: /etc/app.conf
    content: "k=v"
  pkg:
    type: package
    machine: local
    provider: apt
    packages: [git]
policies:
  - type: require
    message: files need an owner
    field: owner
    resource_type: file
    compliance:
      - framework: soc2
        control: CC6.1
"#,
    )
    .unwrap();
    cfg
}

/// Two provenance events on one machine, with distinct timestamps so ordering
/// is a property of the data rather than of `read_dir`.
fn audited_project(dir: &std::path::Path) -> std::path::PathBuf {
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        "version: \"1.0\"\nname: audited\nmachines:\n  local:\n    hostname: localhost\n    \
         addr: localhost\nresources: {}\n",
    )
    .unwrap();
    let md = dir.join("state").join("local");
    std::fs::create_dir_all(&md).unwrap();
    std::fs::write(
        md.join("events.jsonl"),
        "{\"ts\":\"2026-08-01T10:00:00Z\",\"event\":\"apply_started\",\"machine\":\"local\",\
         \"run_id\":\"r-000000000001\",\"forjar_version\":\"1.20.1\",\"operator\":\"ng@box\"}\n\
         {\"ts\":\"2026-08-01T10:00:05Z\",\"event\":\"resource_converged\",\"machine\":\"local\",\
         \"resource\":\"conf\",\"duration_seconds\":0.5,\"hash\":\"abc123\"}\n",
    )
    .unwrap();
    cfg
}

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
    assert_eq!(out["frameworks"], serde_json::json!(["soc2"]));
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

// ── workspace ───────────────────────────────────────────────────────

#[test]
fn workspace_reports_the_selected_workspace_and_its_siblings() {
    let d = tempfile::tempdir().unwrap();
    let cfg = d.path().join("forjar.yaml");
    std::fs::write(
        &cfg,
        "version: \"1.0\"\nname: ws\nmachines:\n  local:\n    hostname: localhost\n    \
         addr: localhost\nresources: {}\n",
    )
    .unwrap();
    for ws in ["staging", "prod"] {
        std::fs::create_dir_all(d.path().join("state").join(ws)).unwrap();
    }
    std::fs::create_dir_all(d.path().join(".forjar")).unwrap();
    std::fs::write(d.path().join(".forjar").join("workspace"), "prod").unwrap();

    let out = call(
        "workspace",
        serde_json::json!({ "path": cfg.display().to_string() }),
    )
    .expect("workspace runs");

    assert_eq!(
        out["active"], "prod",
        "the active workspace decides which state dir every other verb reads; \
         an agent that cannot ask is guessing: {out}"
    );
    assert_eq!(
        out["workspaces"],
        serde_json::json!([
            { "name": "prod", "active": true },
            { "name": "staging", "active": false },
        ]),
        "{out}"
    );
}

/// `read_dir` returns entries in no defined order — on the filesystems forjar
/// runs on it is a hash order, not creation order — so an unsorted listing can
/// differ between two calls over an unchanged directory. That is a poor
/// property for a tool whose output an agent diffs.
///
/// Twelve names, created in reverse, is the falsifier: an unsorted read would
/// have to land on the sorted permutation by chance (1 in 12!).
#[test]
fn workspace_listing_is_sorted_not_left_in_read_dir_order() {
    let d = tempfile::tempdir().unwrap();
    let cfg = d.path().join("forjar.yaml");
    std::fs::write(
        &cfg,
        "version: \"1.0\"\nname: ws\nmachines:\n  local:\n    hostname: localhost\n    \
         addr: localhost\nresources: {}\n",
    )
    .unwrap();
    let names: Vec<String> = (0..12).map(|i| format!("ws{i:02}")).collect();
    for n in names.iter().rev() {
        std::fs::create_dir_all(d.path().join("state").join(n)).unwrap();
    }

    let out = call(
        "workspace",
        serde_json::json!({ "path": cfg.display().to_string() }),
    )
    .expect("workspace runs");

    let got: Vec<String> = out["workspaces"]
        .as_array()
        .expect("workspaces array")
        .iter()
        .map(|w| w["name"].as_str().unwrap_or_default().to_string())
        .collect();
    assert_eq!(got, names, "listing came back in directory order");
}

/// No workspace selected is `null`, and `null` MEANS the default workspace —
/// it is not "unknown". A caller that cannot tell those apart cannot tell
/// `state/` from `state/<name>/`.
#[test]
fn workspace_reports_the_default_as_null_not_as_an_error() {
    let d = tempfile::tempdir().unwrap();
    let cfg = d.path().join("forjar.yaml");
    std::fs::write(
        &cfg,
        "version: \"1.0\"\nname: ws\nmachines:\n  local:\n    hostname: localhost\n    \
         addr: localhost\nresources: {}\n",
    )
    .unwrap();

    let out = call(
        "workspace",
        serde_json::json!({ "path": cfg.display().to_string() }),
    )
    .expect("workspace runs");

    assert_eq!(out["active"], serde_json::Value::Null);
    assert_eq!(out["workspaces"], serde_json::json!([]));
    assert_eq!(
        out["state_base"],
        d.path().join("state").display().to_string(),
        "the state base is echoed so an empty list can be told apart from \
         having pointed the tool at the wrong directory: {out}"
    );
}

/// GH-208: the workspace marker lives beside the CONFIG, not in the server's
/// cwd. The CLI hard-codes `.` and is right to — its cwd is the project. An MCP
/// server's cwd is chosen by the client, so a project addressed by absolute
/// path must still find its own `.forjar/workspace`.
#[test]
fn workspace_follows_the_config_not_the_process_cwd() {
    let d = tempfile::tempdir().unwrap();
    let cfg = d.path().join("forjar.yaml");
    std::fs::write(
        &cfg,
        "version: \"1.0\"\nname: ws\nmachines:\n  local:\n    hostname: localhost\n    \
         addr: localhost\nresources: {}\n",
    )
    .unwrap();
    std::fs::create_dir_all(d.path().join("state").join("yoga")).unwrap();
    std::fs::create_dir_all(d.path().join(".forjar")).unwrap();
    std::fs::write(d.path().join(".forjar").join("workspace"), "yoga").unwrap();

    // The test process runs from the crate root, so cwd is ALREADY not the
    // fixture's directory — exactly the situation of an MCP stdio server.
    let out = call(
        "workspace",
        serde_json::json!({ "path": cfg.display().to_string() }),
    )
    .expect("workspace runs");

    assert_eq!(
        out["active"], "yoga",
        "the marker beside the config was not read — the tool looked in the \
         process cwd (GH-208): {out}"
    );
}
