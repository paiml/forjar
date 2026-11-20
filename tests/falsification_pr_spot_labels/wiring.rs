//! The gate step must run the parsed check, and nothing may skip it, mask it or narrow it.

use crate::*;

pub(crate) fn ci_value() -> Value {
    serde_yaml_ng::from_str(&ci_text()).unwrap()
}

const CMD: &str = "cargo test --locked --test falsification_pr_spot_labels --test falsification_ci_reusable_jobs_route_to_x64";
const GUARD_KEYS: [&str; 5] = ["timeout-minutes", "runs-on", "permissions", "env", "steps"];

/// Exact wiring: the job has no `if`/`continue-on-error`/extra keys; its steps are the checkout and
/// ONE step whose whole command is the guard test (so no `||`, `;`, pipe, filter or `if:` fits);
/// `gate` needs the job and reads its result.
pub(crate) fn check_ci_wiring(ci: &Value) -> Result<(), String> {
    let jobs = ci.get("jobs").ok_or("no jobs")?;
    let g = jobs
        .get("spot-labels-guard")
        .ok_or("no spot-labels-guard job")?;
    let gm = g.as_mapping().ok_or("guard job is not a mapping")?;
    if let Some(k) = gm
        .keys()
        .filter_map(Value::as_str)
        .find(|k| !GUARD_KEYS.contains(k))
    {
        return Err(format!(
            "guard job carries `{k}`, which can skip or mask it"
        ));
    }
    let steps = g
        .get("steps")
        .and_then(Value::as_sequence)
        .ok_or("no steps")?;
    if steps.len() != 2 {
        return Err(format!(
            "guard job has {} steps, want checkout + the test",
            steps.len()
        ));
    }
    let first = steps[0].as_mapping().ok_or("step 0 is not a mapping")?;
    let uses = first.get("uses").and_then(Value::as_str).unwrap_or("");
    if !uses.starts_with("actions/checkout@")
        || first.keys().filter_map(Value::as_str).any(|k| k != "uses")
    {
        return Err("step 0 is not a bare actions/checkout".into());
    }
    let second = steps[1].as_mapping().ok_or("step 1 is not a mapping")?;
    if let Some(k) = second
        .keys()
        .filter_map(Value::as_str)
        .find(|k| !["name", "run"].contains(k))
    {
        return Err(format!(
            "the guard step carries `{k}`, which can skip or mask it"
        ));
    }
    if steps[1].get("run").and_then(Value::as_str).map(str::trim) != Some(CMD) {
        return Err(format!("the guard step is not exactly `{CMD}`"));
    }
    let gate = jobs.get("gate").ok_or("no gate job")?;
    if gate.get("continue-on-error").is_some() {
        return Err("gate continue-on-error".into());
    }
    let needs: Vec<&str> = gate
        .get("needs")
        .and_then(Value::as_sequence)
        .ok_or("gate has no needs list")?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    if !needs.contains(&"spot-labels-guard") {
        return Err("gate does not need the guard job".into());
    }
    let gate_text = serde_yaml_ng::to_string(gate).unwrap();
    if !gate_text.contains("needs.spot-labels-guard.result") {
        return Err("gate never reads the guard result".into());
    }
    Ok(())
}

#[test]
fn ci_wires_the_parsed_guard_unmasked_into_the_gate() {
    if let Err(e) = check_ci_wiring(&ci_value()) {
        panic!("{e}");
    }
}

const STEP_NAME: &str =
    "      - name: Parsed guard (judges the parsed workflows; exit code unmasked)\n";

/// Each mutant is applied to the real ci.yml text and handed to the REAL validator.
fn mutants() -> Vec<(&'static str, String)> {
    let ci = ci_text();
    let run = |to: &str| ci.replacen(CMD, to, 1);
    let after_name = |add: &str| ci.replacen(STEP_NAME, &format!("{STEP_NAME}{add}"), 1);
    let job_head = "  spot-labels-guard:\n    timeout-minutes: 45\n";
    vec![
        ("or-true", run(&format!("{CMD} || true"))),
        ("or-echo", run(&format!("{CMD} || echo ok"))),
        ("zero-tests", run(&format!("{CMD} __match_zero_tests__"))),
        ("semicolon", run(&format!("{CMD}; true"))),
        ("pipe-cat", run(&format!("{CMD} | cat"))),
        ("pipe-tail", run(&format!("{CMD} 2>&1 | tail -n 1"))),
        ("and-and", run(&format!("{CMD} && true"))),
        ("exit-0", run(&format!("{CMD}\n          exit 0"))),
        (
            "other-test",
            run("cargo test --locked --test falsification_other"),
        ),
        ("true-only", run("true")),
        (
            "no-locked",
            run("cargo test --test falsification_pr_spot_labels"),
        ),
        ("step-if-false", after_name("        if: false\n")),
        (
            "step-if-expr",
            after_name("        if: ${{ always() && false }}\n"),
        ),
        ("step-coe", after_name("        continue-on-error: true\n")),
        ("step-shell", after_name("        shell: bash {0}\n")),
        (
            "step-env",
            after_name("        env:\n          RUSTFLAGS: --cfg x\n"),
        ),
        (
            "job-if-false",
            ci.replacen(job_head, &format!("{job_head}    if: false\n"), 1),
        ),
        (
            "job-coe",
            ci.replacen(
                job_head,
                &format!("{job_head}    continue-on-error: true\n"),
                1,
            ),
        ),
        (
            "gate-drops-need",
            ci.replacen(", spot-labels-guard]", "]", 1),
        ),
        (
            "gate-drops-read",
            ci.replace("needs.spot-labels-guard.result", "needs.classify.result"),
        ),
    ]
}

#[test]
fn ci_step_masking_mutants_are_refused() {
    check_ci_wiring(&ci_value()).expect("the real ci.yml must pass the validator first");
    for (name, text) in mutants() {
        assert_ne!(text, ci_text(), "mutant {name} did not apply");
        let v: Value = serde_yaml_ng::from_str(&text)
            .unwrap_or_else(|e| panic!("mutant {name} does not parse: {e}"));
        assert!(
            check_ci_wiring(&v).is_err(),
            "the wiring validator accepted mutant {name}"
        );
    }
}
