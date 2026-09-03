//! The verb/MCP surface reads the state the WORKSPACE SELECTION points at
//! (paiml/forjar#367).
//!
//! `forjar workspace select prod` moves where the CLI reads and writes state:
//! `cli::workspace::resolve_state_dir` joins the active workspace onto the state
//! dir. `mcp::paths::resolve_state_dir*` — which every state-reading verb calls,
//! on `verb call`, over MCP stdio and over HTTP alike — did not. Measured on
//! 1.24.0, one project, `.forjar/workspace = prod`, `forjar apply --yes` having
//! landed state under `state/prod/`:
//!
//! ```text
//!   $ forjar plan -f forjar.yaml
//!     Plan: 0 to add, 0 to change, 0 to destroy, 1 unchanged.
//!   $ forjar verb call plan --json '{"path":".../forjar.yaml"}'
//!     { "to_create": 1, "unchanged": 0 }        # CREATE for a converged file
//!   $ wc -l state/prod/local/events.jsonl  ->  4
//!   $ forjar verb call audit --json '{"path":".../forjar.yaml"}'
//!     { "event_count": 0, "events": [] }        # over a four-event trail
//!   $ forjar verb call status --json '{"path":".../forjar.yaml"}'
//!     { "machines": [] }
//! ```
//!
//! None of those three answers carries a tell. `to_create: 1`, `event_count: 0`
//! and `machines: []` are exactly what an empty project reports, so an agent
//! reading them proposes creating what already exists and concludes nothing has
//! ever run on a machine with a full trail. That is GH-208's failure mode — "I
//! could not find your state" rendered as "you have no state" — reached through
//! a second door.
//!
//! The last two tests are the ones that keep the fix honest rather than merely
//! green: joining the workspace in the WRONG place double-joins
//! (`state/prod/prod`) and turns the `workspace` verb's listing into the machine
//! directories under the active workspace.
//!
//! Usage: cargo test --test falsification_verb_honours_the_workspace

use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_forjar");

/// Invoke a verb through the unified surface — the entry point `verb call`, MCP
/// and HTTP all derive from, so one assertion covers three transports.
///
/// The test process runs with the crate root as its cwd, never the fixture's
/// directory. That is not incidental: it is exactly the situation of an MCP
/// server whose cwd the client chose, and it is what makes this a test of
/// config-directory resolution rather than of `cd`.
fn call(verb: &str, params: serde_json::Value) -> serde_json::Value {
    let v = forjar::verb::find(verb)
        .unwrap_or_else(|| panic!("verb `{verb}` is not on the unified surface"));
    (v.invoke)(params).unwrap_or_else(|e| panic!("verb `{verb}` failed: {e}"))
}

/// A project with one `file` resource writing inside the fixture directory.
fn write_config(dir: &Path) -> PathBuf {
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: wsel\nmachines:\n  local:\n    hostname: localhost\n    \
             addr: localhost\nresources:\n  conf:\n    type: file\n    machine: local\n    \
             path: {}\n    content: \"k=v\"\n    mode: \"0644\"\n",
            dir.join("out.conf").display()
        ),
    )
    .expect("write config");
    cfg
}

fn select_workspace(dir: &Path, name: &str) {
    std::fs::create_dir_all(dir.join(".forjar")).expect("mkdir .forjar");
    std::fs::write(dir.join(".forjar").join("workspace"), name).expect("write marker");
}

