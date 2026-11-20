//! The three light PR jobs (lockfile, examples-validate, file-health) read
//! nothing but the repository, so each carries `permissions: contents: read`
//! and checks out with `persist-credentials: false`. `violations` judges the
//! parsed YAML (never a comment, never a re-serialised string):
//! - job `permissions` are exactly `contents: read`; a workflow-level
//!   `permissions` is absent or no wider than read;
//! - no `continue-on-error` key on the job or on any step;
//! - the job-level `if` is the YAML value the job already had (absent stays
//!   absent; a bool or number is a failure);
//! - every checkout step (`uses` matched case-insensitively) sets
//!   `persist-credentials` to the YAML bool false, and there is exactly one;
//! - `gate` needs both ci.yml jobs and reads both results on a code line;
//! - examples-validate runs this target as an exact `--test` token, on a code
//!   line, in a step with no `if` and no `continue-on-error`.
//!
//! mutation: each row of `MUTANTS` edits the workflow text in memory and the
//! checker must name the violation; the unmutated trees must be clean.

use serde_yaml_ng::Value;

const TARGET: &str = "falsification_ci_light_jobs_are_hardened";
const CODE_IF: &str = "needs.classify.outputs.code == 'true'";

fn read(rel: &str) -> String {
    let p = format!("{}/{rel}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {p}: {e}"))
}

fn parse(text: &str) -> Value {
    serde_yaml_ng::from_str(text).expect("workflow parses")
}

/// Script lines that are code: continuations joined, comment lines dropped.
fn code_lines(run: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut joined = String::new();
    for raw in run.lines() {
        let t = raw.trim();
        if t.starts_with('#') {
            continue;
        }
        if let Some(head) = t.strip_suffix('\\') {
            joined.push_str(head);
            joined.push(' ');
        } else {
            joined.push_str(t);
            out.push(std::mem::take(&mut joined));
        }
    }
    out
}

fn steps(job: &Value) -> Vec<&Value> {
    job["steps"]
        .as_sequence()
        .map_or(vec![], |s| s.iter().collect())
}

fn runs_target(step: &Value) -> bool {
    let run = step["run"].as_str().unwrap_or("");
    code_lines(run).iter().any(|l| {
        let tok: Vec<&str> = l.split_whitespace().collect();
        tok.windows(2).any(|w| w == ["--test", TARGET])
    })
}

fn wide_permissions(doc: &Value) -> bool {
    match doc.get("permissions") {
        None => false,
        Some(Value::String(s)) => s != "read-all",
        Some(Value::Mapping(m)) => m
            .values()
            .any(|v| !matches!(v.as_str(), Some("read" | "none"))),
        Some(_) => true,
    }
}

fn check_job(doc: &Value, file: &str, name: &str, want_if: Option<&str>, out: &mut Vec<String>) {
    let job = &doc["jobs"][name];
    let mut want = serde_yaml_ng::Mapping::new();
    want.insert("contents".into(), "read".into());
    if job["permissions"] != Value::Mapping(want) {
        out.push(format!("{name}: permissions"));
    }
    if job.get("continue-on-error").is_some() {
        out.push(format!("{name}: continue-on-error (job)"));
    }
    if steps(job)
        .iter()
        .any(|s| s.get("continue-on-error").is_some())
    {
        out.push(format!("{name}: continue-on-error (step)"));
    }
    if job.get("if") != want_if.map(|s| Value::String(s.into())).as_ref() {
        out.push(format!("{name}: if"));
    }
    let checkouts: Vec<&Value> = steps(job)
        .into_iter()
        .filter(|s| {
            s["uses"]
                .as_str()
                .is_some_and(|u| u.to_ascii_lowercase().starts_with("actions/checkout@"))
        })
        .collect();
    if checkouts.len() != 1 {
        out.push(format!("{name}: checkout count {}", checkouts.len()));
    }
    if checkouts
        .iter()
        .any(|s| s["with"]["persist-credentials"] != Value::Bool(false))
    {
        out.push(format!("{name}: persist-credentials"));
    }
    if wide_permissions(doc) {
        out.push(format!("{file}: workflow permissions"));
    }
}

fn violations(ci: &str, lint: &str) -> Vec<String> {
    let (ci, lint) = (parse(ci), parse(lint));
    let mut out = Vec::new();
    check_job(&ci, "ci.yml", "lockfile", None, &mut out);
    check_job(&ci, "ci.yml", "examples-validate", Some(CODE_IF), &mut out);
    check_job(&lint, "lint.yml", "file-health", None, &mut out);

    let gate = &ci["jobs"]["gate"];
    let needs = gate["needs"].as_sequence().cloned().unwrap_or_default();
    for j in ["lockfile", "examples-validate"] {
        if !needs.iter().any(|n| n.as_str() == Some(j)) {
            out.push(format!("gate: does not need {j}"));
        }
        let code: Vec<String> = steps(gate)
            .iter()
            .flat_map(|s| code_lines(s["run"].as_str().unwrap_or("")))
            .collect();
        if !code
            .iter()
            .any(|l| l.contains(&format!("needs.{j}.result")))
        {
            out.push(format!("gate: does not read {j} result"));
        }
    }
    if gate.get("continue-on-error").is_some() {
        out.push("gate: continue-on-error".into());
    }
    let ev = &ci["jobs"]["examples-validate"];
    let runs = steps(ev)
        .into_iter()
        .any(|s| runs_target(s) && s.get("if").is_none() && s.get("continue-on-error").is_none());
    if !runs {
        out.push("examples-validate: does not run the target".into());
    }
    out
}

