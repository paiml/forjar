//! What the `workspace` verb reports — and what it does NOT (paiml/forjar#356).
//!
//! Split out of `falsification_verb_pending_discharge.rs` for the 500-line file
//! cap, and then given the tests that the claim which shipped with the verb
//! needed and did not have.
//!
//! THE RETRACTED CLAIM. The commit that added this verb justified it on
//! "`workspace` decides where every other verb reads its state". It does not.
//! `mcp::paths::resolve_state_dir*` — which every state-reading verb calls —
//! resolves `<config dir>/state` and never joins the active workspace onto it,
//! while `cli::workspace::resolve_state_dir` does. Measured against the built
//! binary, one project, `.forjar/workspace = prod`, lock and trail under
//! `state/prod/`:
//!
//! ```text
//!   $ forjar plan -f forjar.yaml
//!   state: <root>/state/prod
//!   Plan: 0 to add, 0 to change, 0 to destroy, 1 unchanged.
//!
//!   $ forjar verb call plan --json '{"path":"<root>/forjar.yaml"}'
//!   { "to_create": 1, "unchanged": 0, ... }
//!
//!   $ forjar verb call audit --json '{"path":"<root>/forjar.yaml"}'
//!   { "event_count": 0, "events": [] }      # over a four-event trail
//! ```
//!
//! That divergence is PRE-EXISTING and is filed as paiml/forjar#367; it is not
//! fixed here. What is fixed here is the report: `workspace_state_dir` now
//! names the directory the selection designates, so a caller can pass it to the
//! next verb instead of assuming the selection was honoured.
//!
//! Usage: cargo test --test falsification_verb_workspace_report

use std::path::Path;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_forjar");

/// Invoke a verb through the unified surface — the same entry point every
/// transport derives from.
///
/// Deliberately a local copy of the three-line helper in
/// `common/verb_pending_fixtures.rs` rather than an include: this file needs
/// only this one function, and importing the module would leave its other
/// fixtures unused in this compilation unit.
fn call(verb: &str, params: serde_json::Value) -> Result<serde_json::Value, String> {
    let v = forjar::verb::find(verb)
        .unwrap_or_else(|| panic!("verb `{verb}` is not on the unified surface"));
    (v.invoke)(params)
}

fn write_config(dir: &Path) -> std::path::PathBuf {
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        "version: \"1.0\"\nname: ws\nmachines:\n  local:\n    hostname: localhost\n    \
         addr: localhost\nresources: {}\n",
    )
    .unwrap();
    cfg
}

fn workspace_report(cfg: &Path) -> serde_json::Value {
    call(
        "workspace",
        serde_json::json!({ "path": cfg.display().to_string() }),
    )
    .expect("workspace runs")
}

// ── what it reports ─────────────────────────────────────────────────

