//! The CLI leaves really do answer from the shared readers (paiml/forjar#356).
//!
//! Two commits on this branch extracted a reader and then fixed its ordering:
//!
//! * `cli::fleet_reporting::cmd_audit` -> `tripwire::audit_trail::collect_events`
//! * `cli::workspace::workspace_list_in` -> `cli::workspace::list_workspaces_in`
//!
//! Every test for both sits on the extracted function. That leaves the
//! delegation itself unguarded, and it is not a theoretical gap — it was
//! measured. Reverting `cmd_audit` to the inline reader it had on `main`
//! (`b.1.ts.cmp(&a.1.ts)` with no tie-break) and running the whole suite:
//!
//! ```text
//!   branch as committed:  suites=225 passed=17439 failed=0
//!   cmd_audit reverted:   suites=225 passed=17439 failed=0
//! ```
//!
//! Byte for byte. The command whose behaviour the ordering fix is *about* could
//! stop calling the fixed reader and nothing in 17,439 tests would notice. The
//! same holds for `workspace list`: `list_workspaces_in` sorts, and every test
//! of the listing order calls `list_workspaces_in`.
//!
//! So these tests run the BUILT BINARY. A test that calls the library proves the
//! library agrees with itself; only the process proves what the command prints.
//!
//! Usage: cargo test --test falsification_verb_cli_reader_parity

use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_forjar");

/// Twelve machines, one event each, ALL stamped the same second, with the
/// directories created in REVERSE name order.
///
/// `tripwire::eventlog::now_iso8601` stamps whole seconds, so a fleet applying
/// one config produces exactly this: a tied group. Returns the state dir.
fn tied_fleet(dir: &Path) -> PathBuf {
    let state = dir.join("state");
    for i in (0..12).rev() {
        let md = state.join(format!("m{i:02}"));
        std::fs::create_dir_all(&md).unwrap();
        std::fs::write(
            md.join("events.jsonl"),
            format!(
                "{{\"ts\":\"2026-08-01T10:00:00Z\",\"event\":\"resource_started\",\
                 \"machine\":\"m{i:02}\",\"resource\":\"r\",\"action\":\"create\"}}\n"
            ),
        )
        .unwrap();
    }
    state
}

/// A config so the verb surface can be told WHICH project it is being asked
/// about. It declares no resources: these tests are about the reader, not the
/// planner.
fn bare_config(dir: &Path, name: &str) -> PathBuf {
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: {name}\nmachines:\n  local:\n    hostname: localhost\n    \
             addr: localhost\nresources: {{}}\n"
        ),
    )
    .unwrap();
    cfg
}

/// The order the OS hands the directory back in — the order that leaks into the
/// answer when nothing sorts.
fn read_dir_order(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect()
}

/// These tests can only falsify while the filesystem does NOT already list the
/// fixture in sorted order. On ext4 that order is a hash of the name and on
/// tmpfs it is creation order (which the fixtures reverse), so neither is
/// sorted — but it is the premise the tests rest on, so it is checked rather
/// than assumed.
fn assert_the_fixture_can_falsify(dir: &Path, sorted: &[String]) {
    let raw = read_dir_order(dir);
    assert_ne!(
        raw,
        sorted,
        "{} was listed by the filesystem in sorted order, so this test cannot \
         tell a deterministic sort from the absence of one — it would pass \
         either way",
        dir.display()
    );
}

fn run(args: &[&str], cwd: &Path) -> std::process::Output {
    let out = Command::new(BIN)
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|e| panic!("cannot run `forjar {}`: {e}", args.join(" ")));
    assert!(
        out.status.success(),
        "`forjar {}` failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

fn json_of(out: &std::process::Output, what: &str) -> serde_json::Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "{what} did not print JSON ({e}): {}",
            String::from_utf8_lossy(&out.stdout)
        )
    })
}

// ── audit ───────────────────────────────────────────────────────────

