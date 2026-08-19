//! GH-253: `plan -r` and `apply -r` must agree about what will change.
//!
//! They did not, in the unsafe direction. `planner::plan` honours `tag_filter`,
//! but `-r` is applied later by the executor
//! (`resource_ops.rs`: `cfg.resource_filter.is_some_and(|f| change.resource_id != f)`).
//! The confirmation prompt counted the *unscoped* plan, so on paiml/infra's
//! 83-resource `machines/intel/forjar.yaml`:
//!
//! ```text
//! $ forjar plan  -f forjar.yaml -r stack-tool-forjar   Plan: 1 to add, ...
//! $ forjar apply -f forjar.yaml -r stack-tool-forjar   Apply 69 change(s) ...
//! ```
//!
//! Execution itself was correctly scoped — apply acted on 1 — so this was a
//! reporting defect, not the whole-machine convergence it first appeared to be.
//! That does not make it cosmetic: the count in the prompt is the only thing an
//! operator sees before approving, and `contracts/plan-apply-equivalence-v1.yaml`
//! obliges apply's outcome counts to match the plan's prediction. A prompt that
//! says 69 when apply will do 1 trains the operator to distrust the number, and
//! the same miscount would have blocked a `-r` apply behind destroys that
//! belonged to resources they never selected.
//!
//! These tests drive the real binary rather than the counting function, because
//! the defect lived in the wiring between plan and prompt — a unit test of the
//! counter would have passed throughout.

use std::io::Write;
use std::process::{Command, Stdio};

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

/// A config with one resource the tests select and several they do not.
///
/// The decoys matter: with a single resource declared, a filter that is ignored
/// entirely still yields the right count, and the regression would not show.
fn config_with_decoys(dir: &std::path::Path) -> std::path::PathBuf {
    let cfg = dir.join("forjar.yaml");
    let mut yaml = String::from(
        r#"version: "1.0"
name: scope-prompt
machines:
  localhost:
    hostname: localhost
    addr: 127.0.0.1
resources:
"#,
    );
    for name in ["selected", "decoy-a", "decoy-b", "decoy-c"] {
        yaml.push_str(&format!(
            "  {name}:\n    type: file\n    machine: localhost\n    path: /tmp/gh253-{name}.txt\n    content: \"{name}\"\n    owner: root\n    mode: \"0644\"\n"
        ));
    }
    std::fs::write(&cfg, yaml).expect("write config");
    cfg
}

/// Drop ANSI SGR sequences so counts can be read from coloured output.
///
/// `plan` colourises its summary (`Plan: \x1b[32m1\x1b[0m to add`), so a parser
/// that reads the raw bytes sees `\x1b[32m1\x1b[0m` and not `1`.
fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            out.push(c);
            continue;
        }
        // Consume the CSI introducer and everything up to the final byte.
        if chars.next() == Some('[') {
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
        }
    }
    out
}

/// Number in `Plan: N to add, ...`.
fn plan_count(text: &str) -> Option<u32> {
    let text = strip_ansi(text);
    let tail = text.split("Plan: ").nth(1)?;
    tail.split(" to add").next()?.trim().parse().ok()
}

/// Number in `Apply N change(s) ...`.
fn prompt_count(text: &str) -> Option<u32> {
    let text = strip_ansi(text);
    let tail = text.split("Apply ").nth(1)?;
    tail.split(" change(s)").next()?.trim().parse().ok()
}

/// Run `apply -r selected`, declining at the prompt, and return combined output.
///
/// Declining is deliberate: the assertion is about what the operator is *told*
/// before deciding, and it keeps the test from touching the host.
fn apply_declined(cfg: &std::path::Path, state: &std::path::Path) -> String {
    let mut child = forjar()
        .args([
            "apply",
            "-f",
            cfg.to_str().unwrap(),
            "-r",
            "selected",
            "--state-dir",
            state.to_str().unwrap(),
            "--no-tripwire",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn apply");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"n\n")
        .expect("decline");
    let out = child.wait_with_output().expect("wait");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn apply_prompt_counts_only_the_selected_resource() {
    // THE REGRESSION. Before the fix this printed the count for all four
    // resources while apply would act on exactly one.
    let dir = tempfile::tempdir().expect("tmpdir");
    let state = dir.path().join("state");
    std::fs::create_dir_all(&state).expect("state dir");
    let cfg = config_with_decoys(dir.path());

    let combined = apply_declined(&cfg, &state);
    let Some(n) = prompt_count(&combined) else {
        panic!("no confirmation prompt in output:\n{combined}");
    };

    assert_eq!(
        n, 1,
        "`-r selected` must offer exactly the 1 resource named, not the \
         3 decoys the operator did not select.\noutput:\n{combined}"
    );
}

#[test]
fn plan_and_apply_agree_under_the_same_selector() {
    // The contract obligation itself (plan-apply-equivalence-v1): the plan is a
    // promise, so the number apply asks about must be the number plan gave.
    let dir = tempfile::tempdir().expect("tmpdir");
    let state = dir.path().join("state");
    std::fs::create_dir_all(&state).expect("state dir");
    let cfg = config_with_decoys(dir.path());

    let plan_out = forjar()
        .args([
            "plan",
            "-f",
            cfg.to_str().unwrap(),
            "-r",
            "selected",
            "--state-dir",
            state.to_str().unwrap(),
        ])
        .output()
        .expect("run plan");
    let plan_text = format!(
        "{}{}",
        String::from_utf8_lossy(&plan_out.stdout),
        String::from_utf8_lossy(&plan_out.stderr)
    );
    let Some(planned) = plan_count(&plan_text) else {
        panic!("could not read plan count from:\n{plan_text}");
    };

    let applied = apply_declined(&cfg, &state);
    let Some(offered) = prompt_count(&applied) else {
        panic!("could not read prompt count from:\n{applied}");
    };

    assert_eq!(
        planned, offered,
        "plan promised {planned} change(s) but apply offered {offered} under \
         the same `-r selected`.\nplan:\n{plan_text}\napply:\n{applied}"
    );
}

#[test]
fn an_unscoped_apply_still_offers_everything() {
    // The fix must not become an over-correction. With no `-r`, all four
    // resources are in scope and must still be counted — otherwise the
    // regression is traded for the opposite one, under-reporting a full apply.
    let dir = tempfile::tempdir().expect("tmpdir");
    let state = dir.path().join("state");
    std::fs::create_dir_all(&state).expect("state dir");
    let cfg = config_with_decoys(dir.path());

    let mut child = forjar()
        .args([
            "apply",
            "-f",
            cfg.to_str().unwrap(),
            "--state-dir",
            state.to_str().unwrap(),
            "--no-tripwire",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn apply");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"n\n")
        .expect("decline");
    let out = child.wait_with_output().expect("wait");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let Some(n) = prompt_count(&combined) else {
        panic!("no confirmation prompt in output:\n{combined}");
    };
    assert_eq!(
        n, 4,
        "an unscoped apply must still offer every declared resource.\noutput:\n{combined}"
    );
}
