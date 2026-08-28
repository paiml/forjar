//! Tests: Coverage for cmd_refresh_only and cmd_apply_from_plan in
//! apply_variants.rs — main paths and error branches (PMAT-088).

use super::apply_from_plan::cmd_apply_from_plan;
use super::apply_variants::*;
use super::helpers::parse_and_validate;
use super::helpers_state::load_machine_locks;
use super::plan_file::save_plan_file;
use super::workspace::inject_workspace_param;
use crate::core::{executor, planner, resolver, types};
use std::path::{Path, PathBuf};

/// Write a localhost config with one file resource targeting `target`.
fn write_cfg(dir: &Path, target: &Path, content: &str) -> PathBuf {
    let yaml = format!(
        "version: \"1.0\"\nname: variants\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: {}\n    content: {}\n",
        target.display(),
        content
    );
    let p = dir.join("forjar.yaml");
    std::fs::write(&p, yaml).unwrap();
    p
}

/// Converge a config against a state dir via the core executor.
fn apply_local(config: &types::ForjarConfig, sd: &Path) -> Vec<types::ApplyResult> {
    let cfg = executor::ApplyConfig {
        config,
        state_dir: sd,
        force: false,
        dry_run: false,
        machine_filter: None,
        resource_filter: None,
        tag_filter: None,
        group_filter: None,
        timeout_secs: None,
        force_unlock: false,
        progress: false,
        retry: 0,
        parallel: None,
        resource_timeout: None,
        rollback_on_failure: false,
        max_parallel: None,
        trace: false,
        run_id: None,
        refresh: false,
        force_tag: None,
    };
    executor::apply(&cfg).expect("apply")
}

fn converged_setup() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("out.txt");
    let cfg = write_cfg(dir.path(), &target, "hello");
    let sd = dir.path().join("state");
    std::fs::create_dir_all(&sd).unwrap();
    let config = parse_and_validate(&cfg).unwrap();
    let results = apply_local(&config, &sd);
    assert_eq!(results[0].resources_converged, 1);
    (dir, cfg, sd, target)
}

