//! PMAT-225: the daily release-goal workflow has the shape that makes a missed
//! release visible — a schedule, gate T with a token, one issue kept open while
//! red and closed when green, nothing swallowed. Each rule is asserted against
//! the YAML GitHub parses, never against a description of it.

use std::path::{Path, PathBuf};

const WORKFLOW: &str = ".github/workflows/release-goal.yml";

fn read() -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(WORKFLOW);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

fn yaml() -> serde_yaml_ng::Value {
    serde_yaml_ng::from_str(&read()).unwrap_or_else(|e| panic!("parse {WORKFLOW}: {e}"))
}

fn steps() -> Vec<serde_yaml_ng::Value> {
    yaml()["jobs"]["release-goal"]["steps"]
        .as_sequence()
        .expect("jobs.release-goal.steps is a sequence")
        .to_vec()
}

fn step_running(needle: &str) -> serde_yaml_ng::Value {
    steps()
        .into_iter()
        .find(|s| s["run"].as_str().is_some_and(|r| r.contains(needle)))
        .unwrap_or_else(|| panic!("no step runs {needle}"))
}

#[test]
fn rule_1_it_runs_every_day_and_on_demand() {
    let y = yaml();
    let cron = y["on"]["schedule"][0]["cron"]
        .as_str()
        .expect("on.schedule[0].cron");
    let fields: Vec<&str> = cron.split_whitespace().collect();
    assert_eq!(fields.len(), 5, "a cron has five fields: {cron}");
    assert_eq!(
        (fields[2], fields[3], fields[4]),
        ("*", "*", "*"),
        "daily: day-of-month, month and day-of-week are all `*` in {cron} — a cadence of \
         two days is measured by the gate from the tag's age, not by firing every other day"
    );
    assert!(
        y["on"]["workflow_dispatch"].is_mapping() || y["on"]["workflow_dispatch"].is_null(),
        "workflow_dispatch is present"
    );
    assert!(
        read().contains("workflow_dispatch:"),
        "an operator can fire it by hand"
    );
}

#[test]
fn rule_2_gate_t_runs_first_with_a_token_and_a_full_clone() {
    let gate = step_running("scripts/dogfood/tagged.sh");
    assert!(
        gate["env"]["GH_TOKEN"].as_str().is_some(),
        "the gate asks GitHub for the PR windows and needs GH_TOKEN"
    );
    let checkout = steps()
        .into_iter()
        .find(|s| {
            s["uses"]
                .as_str()
                .is_some_and(|u| u.starts_with("actions/checkout@"))
        })
        .expect("a checkout step");
    assert_eq!(
        checkout["with"]["fetch-depth"].as_u64(),
        Some(0),
        "ancestry needs the whole history"
    );
    assert_eq!(
        checkout["with"]["fetch-tags"].as_bool(),
        Some(true),
        "the tags are the measured side"
    );
    let gate_index = steps()
        .iter()
        .position(|s| s["run"].as_str().is_some_and(|r| r.contains("tagged.sh")))
        .expect("gate T");
    let build_index = steps()
        .iter()
        .position(|s| s["run"].as_str().is_some_and(|r| r.contains("cargo build")))
        .expect("the build");
    assert!(
        gate_index < build_index,
        "gate T takes seconds and runs before the release build"
    );
}

#[test]
fn rule_3_one_issue_is_kept_open_while_red_and_closed_when_green() {
    let red = steps()
        .into_iter()
        .find(|s| s["if"].as_str() == Some("failure()"))
        .expect("a step that runs on failure");
    let run = red["run"].as_str().expect("run");
    assert!(
        run.contains("--label release-goal") && run.contains("gh issue create"),
        "red creates the labelled issue"
    );
    assert!(
        run.contains("gh issue comment"),
        "red comments on the open one instead of opening a second"
    );
    let green = steps()
        .into_iter()
        .find(|s| s["if"].as_str() == Some("success()"))
        .expect("a step that runs on success");
    assert!(
        green["run"]
            .as_str()
            .is_some_and(|r| r.contains("gh issue close")),
        "green closes it"
    );
    assert_eq!(
        yaml()["permissions"]["issues"].as_str(),
        Some("write"),
        "the token can write issues"
    );
}

#[test]
fn rule_4_nothing_is_swallowed() {
    // Comments may NAME the forbidden shapes (the header does); code may not use them.
    for line in read().lines() {
        let l = line.trim();
        if l.starts_with('#') {
            continue;
        }
        assert!(
            !l.contains("continue-on-error"),
            "a red gate is a red run: {l}"
        );
        assert!(!l.contains("|| true"), "no measurement is forgiven: {l}");
    }
    let gate = step_running("scripts/dogfood/tagged.sh");
    assert!(
        gate["run"]
            .as_str()
            .is_some_and(|r| r.contains("set -o pipefail")),
        "tee must not hide the gate's exit code"
    );
}

#[test]
fn rule_5_the_gate_and_the_dogfood_half_that_a_hosted_runner_can_run() {
    let _ = step_running("scripts/dogfood/surface.sh");
    let _ = step_running("scripts/dogfood/docs.sh");
    let p: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/dogfood/tagged.sh");
    assert!(p.is_file(), "the gate the workflow runs exists");
}