/// REJECTION CRITERION: `forjar audit -n 4` over a tied fleet returning an
/// arbitrary four.
///
/// This is the claim the tie-break commit ("test(audit): the ordering fix that
/// survives its own deletion") was written about — "without it, `forjar audit
/// -n 4` over a fleet that applied in one second returned m01, m05, m04, m06,
/// an arbitrary four of twelve, chosen by a filesystem hash" — and it is the
/// claim nothing tested. With `cmd_audit` reverted to the inline reader, the
/// built binary prints exactly that quartet, three runs in a row; with the
/// delegation in place it prints m00..m03.
#[test]
fn the_cli_audit_window_over_a_tie_is_the_machine_names_not_read_dir_order() {
    let d = tempfile::tempdir().unwrap();
    let state = tied_fleet(d.path());
    let names: Vec<String> = (0..12).map(|i| format!("m{i:02}")).collect();
    assert_the_fixture_can_falsify(&state, &names);

    let out = run(
        &[
            "audit",
            "--state-dir",
            &state.display().to_string(),
            "-n",
            "4",
            "--json",
        ],
        d.path(),
    );
    let events = json_of(&out, "forjar audit --json");
    let got: Vec<String> = events
        .as_array()
        .expect("audit --json prints an array")
        .iter()
        .map(|e| e["machine"].as_str().unwrap_or_default().to_string())
        .collect();

    assert_eq!(
        got,
        names[..4],
        "`forjar audit -n 4` returned four events of a twelve-event tie that \
         are not a DEFINED four: which four you get is the order the filesystem \
         listed the state dir in ({:?}). The command is the record consulted \
         when someone is asking what happened, and two hosts of the same fleet \
         reading one copied state dir would answer differently.",
        read_dir_order(&state)
    );

    // Vacuity guard: the window is only interesting because the group ties.
    let all = json_of(
        &run(
            &[
                "audit",
                "--state-dir",
                &state.display().to_string(),
                "-n",
                "100",
                "--json",
            ],
            d.path(),
        ),
        "forjar audit --json",
    );
    let all = all.as_array().expect("array");
    assert_eq!(all.len(), 12, "the fixture lost events");
    assert!(
        all.iter().all(|e| e["timestamp"] == "2026-08-01T10:00:00Z"),
        "the fixture does not tie, so the tie-break is never reached"
    );
}

/// REJECTION CRITERION: the `audit` verb and `forjar audit --json` answering
/// differently.
///
/// One reader, two renderings. `AuditOutput` wraps the list in
/// `{event_count, events}` and the CLI prints the list bare, so the assertion
/// is between the CLI's document and the verb's `events` — and it is an
/// equality, not a length check, because the regression this shape exists to
/// prevent was a Debug-printed event inside a JSON string.
#[test]
fn the_cli_audit_leaf_and_the_audit_verb_print_the_same_events() {
    let d = tempfile::tempdir().unwrap();
    let state = tied_fleet(d.path());
    let cfg = bare_config(d.path(), "audited");

    let from_cli = json_of(
        &run(
            &[
                "audit",
                "--state-dir",
                &state.display().to_string(),
                "-n",
                "5",
                "--json",
            ],
            d.path(),
        ),
        "forjar audit --json",
    );

    let params = serde_json::json!({
        "path": cfg.display().to_string(),
        "state_dir": state.display().to_string(),
        "limit": 5,
    })
    .to_string();
    let from_verb = json_of(
        &run(&["verb", "call", "audit", "--json", &params], d.path()),
        "forjar verb call audit",
    );

    assert_eq!(
        from_verb["events"],
        from_cli,
        "`forjar audit --json` and the `audit` verb returned DIFFERENT trails \
         over one state dir. They are two renderings of \
         `tripwire::audit_trail::collect_events`; a difference means one of \
         them stopped calling it.\n\ncli:  {}\n\nverb: {}",
        serde_json::to_string_pretty(&from_cli).unwrap_or_default(),
        serde_json::to_string_pretty(&from_verb["events"]).unwrap_or_default(),
    );
    assert_eq!(from_verb["event_count"], 5, "{from_verb}");

    // Vacuity guard: two empty arrays are equal.
    assert_eq!(
        from_cli.as_array().map(Vec::len),
        Some(5),
        "the equality above compared an empty trail: {from_cli}"
    );
    assert!(
        from_cli[0]["event"].is_object(),
        "the event must be an object, not a Debug-printed string: {}",
        from_cli[0]["event"]
    );
}

