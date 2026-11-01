//! forjar#847 follow-up falsifiers: per-machine measurement through the
//! transport, `-` is a warning, a dry run survives an unmeasured host, and the
//! real `apply` runs the pre-check.

use super::*;
use crate::cli::apply::cmd_apply;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

const HDR: &str = "Filesystem Type Inodes IUsed IFree IUse% Mounted on\n";

fn df_text(root: &str) -> String {
    format!("{HDR}/dev/sda1 ext4 1000 100 900 {root} /\ntmpfs tmpfs 1000 10 990 1% /dev/shm\n")
}

fn t(name: &str, df: Result<String, String>) -> Target {
    (name.to_string(), df)
}

fn cfg() -> ForjarConfig {
    ForjarConfig::default()
}

#[test]
fn a_dash_is_a_warning_in_every_entry_point_never_red() {
    let btrfs = format!("{HDR}/dev/sda2 btrfs 0 0 0 - /\n");
    let targets = [t("here", Ok(btrfs))];
    let r = assess(&cfg(), &targets).unwrap();
    assert!(r.findings.is_empty(), "{r:?}");
    assert!(
        r.warnings.len() == 1
            && r.warnings[0].contains("not measured")
            && r.warnings[0].contains("here"),
        "{r:?}"
    );
    assert!(pre_check(&cfg(), &targets, "validate", false).is_ok());
    assert!(pre_check(&cfg(), &targets, "apply", false).is_ok());
    assert_eq!(drift_count(&cfg(), &targets, true), Ok(0));
}

#[test]
fn an_unmeasured_df_fails_apply_but_not_a_dry_run() {
    let targets = [t("web", Err("not measured: ssh refused".into()))];
    let e = pre_check(&cfg(), &targets, "apply", false).unwrap_err();
    assert!(e.contains("web") && e.contains("not measured"), "{e}");
    assert!(
        pre_check(&cfg(), &targets, "apply", true).is_ok(),
        "a dry run must survive a host it cannot measure"
    );
}

#[test]
fn each_machine_is_judged_and_named() {
    let targets = [t("web", Ok(df_text("99%"))), t("db", Ok(df_text("10%")))];
    let e = pre_check(&cfg(), &targets, "apply", false).unwrap_err();
    assert!(
        e.contains("web") && e.contains("99%") && !e.contains("db"),
        "{e}"
    );
    assert_eq!(drift_count(&cfg(), &targets, true), Ok(1));
    // The full machine is the SECOND one: a first-target-only reader misses it.
    let targets = [t("db", Ok(df_text("10%"))), t("web", Ok(df_text("99%")))];
    assert!(pre_check(&cfg(), &targets, "apply", false).is_err());
}

#[test]
fn one_unmeasured_machine_among_good_ones_fails_closed() {
    let targets = [
        t("db", Ok(df_text("10%"))),
        t("web", Err("not measured: down".into())),
    ];
    let e = pre_check(&cfg(), &targets, "apply", false).unwrap_err();
    assert!(e.contains("web"), "{e}");
}

fn fake_runtime(dir: &Path, root_pct: &str) -> PathBuf {
    let p = dir.join("fake-runtime");
    std::fs::write(
        &p,
        format!(
            "#!/bin/sh\ncat >/dev/null\nprintf '%s\\n' 'Filesystem Type Inodes IUsed IFree IUse% Mounted on' \
             'overlay overlay 1000 990 10 {root_pct} /'\n"
        ),
    )
    .unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    p
}

fn container_cfg(runtime: &Path) -> ForjarConfig {
    let yaml = format!(
        "version: \"1.0\"\nname: t\nmachines:\n  box:\n    hostname: box\n    addr: container\n    \
         transport: container\n    container:\n      runtime: {}\n      image: x\n      ephemeral: false\n\
         resources: {{}}\n",
        runtime.display()
    );
    crate::core::parser::parse_config(&yaml).unwrap()
}

#[test]
fn df_runs_through_the_machines_transport_not_on_the_controller() {
    let d = tempfile::tempdir().unwrap();
    let c = container_cfg(&fake_runtime(d.path(), "99%"));
    // The controller's own `/` is nowhere near 99%: only a measurement made
    // INSIDE the container can see this.
    let targets = collect(&c, None, false);
    assert_eq!(targets.len(), 1, "{targets:?}");
    assert_eq!(targets[0].0, "box");
    let e = pre_check(&c, &targets, "apply", false).unwrap_err();
    assert!(e.contains("box") && e.contains("99%"), "{e}");
    let ok = container_cfg(&fake_runtime(
        tempfile::tempdir().unwrap().keep().as_path(),
        "10%",
    ));
    assert!(pre_check(&ok, &collect(&ok, None, false), "apply", false).is_ok());
}

#[test]
fn a_machine_filter_limits_what_is_measured() {
    let d = tempfile::tempdir().unwrap();
    let c = container_cfg(&fake_runtime(d.path(), "99%"));
    assert!(collect(&c, Some("other"), false).is_empty());
}

#[test]
fn validate_measures_only_machines_the_controller_answers_for() {
    let d = tempfile::tempdir().unwrap();
    let c = container_cfg(&fake_runtime(d.path(), "99%"));
    assert!(
        collect(&c, None, true).is_empty(),
        "validate must not reach into a container"
    );
}

fn config(dir: &Path, threshold: u8) -> (PathBuf, PathBuf) {
    let target = dir.join("made-by-apply");
    let f = dir.join("forjar.yaml");
    std::fs::write(
        &f,
        format!(
            "version: \"1.0\"\nname: t\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\n\
             resources:\n  r:\n    type: file\n    machine: local\n    path: {}\n    content: hi\n\
             inode_pressure:\n  threshold_pct: 100\n  mounts:\n    - path: /\n      threshold_pct: {threshold}\n",
            target.display()
        ),
    )
    .unwrap();
    (f, target)
}

fn apply(file: &Path, state: &Path, dry_run: bool) -> Result<(), String> {
    cmd_apply(
        file,
        state,
        None,
        None,
        None,
        None,
        false,
        dry_run,
        false,
        &[],
        false,
        None,
        false,
        false,
        None,
        None,
        false,
        false,
        None,
        false,
        false,
        0,
        true,
        false,
        None,
        false,
        None,
        None,
        None,
        false,
        None,
        false,
        None,
        false,
        None,
        &[],
    )
}

#[test]
fn the_real_apply_runs_the_inode_pre_check() {
    let d = tempfile::tempdir().unwrap();
    let state = d.path().join("state");
    let (red, target) = config(d.path(), 1);
    let e = apply(&red, &state, false).unwrap_err();
    assert!(e.contains("inode"), "{e}");
    assert!(
        !target.exists(),
        "apply wrote to a host that failed the inode check"
    );
    // The same config at 100% proves the refusal was the inode check.
    let (green, target) = config(d.path(), 100);
    apply(&green, &state, false).unwrap();
    assert!(target.exists());
}

#[test]
fn the_real_dry_run_warns_instead_of_refusing() {
    let d = tempfile::tempdir().unwrap();
    let (red, target) = config(d.path(), 1);
    apply(&red, &d.path().join("state"), true).unwrap();
    assert!(!target.exists());
}
