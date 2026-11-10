//! infra#3828: mutation runs on `schedule` only. Operator ruling 2026-11-10
//! 16:50Z: "NO build can do mutation outside of nightly; the end. FOREVER".
//!
//! Gate F measures the 95% coverage floor and nothing else, and no workflow
//! other than mutation.yml names `cargo mutants`; mutation.yml itself triggers
//! on `schedule` alone.

use std::path::PathBuf;

fn read(rel: &str) -> String {
    std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel))
        .unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn code_lines(src: &str) -> Vec<&str> {
    src.lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect()
}

#[test]
fn gate_f_never_invokes_cargo_mutants() {
    let gate = read("scripts/dogfood/coverage.sh");
    for l in code_lines(&gate) {
        assert!(
            !l.contains("cargo mutants"),
            "gate F runs mutation outside nightly: {l}"
        );
    }
}

#[test]
fn mutation_workflow_triggers_on_schedule_only() {
    let wf = read(".github/workflows/mutation.yml");
    let on: String = wf
        .lines()
        .skip_while(|l| !l.starts_with("on:"))
        .skip(1)
        .take_while(|l| l.starts_with(' ') || l.starts_with('#') || l.is_empty())
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        on.contains("schedule:"),
        "mutation.yml has no schedule: {on}"
    );
    for banned in [
        "push:",
        "pull_request",
        "merge_group",
        "workflow_dispatch",
        "workflow_call",
        "train/",
    ] {
        assert!(
            !on.contains(banned),
            "mutation.yml triggers on {banned}: {on}"
        );
    }
}

#[test]
fn no_other_workflow_or_gate_runs_or_requires_mutation() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".github/workflows");
    for e in std::fs::read_dir(dir).expect("workflows dir") {
        let p = e.expect("entry").path();
        if p.file_name().is_some_and(|n| n == "mutation.yml") {
            continue;
        }
        let Ok(body) = std::fs::read_to_string(&p) else {
            continue;
        };
        for l in code_lines(&body) {
            assert!(
                !l.contains("cargo mutants")
                    && !l.contains("cargo-mutants")
                    && !l.contains("mutation.yml"),
                "{} runs or depends on mutation: {l}",
                p.display()
            );
        }
    }
}