/// `forjar apply --yes` from inside the fixture, the way a user reaches it.
///
/// SAFETY: one `type: file` resource on machine `local`, writing a path inside
/// the tempdir. No package, service, mount or network resource is involved.
fn apply(dir: &Path) {
    let out = Command::new(BIN)
        .args(["apply", "-f", "forjar.yaml", "--yes"])
        .current_dir(dir)
        .output()
        .expect("run forjar apply");
    assert!(
        out.status.success(),
        "`forjar apply` failed, so there is no state to disagree about: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A fixture whose state landed under `state/<ws>/`, with the vacuity guard
/// that makes every assertion below about resolution rather than about an empty
/// tree.
fn applied_under(dir: &Path, ws: &str) -> PathBuf {
    let cfg = write_config(dir);
    select_workspace(dir, ws);
    apply(dir);
    let designated = dir.join("state").join(ws);
    assert!(
        designated.join("local").is_dir(),
        "the CLI did not write state under the selected workspace `{ws}`, so \
         these assertions would be comparing two absences"
    );
    cfg
}

fn events_on_disk(dir: &Path, ws: &str) -> usize {
    let trail = dir
        .join("state")
        .join(ws)
        .join("local")
        .join("events.jsonl");
    let text = std::fs::read_to_string(&trail)
        .unwrap_or_else(|e| panic!("no trail at {}: {e}", trail.display()));
    let n = text.lines().filter(|l| !l.trim().is_empty()).count();
    assert!(n > 0, "the trail is empty, so `event_count` proves nothing");
    n
}

// ── the defect ──────────────────────────────────────────────────────

/// REJECTION CRITERION: a converged resource reported as CREATE because the
/// verb surface read `state/` while the CLI wrote `state/prod/`.
#[test]
fn plan_reads_the_lock_the_selection_points_at() {
    let d = tempfile::tempdir().unwrap();
    let cfg = applied_under(d.path(), "prod");

    let verb = call(
        "plan",
        serde_json::json!({ "path": cfg.display().to_string() }),
    );

    assert_eq!(
        verb["unchanged"], 1,
        "the `plan` verb did not read the lock under the selected workspace, \
         so a converged resource is reported as work to do: {verb}"
    );
    assert_eq!(verb["to_create"], 0, "{verb}");
}

/// REJECTION CRITERION: `event_count: 0` over a trail that is on disk.
#[test]
fn audit_reads_the_trail_the_selection_points_at() {
    let d = tempfile::tempdir().unwrap();
    let cfg = applied_under(d.path(), "prod");
    let on_disk = events_on_disk(d.path(), "prod");

    let verb = call(
        "audit",
        serde_json::json!({ "path": cfg.display().to_string(), "limit": 1000 }),
    );

    assert_eq!(
        verb["event_count"],
        serde_json::json!(on_disk),
        "the `audit` verb reported {} events over a {on_disk}-event trail in \
         state/prod/local/events.jsonl — an empty trail is the same defect \
         wearing a different name: {verb}",
        verb["event_count"]
    );
}

/// REJECTION CRITERION: `machines: []` on a fleet that has been applied.
#[test]
fn status_sees_the_machines_under_the_selected_workspace() {
    let d = tempfile::tempdir().unwrap();
    let cfg = applied_under(d.path(), "prod");

    let verb = call(
        "status",
        serde_json::json!({ "path": cfg.display().to_string() }),
    );

    let machines: Vec<String> = verb["machines"]
        .as_array()
        .unwrap_or_else(|| panic!("status returned no machines array: {verb}"))
        .iter()
        .filter_map(|m| m["name"].as_str().map(str::to_string))
        .collect();
    assert!(
        machines.iter().any(|m| m == "local"),
        "the `status` verb reported {machines:?} for a fleet whose lock is at \
         state/prod/local/ — indistinguishable from a project nobody has ever \
         applied: {verb}"
    );
}

// ── the two ways the fix can be wrong ───────────────────────────────

/// GUARD, green in both directions: an EXPLICIT `state_dir` is honoured
/// verbatim, never joined again.
///
/// Handing `workspace_state_dir` back as the next verb's `state_dir` is the
/// documented workaround, so a fix that joins the active workspace onto an
/// explicit override turns the workaround into `state/prod/prod` and breaks
/// every caller who followed it. (`cli::workspace::resolve_state_dir` does
/// exactly that today — `forjar plan --state-dir state/prod` inside the `prod`
/// workspace resolves `state/prod/prod` — which is why this is asserted here
/// rather than assumed.)
#[test]
fn an_explicit_state_dir_is_not_joined_a_second_time() {
    let d = tempfile::tempdir().unwrap();
    let cfg = applied_under(d.path(), "prod");
    let on_disk = events_on_disk(d.path(), "prod");
    let designated = d.path().join("state").join("prod");

    let told = call(
        "audit",
        serde_json::json!({
            "path": cfg.display().to_string(),
            "state_dir": designated.display().to_string(),
            "limit": 1000,
        }),
    );

    assert_eq!(
        told["event_count"],
        serde_json::json!(on_disk),
        "an explicit `state_dir` was not honoured verbatim — the active \
         workspace was joined onto it, giving state/prod/prod: {told}"
    );
}

/// GUARD: no selection means the state base itself, exactly as before.
#[test]
fn a_project_with_no_selection_still_resolves_the_bare_state_dir() {
    let d = tempfile::tempdir().unwrap();
    let cfg = write_config(d.path());
    apply(d.path());
    assert!(
        d.path().join("state").join("local").is_dir(),
        "with no workspace selected the CLI writes state/<machine>/ directly"
    );

    let verb = call(
        "plan",
        serde_json::json!({ "path": cfg.display().to_string() }),
    );

    assert_eq!(
        verb["unchanged"], 1,
        "resolution of a project with no `.forjar/workspace` changed: {verb}"
    );
}

/// GUARD, and the one that catches the collateral: the `workspace` verb needs
/// the state BASE, not the joined directory.
///
/// It enumerates `state/` and does its OWN `state_base.join(active)`. Route it
/// through the joined resolver and it reports the MACHINE directories under the
/// active workspace as if they were workspaces, and `workspace_state_dir`
/// becomes `state/prod/prod` — the double-join, re-entered from inside the fix
/// for the double-join.
#[test]
fn the_workspace_verb_still_enumerates_the_state_base_not_the_joined_dir() {
    let d = tempfile::tempdir().unwrap();
    let cfg = applied_under(d.path(), "prod");
    std::fs::create_dir_all(d.path().join("state").join("staging")).unwrap();

    let out = call(
        "workspace",
        serde_json::json!({ "path": cfg.display().to_string() }),
    );

    assert_eq!(
        out["state_base"],
        serde_json::json!(d.path().join("state").display().to_string()),
        "the workspace verb inspected the joined directory instead of the \
         state base: {out}"
    );
    assert_eq!(
        out["workspace_state_dir"],
        serde_json::json!(d.path().join("state").join("prod").display().to_string()),
        "double-join: the selection was applied twice: {out}"
    );
    let names: Vec<String> = out["workspaces"]
        .as_array()
        .unwrap_or_else(|| panic!("no workspaces array: {out}"))
        .iter()
        .filter_map(|w| w["name"].as_str().map(str::to_string))
        .collect();
    assert_eq!(
        names,
        vec!["prod".to_string(), "staging".to_string()],
        "the listing is not the workspaces — `local` here would be a MACHINE \
         directory reported as a workspace: {out}"
    );
}
