//! Tests: Coverage for apply_mode_exits / apply_early_exits / apply_pre_checks
//! in dispatch_apply_b.rs, driven through dispatch_apply_cmd (PMAT-088).

use super::commands::*;
use super::dispatch_apply_b::dispatch_apply_cmd;
use super::helpers::parse_and_validate;
use super::helpers_state::load_machine_locks;
use super::plan_file::save_plan_file;
use super::workspace::inject_workspace_param;
use crate::core::{planner, resolver};
use std::path::{Path, PathBuf};

/// Local clap wrapper so tests build ApplyArgs from CLI strings.
#[derive(clap::Parser)]
#[command(name = "forjar")]
struct TestCli {
    #[command(subcommand)]
    cmd: Commands,
}

fn apply_cmd(extra: &[String]) -> Commands {
    let mut argv: Vec<String> = vec!["forjar".into(), "apply".into()];
    argv.extend(extra.iter().cloned());
    let cli = <TestCli as clap::Parser>::try_parse_from(argv).expect("clap parse");
    cli.cmd
}

/// Dispatch on a 16 MiB stack — the apply pipeline's frames exceed the
/// default test-thread stack (same pattern as tests_cov_dispatch_5).
fn run_apply(extra: &[&str]) -> Result<(), String> {
    let argv: Vec<String> = extra.iter().map(|s| s.to_string()).collect();
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || dispatch_apply_cmd(apply_cmd(&argv), false))
        .expect("spawn apply thread")
        .join()
        .expect("join apply thread")
}

/// Write a single-machine localhost config whose file resource targets `target`.
fn write_cfg(dir: &Path, target: &Path) -> PathBuf {
    let yaml = format!(
        "version: \"1.0\"\nname: modes\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: {}\n    content: hello\n",
        target.display()
    );
    let p = dir.join("forjar.yaml");
    std::fs::write(&p, yaml).unwrap();
    p
}

