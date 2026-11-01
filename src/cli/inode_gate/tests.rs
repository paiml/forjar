//! forjar#847 falsifiers: one per entry point (drift, validate, apply), plus
//! the real commands wired to the real `df` on a mount that is always >= 1%.

use super::*;
use std::path::Path;

const HDR: &str = "Filesystem Type Inodes IUsed IFree IUse% Mounted on\n";

fn df(root: &str, tmp: &str) -> Result<String, String> {
    Ok(format!(
        "{HDR}/dev/nvme0n1p2 ext4 1000 100 900 {root} /\ntmpfs tmpfs 1000 990 10 {tmp} /dev/shm\n"
    ))
}

fn cfg() -> ForjarConfig {
    ForjarConfig::default()
}

#[test]
fn drift_counts_a_99_percent_tmpfs_and_not_the_same_mount_at_10() {
    assert_eq!(drift_count(&cfg(), df("10%", "99%"), true).unwrap(), 1);
    assert_eq!(drift_count(&cfg(), df("10%", "10%"), true).unwrap(), 0);
}

#[test]
fn drift_that_cannot_measure_fails_closed() {
    let e = drift_count(&cfg(), Err("not measured: no df".into()), true).unwrap_err();
    assert!(e.contains("not measured"), "{e}");
}

#[test]
fn validate_refuses_a_full_tmpfs_and_accepts_it_at_10() {
    let e = pre_check(&cfg(), df("10%", "99%"), "validate", false).unwrap_err();
    assert!(
        e.contains("/dev/shm") && e.contains("99%") && e.contains("validate"),
        "{e}"
    );
    assert!(pre_check(&cfg(), df("10%", "10%"), "validate", false).is_ok());
}

#[test]
fn apply_precheck_refuses_a_full_root_and_accepts_it_at_89() {
    assert!(pre_check(&cfg(), df("90%", "10%"), "apply", false).is_err());
    assert!(pre_check(&cfg(), df("89%", "10%"), "apply", false).is_ok());
}

#[test]
fn apply_dry_run_warns_and_does_not_refuse() {
    assert!(pre_check(&cfg(), df("99%", "10%"), "apply", true).is_ok());
}

#[test]
fn a_config_policy_overrides_the_default() {
    let mut c = cfg();
    c.inode_pressure = Some(crate::core::inode_pressure::InodePressure {
        threshold_pct: 50,
        mounts: vec![],
    });
    assert_eq!(
        judge(&c, df("60%", "10%").unwrap().as_str()).unwrap().len(),
        1
    );
}

fn write(dir: &Path, threshold: u8) -> std::path::PathBuf {
    let f = dir.join("forjar.yaml");
    std::fs::write(
        &f,
        format!(
            "version: \"1.0\"\nname: t\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\n\
             resources: {{}}\ninode_pressure:\n  threshold_pct: 100\n  mounts:\n    - path: /\n      threshold_pct: {threshold}\n"
        ),
    )
    .unwrap();
    f
}

#[test]
fn real_validate_is_red_on_the_real_root_at_1_percent_and_green_at_100() {
    let d = tempfile::tempdir().unwrap();
    let e = super::super::validate_core::cmd_validate(&write(d.path(), 1), false, false, false)
        .unwrap_err();
    assert!(e.contains("inode"), "{e}");
    super::super::validate_core::cmd_validate(&write(d.path(), 100), false, false, false).unwrap();
}

#[test]
fn real_drift_counts_the_real_root_at_1_percent() {
    let d = tempfile::tempdir().unwrap();
    let state = d.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    let run = |t| {
        super::super::drift::cmd_drift(
            &write(d.path(), t),
            &state,
            None,
            false,
            None,
            false,
            false,
            true,
            false,
            None,
            false,
            false,
        )
    };
    assert_eq!(run(1).unwrap_err(), "1 drift finding(s)");
    assert_ne!(run(100).unwrap_err_or_default(), "1 drift finding(s)");
}

trait ErrOrDefault {
    fn unwrap_err_or_default(self) -> String;
}
impl ErrOrDefault for Result<(), String> {
    fn unwrap_err_or_default(self) -> String {
        self.err().unwrap_or_default()
    }
}

#[test]
fn an_unusable_policy_is_refused_at_validate_time() {
    let yaml = "version: \"1.0\"\nname: t\nmachines: {}\nresources: {}\ninode_pressure:\n  threshold_pct: 0\n";
    let config = crate::core::parser::parse_config(yaml).unwrap();
    let errs = crate::core::parser::validate_config(&config);
    assert!(
        errs.iter().any(|e| e.message.contains("threshold_pct")),
        "{errs:?}"
    );
}