/// Mirror cmd_apply_from_plan's config mutation, then save a plan file.
fn save_plan_for(cfg: &Path, sd: &Path, ws: Option<&str>, out: &Path) {
    let mut config = parse_and_validate(cfg).unwrap();
    inject_workspace_param(&mut config, ws);
    resolver::resolve_data_sources(&mut config).unwrap();
    let order = resolver::build_execution_order(&config).unwrap();
    let locks = load_machine_locks(&config, sd, None).unwrap();
    let plan = planner::plan(&config, &order, &locks, None);
    save_plan_file(&plan, &config, cfg, sd, out).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── cmd_refresh_only ──

    #[test]
    fn refresh_invalid_config_err() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("bad.yaml");
        std::fs::write(&p, "not: valid: yaml: [[[").unwrap();
        let r = cmd_refresh_only(&p, dir.path(), None, false, None, None, None);
        assert!(r.is_err());
    }

    #[test]
    fn refresh_empty_state_ok() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("o.txt");
        let cfg = write_cfg(dir.path(), &target, "x");
        let sd = dir.path().join("state");
        std::fs::create_dir_all(&sd).unwrap();
        let r = cmd_refresh_only(&cfg, &sd, None, false, None, None, None);
        assert!(r.is_ok());
    }

    #[test]
    fn refresh_after_converge_ok() {
        let (_d, cfg, sd, _t) = converged_setup();
        let r = cmd_refresh_only(&cfg, &sd, None, false, None, None, None);
        assert!(r.is_ok(), "refresh on converged state: {r:?}");
        // Second refresh: live_hash now stored, stable hash → no drift path.
        let r2 = cmd_refresh_only(&cfg, &sd, None, true, None, None, None);
        assert!(r2.is_ok());
    }

    #[test]
    fn refresh_detects_drift_verbose() {
        let (_d, cfg, sd, target) = converged_setup();
        // Seed live_hash, then mutate the target file out-of-band.
        cmd_refresh_only(&cfg, &sd, None, false, None, None, None).unwrap();
        std::fs::write(&target, "tampered").unwrap();
        let r = cmd_refresh_only(&cfg, &sd, None, true, None, None, None);
        assert!(r.is_ok(), "drift is reported, not an error: {r:?}");
    }

    #[test]
    fn refresh_machine_filter_no_match_ok() {
        let (_d, cfg, sd, _t) = converged_setup();
        let r = cmd_refresh_only(&cfg, &sd, Some("ghost"), false, None, None, None);
        assert!(r.is_ok());
    }

    #[test]
    fn refresh_with_workspace_param_ok() {
        let (_d, cfg, sd, _t) = converged_setup();
        let r = cmd_refresh_only(&cfg, &sd, Some("m"), true, Some(30), None, Some("ws1"));
        assert!(r.is_ok());
    }

    #[test]
    fn refresh_missing_env_file_err() {
        let (_d, cfg, sd, _t) = converged_setup();
        let r = cmd_refresh_only(
            &cfg,
            &sd,
            None,
            false,
            None,
            Some(Path::new("/nonexistent/params.env")),
            None,
        );
        assert!(r.is_err());
    }

    #[test]
    fn refresh_skips_non_converged_resources() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = write_cfg(dir.path(), Path::new("/dev/null/forjar-cov/x"), "y");
        let sd = dir.path().join("state");
        std::fs::create_dir_all(&sd).unwrap();
        let config = parse_and_validate(&cfg).unwrap();
        let _ = apply_local(&config, &sd); // resource fails → status Failed in lock
        let r = cmd_refresh_only(&cfg, &sd, None, true, None, None, None);
        assert!(r.is_ok(), "failed resources are skipped: {r:?}");
    }

    // ── cmd_apply_from_plan ──

    #[test]
    fn from_plan_happy_path_ok() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("planned.txt");
        let cfg = write_cfg(dir.path(), &target, "v1");
        let sd = dir.path().join("state");
        std::fs::create_dir_all(&sd).unwrap();
        let plan_path = dir.path().join("plan.json");
        save_plan_for(&cfg, &sd, Some("pfws"), &plan_path);

        let r = cmd_apply_from_plan(&cfg, &sd, &plan_path, true, None, Some("pfws"), None);
        assert!(r.is_ok(), "saved plan should apply cleanly: {r:?}");
        assert!(target.exists(), "planned file resource should be created");
    }

    #[test]
    fn from_plan_zero_changes_ok() {
        let (d, cfg, sd, _t) = converged_setup();
        // Plan built AFTER converge has no pending changes.
        let plan_path = d.path().join("plan.json");
        save_plan_for(&cfg, &sd, Some("pfws"), &plan_path);
        let r = cmd_apply_from_plan(&cfg, &sd, &plan_path, false, None, Some("pfws"), None);
        assert!(r.is_ok(), "no-change plan exits early: {r:?}");
    }

    #[test]
    fn from_plan_missing_file_err() {
        let (d, cfg, sd, _t) = converged_setup();
        let r = cmd_apply_from_plan(
            &cfg,
            &sd,
            &d.path().join("nope.json"),
            false,
            None,
            Some("pfws"),
            None,
        );
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("read plan file"));
    }

    #[test]
    fn from_plan_bad_format_err() {
        let (d, cfg, sd, _t) = converged_setup();
        let plan_path = d.path().join("plan.json");
        std::fs::write(&plan_path, r#"{"format": "bogus-v9"}"#).unwrap();
        let r = cmd_apply_from_plan(&cfg, &sd, &plan_path, false, None, Some("pfws"), None);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("unsupported plan format"));
    }

    #[test]
    fn from_plan_config_changed_err() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("c.txt");
        let cfg = write_cfg(dir.path(), &target, "v1");
        let sd = dir.path().join("state");
        std::fs::create_dir_all(&sd).unwrap();
        let plan_path = dir.path().join("plan.json");
        save_plan_for(&cfg, &sd, Some("pfws"), &plan_path);
        // Mutate the config after the plan was saved → hash mismatch.
        write_cfg(dir.path(), &target, "v2-changed");
        let r = cmd_apply_from_plan(&cfg, &sd, &plan_path, false, None, Some("pfws"), None);
        let err = r.unwrap_err();
        assert!(err.starts_with("PLAN_HASH_MISMATCH:"), "{err}");
        assert!(err.contains("config leg"), "{err}");
    }

    #[test]
    fn from_plan_failing_resource_err() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = write_cfg(dir.path(), Path::new("/dev/null/forjar-cov/p.txt"), "y");
        let sd = dir.path().join("state");
        std::fs::create_dir_all(&sd).unwrap();
        let plan_path = dir.path().join("plan.json");
        save_plan_for(&cfg, &sd, Some("pfws"), &plan_path);
        let r = cmd_apply_from_plan(&cfg, &sd, &plan_path, false, None, Some("pfws"), None);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("failed"));
    }
}