fn setup() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("out.txt");
    let cfg = write_cfg(dir.path(), &target);
    let sd = dir.path().join("state");
    std::fs::create_dir_all(&sd).unwrap();
    (dir, cfg, sd)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(p: &Path) -> String {
        p.display().to_string()
    }

    // ── apply_mode_exits branches ──

    // GH-210: `--preview` used to short-circuit into the plan renderer and
    // return Ok(()) having converged NOTHING and written no state. It now
    // prints the generated scripts and then performs the apply, so these tests
    // assert the apply really happened rather than that a no-op exited 0.
    #[test]
    fn mode_preview_shows_scripts_and_then_applies() {
        let (d, cfg, sd) = setup();
        let target = d.path().join("out.txt");
        let r = run_apply(&["-f", &s(&cfg), "--state-dir", &s(&sd), "--preview", "--yes"]);
        assert!(r.is_ok(), "{r:?}");
        assert!(
            target.exists(),
            "--preview is documented as showing scripts BEFORE execution; \
             the execution must follow"
        );
    }

    #[test]
    fn mode_output_scripts_ok() {
        let (d, cfg, sd) = setup();
        let scripts = d.path().join("scripts");
        let r = run_apply(&[
            "-f",
            &s(&cfg),
            "--state-dir",
            &s(&sd),
            "--output-scripts",
            &s(&scripts),
        ]);
        assert!(r.is_ok());
        assert!(scripts.exists(), "script dir should be created");
    }

    #[test]
    fn mode_diff_only_ok() {
        let (_d, cfg, sd) = setup();
        let r = run_apply(&["-f", &s(&cfg), "--state-dir", &s(&sd), "--diff-only"]);
        assert!(r.is_ok());
    }

    #[test]
    fn mode_diff_only_json_ok() {
        let (_d, cfg, sd) = setup();
        let r = run_apply(&[
            "-f",
            &s(&cfg),
            "--state-dir",
            &s(&sd),
            "--diff-only",
            "--json",
        ]);
        assert!(r.is_ok());
    }

    #[test]
    fn mode_check_after_apply_ok() {
        let (_d, cfg, sd) = setup();
        let applied = run_apply(&["-f", &s(&cfg), "--state-dir", &s(&sd), "--yes"]);
        assert!(applied.is_ok(), "full apply should converge: {applied:?}");
        let r = run_apply(&["-f", &s(&cfg), "--state-dir", &s(&sd), "--check"]);
        assert!(r.is_ok(), "check after converge should pass: {r:?}");
    }

    #[test]
    fn mode_check_json_ok() {
        let (_d, cfg, sd) = setup();
        // FJ-2720: check scripts are no longer observational — the verdict is
        // the exit code. Nothing has been applied in this fixture, so the JSON
        // branch must report divergence.
        let r = run_apply(&["-f", &s(&cfg), "--state-dir", &s(&sd), "--check", "--json"]);
        assert!(r.is_err(), "nothing applied yet, so check must not pass: {r:?}");
    }

    #[test]
    fn mode_check_invalid_config_err() {
        let (d, _cfg, sd) = setup();
        let bad = d.path().join("bad.yaml");
        std::fs::write(&bad, "not: valid: yaml: [[[").unwrap();
        let r = run_apply(&["-f", &s(&bad), "--state-dir", &s(&sd), "--check"]);
        assert!(r.is_err());
    }

    #[test]
    fn mode_refresh_only_empty_state_ok() {
        let (_d, cfg, sd) = setup();
        let r = run_apply(&["-f", &s(&cfg), "--state-dir", &s(&sd), "--refresh-only"]);
        assert!(r.is_ok());
    }

    #[test]
    fn mode_plan_file_roundtrip_ok() {
        let (d, cfg, sd) = setup();
        // Build the plan exactly as cmd_apply_from_plan reconstructs the config.
        let mut config = parse_and_validate(&cfg).unwrap();
        inject_workspace_param(&mut config, Some("pfws"));
        resolver::resolve_data_sources(&mut config).unwrap();
        let order = resolver::build_execution_order(&config).unwrap();
        let locks = load_machine_locks(&config, &sd, None).unwrap();
        let plan = planner::plan(&config, &order, &locks, None);
        let plan_path = d.path().join("plan.json");
        save_plan_file(
            &plan,
            &crate::core::plan_selectors::PlanSelectors::default(),
            &config,
            &cfg,
            &sd,
            &plan_path,
        )
        .unwrap();

        let r = run_apply(&[
            "-f",
            &s(&cfg),
            "--state-dir",
            &s(&sd),
            "--workspace",
            "pfws",
            "--plan-file",
            &s(&plan_path),
        ]);
        assert!(r.is_ok(), "plan-file apply should succeed: {r:?}");
    }

    // ── apply_early_exits branches ──

    #[test]
    fn early_dry_run_verbose_ok() {
        let (_d, cfg, sd) = setup();
        let r = run_apply(&["-f", &s(&cfg), "--state-dir", &s(&sd), "--dry-run-verbose"]);
        assert!(r.is_ok());
    }

    #[test]
    fn early_dry_run_graph_ok() {
        let (_d, cfg, sd) = setup();
        let r = run_apply(&["-f", &s(&cfg), "--state-dir", &s(&sd), "--dry-run-graph"]);
        assert!(r.is_ok());
    }

    #[test]
    fn early_dry_run_cost_ok() {
        let (_d, cfg, sd) = setup();
        let r = run_apply(&["-f", &s(&cfg), "--state-dir", &s(&sd), "--dry-run-cost"]);
        assert!(r.is_ok());
    }

    #[test]
    fn early_canary_machine_single_ok() {
        let (_d, cfg, sd) = setup();
        // Single machine → canary converges, "No remaining machines" path.
        let r = run_apply(&[
            "-f",
            &s(&cfg),
            "--state-dir",
            &s(&sd),
            "--canary-machine",
            "m",
        ]);
        assert!(r.is_ok(), "canary on single machine should pass: {r:?}");
    }

    // ── apply_pre_checks branches ──

    #[test]
    fn pre_confirmation_and_preflight_ok() {
        let (_d, cfg, sd) = setup();
        let r = run_apply(&[
            "-f",
            &s(&cfg),
            "--state-dir",
            &s(&sd),
            "--confirmation-message",
            "really?",
            "--pre-flight",
            "true",
            "--preview",
            "--yes",
        ]);
        assert!(r.is_ok());
    }

    #[test]
    fn pre_preflight_failure_err() {
        let (_d, cfg, sd) = setup();
        let r = run_apply(&[
            "-f",
            &s(&cfg),
            "--state-dir",
            &s(&sd),
            "--pre-flight",
            "false",
            "--preview",
        ]);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("Pre-flight"));
    }

    #[test]
    fn pre_script_failure_err() {
        let (d, cfg, sd) = setup();
        let script = d.path().join("pre.sh");
        std::fs::write(&script, "#!/bin/bash\nexit 1\n").unwrap();
        let r = run_apply(&[
            "-f",
            &s(&cfg),
            "--state-dir",
            &s(&sd),
            "--pre-script",
            &s(&script),
            "--preview",
        ]);
        assert!(r.is_err());
    }

    #[test]
    fn pre_webhook_before_best_effort_ok() {
        let (_d, cfg, sd) = setup();
        // Unreachable webhook is best-effort — apply continues into preview.
        let r = run_apply(&[
            "-f",
            &s(&cfg),
            "--state-dir",
            &s(&sd),
            "--webhook-before",
            "http://127.0.0.1:9/forjar",
            "--preview",
            "--yes",
        ]);
        assert!(r.is_ok());
    }

    #[test]
    fn pre_abort_on_drift_clean_state_ok() {
        let (_d, cfg, sd) = setup();
        let applied = run_apply(&["-f", &s(&cfg), "--state-dir", &s(&sd), "--yes"]);
        assert!(applied.is_ok());
        // No drift after converge → proceeds into preview mode.
        let r = run_apply(&[
            "-f",
            &s(&cfg),
            "--state-dir",
            &s(&sd),
            "--abort-on-drift",
            "--preview",
            "--yes",
        ]);
        assert!(r.is_ok(), "clean state should not abort: {r:?}");
    }

    // ── apply_backups + apply_execute ──

    #[test]
    fn backup_and_snapshot_before_apply_ok() {
        let (_d, cfg, sd) = setup();
        let r = run_apply(&[
            "-f",
            &s(&cfg),
            "--state-dir",
            &s(&sd),
            "--yes",
            "--backup",
            "--snapshot-before",
            "pre1",
        ]);
        assert!(r.is_ok(), "apply with backups should succeed: {r:?}");
    }

    #[test]
    fn dry_run_apply_ok() {
        let (_d, cfg, sd) = setup();
        let r = run_apply(&["-f", &s(&cfg), "--state-dir", &s(&sd), "--dry-run", "--yes"]);
        assert!(r.is_ok());
    }
}
