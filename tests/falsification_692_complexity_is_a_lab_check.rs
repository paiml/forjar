//! The complexity check is a LAB check: it measures every night and can never
//! block a merge or a release.
//!
//! forjar#692. The generated pre-commit hook refuses a touched function that
//! grew past the pmat.toml limits, but the hook is opt-in per clone, so #690
//! reached main with `toolchain_gaps` at Cognitive 39 > 25 and the next PR to
//! merge main had its merge commit refused for a function it never wrote.
//! .github/workflows/complexity-lab.yml asks the hook's question of main every
//! night. These tests pin the shape that keeps it a LAB check, PARSED rather
//! than grepped where the YAML carries the meaning:
//!
//! * exactly one workflow runs scripts/ci/complexity-diff-scope.sh, and its
//!   only trigger is `schedule` -- no pull_request, merge queue, release or
//!   dispatch;
//! * no other workflow names the script, so no gate can need it;
//! * nothing in it is swallowed (`continue-on-error`, `|| true`);
//! * red keeps ONE issue and edits it in place instead of commenting, green
//!   closes it, and a 7-day stop exists;
//! * the limits live in pmat.toml `[quality]` -- the file `pmat hooks`
//!   generates the hook from -- and the script carries no number of its own.
//!
//! What the script DECIDES is proven by running it, not here: red on #690
//! (Cognitive 39 > 25), green on #693, and green on a branch that merges #690
//! in, because the diff starts at the merge base.

use serde_yaml_ng::Value;
use std::path::Path;

const SCRIPT: &str = "scripts/ci/complexity-diff-scope.sh";
const WORKFLOW: &str = ".github/workflows/complexity-lab.yml";

fn repo_file(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{rel} must be readable: {e}"))
}

fn workflows() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows");
    let mut out: Vec<(String, String)> = std::fs::read_dir(&dir)
        .expect(".github/workflows must be readable")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml" || x == "yaml"))
        .map(|p| {
            let rel = format!(
                ".github/workflows/{}",
                p.file_name().unwrap_or_default().to_string_lossy()
            );
            let text = std::fs::read_to_string(&p).unwrap_or_default();
            (rel, text)
        })
        .collect();
    out.sort();
    out
}

fn lab() -> Value {
    serde_yaml_ng::from_str(&repo_file(WORKFLOW)).expect("complexity-lab.yml must parse as YAML")
}

fn steps(w: &Value) -> Vec<Value> {
    w["jobs"]
        .as_mapping()
        .expect("complexity-lab.yml must define `jobs`")
        .values()
        .flat_map(|job| job["steps"].as_sequence().cloned().unwrap_or_default())
        .collect()
}

fn run_of(step: &Value) -> &str {
    step["run"].as_str().unwrap_or_default()
}

#[test]
fn exactly_one_workflow_runs_the_script_and_only_on_a_schedule() {
    let naming: Vec<String> = workflows()
        .into_iter()
        .filter(|(_, text)| text.contains(SCRIPT))
        .map(|(rel, _)| rel)
        .collect();
    assert_eq!(
        naming,
        vec![WORKFLOW.to_string()],
        "only the LAB workflow may name {SCRIPT}; a PR, merge-queue or release workflow that runs it makes it a GATE"
    );

    let w = lab();
    let triggers: Vec<String> = w["on"]
        .as_mapping()
        .expect("complexity-lab.yml `on` must be a mapping")
        .keys()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect();
    assert_eq!(
        triggers,
        vec!["schedule".to_string()],
        "a LAB check runs at night only, never on a PR, the merge queue, a release or a dispatch"
    );
    assert!(
        steps(&w).iter().any(|s| run_of(s).contains(SCRIPT)),
        "complexity-lab.yml must run {SCRIPT} in a step, not only mention it"
    );
}

#[test]
fn nothing_in_the_lab_workflow_is_swallowed() {
    for (n, line) in repo_file(WORKFLOW).lines().enumerate() {
        let code = line.split('#').next().unwrap_or_default();
        assert!(
            !code.contains("continue-on-error") && !code.contains("|| true"),
            "{WORKFLOW}:{}: a third state on a red check is a waiver: {line}",
            n + 1
        );
    }
}

#[test]
fn red_keeps_one_issue_edited_in_place_and_green_closes_it() {
    let w = lab();
    let all = steps(&w);
    let red = all
        .iter()
        .find(|s| s["if"].as_str().is_some_and(|c| c.contains("failure()")))
        .expect("a step that runs on failure");
    let r = run_of(red);
    assert!(
        r.contains("gh issue edit"),
        "red must update the open issue's body"
    );
    assert!(
        r.contains("gh issue create"),
        "red with no open issue must open one"
    );
    assert!(
        !r.contains("gh issue comment"),
        "red must stay silent: edit the body, never comment night after night"
    );

    let green = all
        .iter()
        .find(|s| s["if"].as_str().is_some_and(|c| c.contains("success()")))
        .expect("a step that runs on success");
    assert!(
        run_of(green).contains("gh issue close"),
        "green must close the issue"
    );

    assert!(
        all.iter().any(|s| run_of(s).contains("7 * 86400")),
        "red or unmeasured for 7 days, the check must stop until its owner closes the issue"
    );
}

#[test]
fn the_limits_live_in_pmat_toml_and_nowhere_else() {
    let table: toml::Table = repo_file("pmat.toml")
        .parse()
        .expect("pmat.toml must parse");
    let quality = table
        .get("quality")
        .and_then(toml::Value::as_table)
        .expect("pmat.toml must have [quality] -- `pmat hooks` reads the hook's limits from it");
    for key in ["max_complexity", "max_cognitive_complexity"] {
        let v = quality.get(key).and_then(toml::Value::as_integer);
        assert!(
            v.is_some_and(|v| v > 0),
            "pmat.toml [quality] {key} must be a positive integer, got {v:?}"
        );
    }

    let script = repo_file(SCRIPT);
    assert!(
        script.contains("pmat.toml"),
        "the script must read pmat.toml"
    );
    for flag in ["--max-cyclomatic", "--max-cognitive"] {
        let line = script
            .lines()
            .find(|l| l.contains(flag))
            .unwrap_or_else(|| panic!("the script must pass {flag}"));
        let value = line.split(flag).nth(1).unwrap_or_default().trim_start();
        assert!(
            value.starts_with("\"$max_"),
            "{flag} must take the value read from pmat.toml, not a literal: {line}"
        );
    }
    assert!(
        script.contains("--diff-scope"),
        "the script must ask the hook's question (--diff-scope), or it bills a change for debt it did not write"
    );
}