#[test]
fn workspace_reports_the_selected_workspace_and_its_siblings() {
    let d = tempfile::tempdir().unwrap();
    let cfg = write_config(d.path());
    for ws in ["staging", "prod"] {
        std::fs::create_dir_all(d.path().join("state").join(ws)).unwrap();
    }
    std::fs::create_dir_all(d.path().join(".forjar")).unwrap();
    std::fs::write(d.path().join(".forjar").join("workspace"), "prod").unwrap();

    let out = workspace_report(&cfg);

    assert_eq!(
        out["active"], "prod",
        "the selection recorded in `.forjar/workspace` is what this verb \
         reports; an agent that cannot ask cannot name the directory the CLI \
         is working in: {out}"
    );
    assert_eq!(
        out["workspaces"],
        serde_json::json!([
            { "name": "prod", "active": true },
            { "name": "staging", "active": false },
        ]),
        "{out}"
    );
    assert_eq!(
        out["workspace_state_dir"],
        d.path().join("state").join("prod").display().to_string(),
        "the directory the selection DESIGNATES — which the other verbs do not \
         resolve, see paiml/forjar#367: {out}"
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
    let cfg = write_config(d.path());
    let names: Vec<String> = (0..12).map(|i| format!("ws{i:02}")).collect();
    for n in names.iter().rev() {
        std::fs::create_dir_all(d.path().join("state").join(n)).unwrap();
    }

    let out = workspace_report(&cfg);

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
    let cfg = write_config(d.path());

    let out = workspace_report(&cfg);

    assert_eq!(out["active"], serde_json::Value::Null);
    assert_eq!(out["workspaces"], serde_json::json!([]));
    assert_eq!(
        out["state_base"],
        d.path().join("state").display().to_string(),
        "the state base is echoed so the caller knows which directory was \
         inspected: {out}"
    );
    assert_eq!(
        out["workspace_state_dir"], out["state_base"],
        "with nothing selected, the directory the selection designates IS the \
         state base: {out}"
    );
}

/// GH-208: the workspace marker lives beside the CONFIG, not in the server's
/// cwd. The CLI hard-codes `.` and is right to — its cwd is the project. An MCP
/// server's cwd is chosen by the client, so a project addressed by absolute
/// path must still find its own `.forjar/workspace`.
#[test]
fn workspace_follows_the_config_not_the_process_cwd() {
    let d = tempfile::tempdir().unwrap();
    let cfg = write_config(d.path());
    std::fs::create_dir_all(d.path().join("state").join("yoga")).unwrap();
    std::fs::create_dir_all(d.path().join(".forjar")).unwrap();
    std::fs::write(d.path().join(".forjar").join("workspace"), "yoga").unwrap();

    // The test process runs from the crate root, so cwd is ALREADY not the
    // fixture's directory — exactly the situation of an MCP stdio server.
    let out = workspace_report(&cfg);

    assert_eq!(
        out["active"], "yoga",
        "the marker beside the config was not read — the tool looked in the \
         process cwd (GH-208): {out}"
    );
}

// ── the two claims the code did not support ─────────────────────────

/// REJECTION CRITERION: an empty listing that cannot be told from a wrong path.
///
/// `state_base` was documented as letting "a caller tell an empty list apart
/// from having pointed the tool at the wrong directory". It could not: it is
/// built from `path`/`state_dir` alone and is the same string either way, and
/// `list_workspaces_in` returns `Ok(vec![])` both when the state base is empty
/// and when it is absent. The two reports were byte-identical apart from the
/// path they echoed back.
#[test]
fn an_empty_listing_is_distinguishable_from_a_state_base_that_is_not_there() {
    let applied_nothing = tempfile::tempdir().unwrap();
    let cfg_a = write_config(applied_nothing.path());
    std::fs::create_dir_all(applied_nothing.path().join("state")).unwrap();
    let there = workspace_report(&cfg_a);

    let wrong_directory = tempfile::tempdir().unwrap();
    let cfg_b = write_config(wrong_directory.path());
    let absent = workspace_report(&cfg_b);

    assert_eq!(there["workspaces"], serde_json::json!([]), "{there}");
    assert_eq!(absent["workspaces"], serde_json::json!([]), "{absent}");
    assert_eq!(there["state_base_exists"], true, "{there}");
    assert_eq!(absent["state_base_exists"], false, "{absent}");

    // Vacuity guard: the two reports must differ ONLY in the path-derived
    // fields and the new one. If they already differed elsewhere, the
    // assertions above are riding on some incidental difference.
    let strip = |v: &serde_json::Value| {
        let mut o = v.as_object().expect("object").clone();
        o.remove("state_base");
        o.remove("state_base_exists");
        o.remove("workspace_state_dir");
        serde_json::Value::Object(o)
    };
    assert_eq!(
        strip(&there),
        strip(&absent),
        "the rest of the two reports already differed, so `state_base_exists` \
         is not what distinguishes them"
    );
}

/// REJECTION CRITERION: `workspace_state_dir` that is not the path the other
/// verbs need to be handed.
///
/// The retracted claim said the selection governs the other verbs. It does not,
/// so the report has to give a caller something it can ACT on: the same trail
/// is invisible to `audit` called with `path` alone and visible to `audit`
/// called with `workspace_state_dir` as its `state_dir`.
#[test]
fn the_designated_directory_is_the_one_another_verb_has_to_be_given() {
    let d = tempfile::tempdir().unwrap();
    let cfg = write_config(d.path());
    let md = d.path().join("state").join("prod").join("local");
    std::fs::create_dir_all(&md).unwrap();
    std::fs::write(
        md.join("events.jsonl"),
        "{\"ts\":\"2026-08-01T10:00:00Z\",\"event\":\"apply_started\",\"machine\":\"local\",\
         \"run_id\":\"r-000000000001\",\"forjar_version\":\"1.21.0\",\"operator\":\"ng@box\"}\n\
         {\"ts\":\"2026-08-01T10:00:05Z\",\"event\":\"resource_converged\",\"machine\":\"local\",\
         \"resource\":\"conf\",\"duration_seconds\":0.5,\"hash\":\"abc123\"}\n",
    )
    .unwrap();
    std::fs::create_dir_all(d.path().join(".forjar")).unwrap();
    std::fs::write(d.path().join(".forjar").join("workspace"), "prod").unwrap();

    let ws = workspace_report(&cfg);
    let designated = ws["workspace_state_dir"].as_str().expect("a path");
    assert_eq!(
        designated,
        d.path().join("state").join("prod").display().to_string(),
        "{ws}"
    );

    let blind = call(
        "audit",
        serde_json::json!({ "path": cfg.display().to_string() }),
    )
    .expect("audit runs");
    assert_eq!(
        blind["event_count"], 0,
        "the `audit` verb now honours the active workspace. If that is \
         deliberate, paiml/forjar#367 has been fixed and the doc comment on \
         `WorkspaceOutput::workspace_state_dir` — which states the opposite, as \
         measured — has to be rewritten with it: {blind}"
    );

    let told = call(
        "audit",
        serde_json::json!({ "path": cfg.display().to_string(), "state_dir": designated }),
    )
    .expect("audit runs");
    assert_eq!(
        told["event_count"], 2,
        "handing `workspace_state_dir` back as `state_dir` did not reach the \
         trail the selection points at, which is the only thing that makes the \
         field worth reporting: {told}"
    );
}

/// The measurement the doc comment on `workspace_state_dir` quotes, run against
/// the BUILT BINARY so the quote cannot go stale silently.
///
/// One project, one selection, two surfaces: `forjar plan` resolves
/// `state/prod` and reports the resource as unchanged; `verb call plan` on the
/// same config resolves `state` and reports it as a create.
#[test]
fn the_cli_honours_the_selection_and_the_verb_surface_does_not() {
    let d = tempfile::tempdir().unwrap();
    let target = d.path().join("out.conf");
    let cfg = d.path().join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: wsdiv\nmachines:\n  local:\n    hostname: localhost\n    \
             addr: localhost\nresources:\n  conf:\n    type: file\n    machine: local\n    \
             path: {}\n    content: \"k=v\"\n    mode: \"0644\"\n",
            target.display()
        ),
    )
    .unwrap();
    std::fs::create_dir_all(d.path().join(".forjar")).unwrap();
    std::fs::write(d.path().join(".forjar").join("workspace"), "prod").unwrap();

    // stdout AND stderr: `plan` prints its resolved state dir on stderr and the
    // document on stdout, and this test needs both.
    let forjar = |args: &[&str]| {
        let out = Command::new(BIN)
            .args(args)
            .current_dir(d.path())
            .output()
            .unwrap_or_else(|e| panic!("cannot run `forjar {}`: {e}", args.join(" ")));
        assert!(
            out.status.success(),
            "`forjar {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
        (
            String::from_utf8_lossy(&out.stdout).to_string(),
            String::from_utf8_lossy(&out.stderr).to_string(),
        )
    };

    let _ = forjar(&["apply", "-f", "forjar.yaml", "--yes"]);
    assert!(
        d.path().join("state").join("prod").join("local").is_dir(),
        "the CLI did not write state under the selected workspace, so the \
         comparison below has nothing to compare"
    );

    // `forjar plan --json` prints a `state: <dir>` banner before the document,
    // and that banner is itself the evidence: it names `state/prod`.
    let (printed, banner) = forjar(&["plan", "-f", "forjar.yaml", "--json"]);
    let want = format!("state: {}", d.path().join("state").join("prod").display());
    assert!(
        banner.contains(&want) || printed.contains(&want),
        "the CLI did not resolve the selected workspace.\nstdout: \
         {printed}\nstderr: {banner}"
    );
    let start = printed
        .find('{')
        .unwrap_or_else(|| panic!("plan --json printed no JSON: {printed}"));
    let cli: serde_json::Value = serde_json::from_str(&printed[start..])
        .unwrap_or_else(|e| panic!("plan --json did not print JSON ({e}): {printed}"));
    assert_eq!(
        cli["unchanged"], 1,
        "`forjar plan` did not read the lock under the selected workspace: {cli}"
    );

    let verb = call(
        "plan",
        serde_json::json!({ "path": cfg.display().to_string() }),
    )
    .expect("plan runs");
    assert_eq!(
        verb["to_create"], 1,
        "the `plan` verb now reads the selected workspace's lock. That is \
         paiml/forjar#367 fixed — good — and the doc comment on \
         `WorkspaceOutput::workspace_state_dir` quotes this measurement, so it \
         has to be rewritten with it: {verb}"
    );
}
