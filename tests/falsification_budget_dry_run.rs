//! forjar#334: a disk-budget preview must preview.
//!
//! The reported incident: `FORJAR_BUDGET_DRY_RUN=1 forjar apply -r raid-disk-budget`
//! deleted ~1.5 TB and printed `1 converged` — the same line a real reclaim
//! prints. The variable is a variable of the GENERATED REAPER, read on the
//! target, and neither `sudo bash <<'FORJAR_SUDO'` (env_reset) nor `ssh host
//! bash` (no SendEnv) carries it there, so the reaper evaluated
//! `${FORJAR_BUDGET_DRY_RUN:-0}` against an empty environment and fell through
//! to its fail-dangerous default of deleting.
//!
//! These tests run the emitted reaper against a real tree with the SCRUBBED
//! environment that hop actually delivers, and assert on what is left on disk.

#[path = "common/budget_harness.rs"]
mod harness;

use forjar::core::types::{ReclaimKind, ReclaimRule};
use harness::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

struct Ran {
    code: i32,
    stdout: String,
}

/// Run the emitted reaper with EXACTLY the environment given — no inherited
/// `FORJAR_BUDGET_*` of any kind beyond what is passed here.
fn run_scrubbed(script_body: &str, bin: &Path, work: &Path, env: &[(&str, &str)]) -> Ran {
    let script = work.join("reaper.sh");
    fs::write(&script, script_body).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    let mut cmd = Command::new("/bin/sh");
    cmd.arg(&script)
        .env_clear()
        .env(
            "PATH",
            format!(
                "{}:{}",
                bin.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .env("FORJAR_BUDGET_STATUS", work.join("status.json"));
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("run reaper");
    Ran {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
    }
}

/// A tree under pressure with one reclaimable, idle cargo target dir.
fn under_pressure(tag: &str) -> (PathBuf, PathBuf, PathBuf, forjar::core::types::Resource) {
    let root = tmpdir(tag);
    let bin = root.join("bin");
    let target = root.join("src/wt/target");
    mk_cargo_target(&target);
    backdate(&root.join("src"));
    // Stays at 95% for every `df`, so the budget is never met: the reaper keeps
    // going through every candidate and, on a real pass, exits 1 at the end.
    install_df_stub(&bin, &[(95, 1_000); 12]);
    let res = budget_resource(
        &root,
        vec![ReclaimRule {
            name: "targets".into(),
            roots: vec![root.join("src").to_string_lossy().into_owned()],
            kind: ReclaimKind::CargoTarget,
            min_idle_minutes: 60,
        }],
    );
    (root, bin, target, res)
}

/// A one-resource config holding a `disk_budget`, for the codegen surfaces.
fn write_budget_config(root: &Path) -> PathBuf {
    let cfg = root.join("forjar.yaml");
    let r = root.display();
    let body = format!(
        "version: \"1.0\"\n\
         name: budget-preview\n\
         machines:\n\
         \x20 sandbox:\n\
         \x20   hostname: sandbox\n\
         \x20   addr: 127.0.0.1\n\
         resources:\n\
         \x20 budget:\n\
         \x20   type: disk_budget\n\
         \x20   machine: sandbox\n\
         \x20   path: {r}\n\
         \x20   budget_high_watermark_pct: 85\n\
         \x20   budget_target_free_pct: 20\n\
         \x20   budget_reclaim:\n\
         \x20     - name: targets\n\
         \x20       kind: cargo_target\n\
         \x20       roots: [\"{r}/src\"]\n"
    );
    fs::write(&cfg, body).unwrap();
    cfg
}

/// THE falsification. This is the environment `sudo`/`ssh` actually deliver:
/// nothing. The reaper must inspect, say so, and succeed.
#[test]
fn a_reaper_invoked_with_no_environment_deletes_nothing() {
    let (root, bin, target, res) = under_pressure("no-env");
    let body = reaper_body(&res);

    let r = run_scrubbed(&body, &bin, &root, &[]);

    assert!(
        target.exists(),
        "a reaper reached with a scrubbed environment must not delete\n{}",
        r.stdout
    );
    assert!(
        r.stdout.contains("mode=dry-run"),
        "the pass must name its mode\n{}",
        r.stdout
    );
    assert_eq!(
        r.code, 0,
        "a preview that reclaimed nothing is correct, not inert\n{}",
        r.stdout
    );
    fs::remove_dir_all(&root).ok();
}

/// A preview must not inflate `reclaimed_bytes`, and must not stamp the
/// drift-hashed heartbeat with a health record for deletions that never
/// happened.
#[test]
fn a_preview_reports_no_reclaimed_bytes_and_leaves_the_heartbeat_alone() {
    let (root, bin, target, res) = under_pressure("preview-status");
    let status = root.join("status.json");
    let before = "{\"health\":\"effective\",\"reclaimed_bytes\":42}";
    fs::write(&status, before).unwrap();

    let r = run_scrubbed(&reaper_body(&res), &bin, &root, &[]);

    assert!(target.exists(), "preview deleted\n{}", r.stdout);
    assert_eq!(
        fs::read_to_string(&status).unwrap(),
        before,
        "a preview must not rewrite the heartbeat\n{}",
        r.stdout
    );
    assert!(
        r.stdout.contains("reclaimed 0 bytes"),
        "a preview freed nothing, so the reclaim total must be 0\n{}",
        r.stdout
    );
    assert!(
        r.stdout.contains("would_reclaim") && !r.stdout.contains("would_reclaim 0 bytes"),
        "a preview must report what it WOULD have freed\n{}",
        r.stdout
    );
    fs::remove_dir_all(&root).ok();
}

/// The guard against "fixed" by making the reaper permanently inert: the
/// scheduled unit and `forjar apply` still reclaim.
#[test]
fn the_scheduled_and_apply_time_passes_still_reclaim() {
    let (root, bin, target, res) = under_pressure("execute");

    let r = run_scrubbed(
        &reaper_body(&res),
        &bin,
        &root,
        &[("FORJAR_BUDGET_EXECUTE", "1")],
    );

    assert!(
        !target.exists(),
        "the granted pass must still reclaim\n{}",
        r.stdout
    );
    assert!(
        r.stdout.contains("mode=execute"),
        "an executing pass must say so\n{}",
        r.stdout
    );
    fs::remove_dir_all(&root).ok();
}

/// The documented variable must stop being a lie: when it DOES reach the
/// reaper it wins, even over an explicit grant.
#[test]
fn the_documented_dry_run_variable_beats_the_execute_grant() {
    let (root, bin, target, res) = under_pressure("dry-beats-execute");

    let r = run_scrubbed(
        &reaper_body(&res),
        &bin,
        &root,
        &[
            ("FORJAR_BUDGET_EXECUTE", "1"),
            ("FORJAR_BUDGET_DRY_RUN", "1"),
        ],
    );

    assert!(
        target.exists(),
        "an explicit dry-run request must win\n{}",
        r.stdout
    );
    assert!(r.stdout.contains("mode=dry-run"), "{}", r.stdout);
    fs::remove_dir_all(&root).ok();
}

/// The opt-in is granted in exactly two places, both of them forjar's own
/// generated text: the systemd unit and the apply-time pass.
#[test]
fn the_unit_and_the_apply_script_are_the_only_grants() {
    let root = tmpdir("grants");
    let res = budget_resource(&root, vec![]);
    let apply = forjar::resources::disk_budget::apply_script(&res);

    assert!(
        apply.contains("FORJAR_BUDGET_EXECUTE=1 '/usr/local/sbin/forjar-disk-budget-"),
        "the apply-time pass must grant the opt-in explicitly:\n{apply}"
    );
    assert!(
        apply.contains("EXECUTE mode (this deletes)"),
        "the apply-time pass must declare its mode:\n{apply}"
    );
    assert!(
        apply.contains("Environment=FORJAR_BUDGET_EXECUTE=1"),
        "the installed service unit must grant the opt-in:\n{apply}"
    );
    // The reaper itself carries no grant — it is the thing being granted to.
    let reaper = reaper_body(&res);
    assert!(
        !reaper.contains("FORJAR_BUDGET_EXECUTE=1\n"),
        "the reaper must not grant itself the opt-in:\n{reaper}"
    );
    fs::remove_dir_all(&root).ok();
}

/// `canonical_generated_script` hashes `apply_script` into `hash_desired_state`,
/// so an env-dependent apply script would make a machine's desired state depend
/// on whoever happened to run forjar. Pins that against a future "clever" fix
/// that reads the operator's environment at codegen time.
///
/// Runs the real binary twice rather than calling `apply_script` in-process:
/// `std::env::set_var` is a disallowed method in this workspace, and a child
/// process is the honest shape of the question anyway.
#[test]
fn the_apply_script_does_not_vary_with_the_operators_environment() {
    let root = tmpdir("purity");
    let cfg = write_budget_config(&root);

    let emit = |env: &[(&str, &str)]| -> String {
        let mut c = Command::new(env!("CARGO_BIN_EXE_forjar"));
        c.args(["codegen", "-r", "budget", "--phase", "apply", "-f"])
            .arg(&cfg)
            .current_dir(&root);
        for (k, v) in env {
            c.env(k, v);
        }
        let out = c.output().expect("run forjar codegen");
        assert!(
            out.status.success(),
            "codegen failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    };

    let clean = emit(&[]);
    let dirty = emit(&[("FORJAR_BUDGET_DRY_RUN", "1")]);

    assert!(!clean.is_empty(), "codegen emitted nothing");
    assert_eq!(
        clean, dirty,
        "apply_script is hashed into hash_desired_state and must stay pure"
    );
    fs::remove_dir_all(&root).ok();
}

/// The supported preview: `--phase reaper` emits the pass alone, which deletes
/// nothing when run. `--phase apply` emits the installer, which grants the
/// opt-in — piping THAT to `sh` is what deleted 1.5 TB.
#[test]
fn the_reaper_phase_emits_a_preview_and_the_apply_phase_emits_the_installer() {
    let root = tmpdir("phases");
    let cfg = write_budget_config(&root);

    let emit = |phase: &str| -> String {
        let out = Command::new(env!("CARGO_BIN_EXE_forjar"))
            .args(["codegen", "-r", "budget", "--phase", phase, "-f"])
            .arg(&cfg)
            .current_dir(&root)
            .output()
            .expect("run forjar codegen");
        assert!(
            out.status.success(),
            "codegen --phase {phase} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    };

    let reaper = emit("reaper");
    assert!(
        reaper.starts_with("#!/bin/sh"),
        "the reaper phase must emit a runnable pass:\n{reaper}"
    );
    assert!(
        !reaper.contains("FORJAR_BUDGET_EXECUTE=1 '"),
        "the preview must not grant itself the opt-in:\n{reaper}"
    );
    assert!(
        emit("apply").contains("FORJAR_BUDGET_EXECUTE=1 '"),
        "the apply phase is the INSTALLER and does grant it"
    );
    fs::remove_dir_all(&root).ok();
}