#[test]
fn the_light_jobs_are_hardened() {
    let v = violations(
        &read(".github/workflows/ci.yml"),
        &read(".github/workflows/lint.yml"),
    );
    assert!(v.is_empty(), "{v:#?}");
}

const SHA: &str = "3d3c42e5aac5ba805825da76410c181273ba90b1";

/// (name, file, old text, new text, replace every occurrence, expected violation)
type Mutant = (
    &'static str,
    &'static str,
    String,
    String,
    bool,
    &'static str,
);

fn mutants() -> Vec<Mutant> {
    let a = "    name: lockfile-preflight\n";
    let v = "      - name: Verify Cargo.lock is committed and current\n";
    let ex = format!("      - uses: Actions/checkout@{SHA}\n");
    let guards = "      - name: Integration guards (examples, orphans, undo, quorum gates, nightly, version)\n";
    let last = format!("          --test falsification_pr_jobs_name_no_arm64_or_host \\\n          --test {TARGET}");
    let ci = "ci";
    let li = "lint";
    vec![
        ("B1 lockfile if: false (bool)", ci, a.into(), format!("{a}    if: false\n"), false, "lockfile: if"),
        ("B1 file-health if: false (bool)", li, "  file-health:\n    timeout-minutes: 15\n".into(), "  file-health:\n    timeout-minutes: 15\n    if: 0\n".into(), false, "file-health: if"),
        ("B1 examples-validate if loosened to bool", ci, "    if: needs.classify.outputs.code == 'true'\n    needs: classify\n    name: examples-validate".into(), "    if: true\n    needs: classify\n    name: examples-validate".into(), false, "examples-validate: if"),
        ("B2 target commented out", ci, last.clone(), format!("          --test falsification_pr_jobs_name_no_arm64_or_host\n          # --test {TARGET}"), false, "does not run the target"),
        ("B2 target renamed (prefix match)", ci, format!("--test {TARGET}"), format!("--test {TARGET}_noop"), false, "does not run the target"),
        ("B2 target step disabled by if: false", ci, guards.into(), format!("{guards}        if: false\n"), false, "does not run the target"),
        ("B2 target removed from the list", ci, last.clone(), "          --test falsification_pr_jobs_name_no_arm64_or_host".into(), false, "does not run the target"),
        ("B3 mixed-case second checkout", ci, v.into(), format!("{ex}{v}"), false, "checkout count 2"),
        ("B3 file-health persist-credentials as string", li, "persist-credentials: false".into(), "persist-credentials: \"false\"".into(), false, "file-health: persist-credentials"),
        ("B3 lockfile persist-credentials true", ci, "        with:\n          persist-credentials: false\n      - name: Verify Cargo.lock".into(), "        with:\n          persist-credentials: true\n      - name: Verify Cargo.lock".into(), false, "lockfile: persist-credentials"),
        ("perm file-health widened", li, "      contents: read\n".into(), "      contents: write\n".into(), false, "file-health: permissions"),
        ("perm workflow-level write-all", ci, "\nconcurrency:\n".into(), "\npermissions: write-all\n\nconcurrency:\n".into(), false, "workflow permissions"),
        ("coe job-level", ci, a.into(), format!("{a}    continue-on-error: true\n"), false, "lockfile: continue-on-error (job)"),
        ("coe step-level", ci, v.into(), format!("{v}        continue-on-error: true\n"), false, "lockfile: continue-on-error (step)"),
        ("gate drops lockfile from needs", ci, "needs: [classify, ci, lockfile, ".into(), "needs: [classify, ci, ".into(), false, "gate: does not need lockfile"),
        ("gate stops reading lockfile result", ci, "needs.lockfile.result".into(), "needs.classify.result".into(), true, "gate: does not read lockfile result"),
        ("gate stops reading examples-validate result", ci, "needs.examples-validate.result".into(), "needs.classify.result".into(), true, "gate: does not read examples-validate result"),
    ]
}

#[test]
fn every_mutant_is_refused() {
    let (ci, lint) = (
        read(".github/workflows/ci.yml"),
        read(".github/workflows/lint.yml"),
    );
    let table = mutants();
    assert!(table.len() >= 10, "{} mutants", table.len());
    for (name, file, old, new, all, want) in table {
        let src = if file == "ci" { &ci } else { &lint };
        let n = src.matches(old.as_str()).count();
        assert!(
            n >= 1 && (all || n == 1),
            "{name}: old text occurs {n} times"
        );
        let mutated = src.replace(old.as_str(), &new);
        let (c, l) = if file == "ci" {
            (mutated.as_str(), lint.as_str())
        } else {
            (ci.as_str(), mutated.as_str())
        };
        let got = violations(c, l);
        assert!(
            got.iter().any(|g| g.contains(want)),
            "{name}: wanted `{want}`, got {got:?}"
        );
    }
}

#[test]
fn comments_and_false_words_do_not_trip_the_checker() {
    let (ci, lint) = (
        read(".github/workflows/ci.yml"),
        read(".github/workflows/lint.yml"),
    );
    let a = "    name: lockfile-preflight\n";
    let ci2 = ci.replacen(a, &format!("{a}    # no continue-on-error here\n"), 1);
    assert!(violations(&ci2, &lint).is_empty());
    let guards = "      - name: Integration guards (examples, orphans, undo, quorum gates, nightly, version)\n";
    let ci3 = ci.replacen(
        guards,
        "      - name: Integration guards (--test x_hardened)\n",
        1,
    );
    assert!(violations(&ci3, &lint).is_empty());
}