// ── workspace ───────────────────────────────────────────────────────

/// Twelve workspaces created in reverse name order, one of them selected.
fn workspace_project(dir: &Path) -> PathBuf {
    bare_config(dir, "ws");
    for i in (0..12).rev() {
        std::fs::create_dir_all(dir.join("state").join(format!("ws{i:02}"))).unwrap();
    }
    std::fs::create_dir_all(dir.join(".forjar")).unwrap();
    std::fs::write(dir.join(".forjar").join("workspace"), "ws07").unwrap();
    dir.join("state")
}

/// REJECTION CRITERION: `forjar workspace list` printing directory order.
///
/// The same gap as `audit`: `list_workspaces_in` sorts and every test of the
/// order calls `list_workspaces_in`, so `workspace_list_in` could go back to
/// its own `read_dir` loop with the suite still green. The CLI leaf takes no
/// `--state-dir`, so this drives it the way a user does — from the project
/// directory.
#[test]
fn the_cli_workspace_list_is_sorted_not_left_in_read_dir_order() {
    let d = tempfile::tempdir().unwrap();
    let state = workspace_project(d.path());
    let names: Vec<String> = (0..12).map(|i| format!("ws{i:02}")).collect();
    assert_the_fixture_can_falsify(&state, &names);

    let out = run(&["workspace", "list"], d.path());
    let printed = String::from_utf8_lossy(&out.stdout).to_string();
    let got: Vec<String> = printed
        .lines()
        .map(|l| l.trim().trim_end_matches(" *").to_string())
        .filter(|l| !l.is_empty())
        .collect();

    assert_eq!(
        got,
        names,
        "`forjar workspace list` printed the workspaces in the order the \
         filesystem listed them ({:?}), so two runs over an unchanged \
         directory can print two different documents.\n\n{printed}",
        read_dir_order(&state)
    );
    assert!(
        printed.contains("ws07 *"),
        "the selected workspace lost its marker:\n{printed}"
    );
}

/// REJECTION CRITERION: the `workspace` verb and `forjar workspace list`
/// naming different workspaces, or disagreeing about which is selected.
#[test]
fn the_cli_workspace_list_and_the_workspace_verb_name_the_same_workspaces() {
    let d = tempfile::tempdir().unwrap();
    workspace_project(d.path());

    let printed =
        String::from_utf8_lossy(&run(&["workspace", "list"], d.path()).stdout).to_string();
    let from_cli: Vec<(String, bool)> = printed
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| match l.strip_suffix(" *") {
            Some(name) => (name.to_string(), true),
            None => (l.to_string(), false),
        })
        .collect();

    let out = run(
        &[
            "verb",
            "call",
            "workspace",
            "--json",
            "{\"path\":\"forjar.yaml\"}",
        ],
        d.path(),
    );
    let from_verb = json_of(&out, "forjar verb call workspace");
    let verb_pairs: Vec<(String, bool)> = from_verb["workspaces"]
        .as_array()
        .expect("workspaces array")
        .iter()
        .map(|w| {
            (
                w["name"].as_str().unwrap_or_default().to_string(),
                w["active"].as_bool().unwrap_or_default(),
            )
        })
        .collect();

    assert_eq!(
        from_cli, verb_pairs,
        "`forjar workspace list` and the `workspace` verb disagree about the \
         workspaces under one state dir.\n\ncli:\n{printed}\nverb: {from_verb}"
    );
    assert_eq!(from_verb["active"], "ws07", "{from_verb}");

    // Vacuity guard: two empty lists are equal.
    assert_eq!(from_cli.len(), 12, "compared an empty listing: {printed}");
}
