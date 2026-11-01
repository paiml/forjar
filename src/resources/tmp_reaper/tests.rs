//! forjar#847 falsifiers for `tmp_reaper`: the generated shell is RUN, never
//! just read.

use super::*;
use crate::core::types::{MachineTarget, ResourceType};
use std::path::Path;
use std::process::Command;

fn res(path: &str, age: &str, pattern: Option<&str>) -> Resource {
    Resource {
        resource_type: ResourceType::TmpReaper,
        machine: MachineTarget::Single("m".into()),
        path: Some(path.into()),
        max_age: Some(age.into()),
        pattern: pattern.map(Into::into),
        ..Default::default()
    }
}

fn sh(script: &str) -> (i32, String) {
    let o = Command::new("bash").arg("-c").arg(script).output().unwrap();
    let mut s = String::from_utf8_lossy(&o.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&o.stderr));
    (o.status.code().unwrap_or(-1), s)
}

fn age(p: &Path, days: u32) {
    let st = Command::new("touch")
        .args(["-h", "-d", &format!("{days} days ago")])
        .arg(p)
        .status()
        .unwrap();
    assert!(st.success());
}

fn file(dir: &Path, name: &str, days: u32) -> std::path::PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, "x").unwrap();
    age(&p, days);
    p
}

#[test]
fn removes_old_files_and_keeps_new_ones() {
    let d = tempfile::tempdir().unwrap();
    let old = file(d.path(), "old.tmp", 10);
    let new = file(d.path(), "new.tmp", 0);
    let (rc, out) = sh(&apply_script(&res(d.path().to_str().unwrap(), "7d", None)));
    assert_eq!(rc, 0, "{out}");
    assert!(!old.exists() && new.exists(), "{out}");
}

#[test]
fn a_second_apply_removes_nothing_and_says_so() {
    let d = tempfile::tempdir().unwrap();
    file(d.path(), "a", 10);
    file(d.path(), "b", 10);
    let r = res(d.path().to_str().unwrap(), "7d", None);
    let (_, first) = sh(&apply_script(&r));
    assert!(first.contains("removed 2 file(s), 2 inode(s)"), "{first}");
    let (rc, second) = sh(&apply_script(&r));
    assert_eq!(rc, 0);
    assert!(second.contains("removed 0 file(s), 0 inode(s)"), "{second}");
}

#[test]
fn pattern_limits_what_is_removed() {
    let d = tempfile::tempdir().unwrap();
    let tmp = file(d.path(), "x.tmp", 10);
    let keep = file(d.path(), "x.log", 10);
    sh(&apply_script(&res(
        d.path().to_str().unwrap(),
        "7d",
        Some("*.tmp"),
    )));
    assert!(!tmp.exists() && keep.exists());
}

#[test]
fn an_old_symlink_is_unlinked_and_its_target_outside_survives() {
    let d = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let victim = file(outside.path(), "victim", 30);
    let reap = d.path().join("reap");
    std::fs::create_dir(&reap).unwrap();
    let link = reap.join("link");
    std::os::unix::fs::symlink(outside.path(), &link).unwrap();
    age(&link, 10);
    let (rc, out) = sh(&apply_script(&res(reap.to_str().unwrap(), "7d", None)));
    assert_eq!(rc, 0, "{out}");
    assert!(victim.exists(), "followed a symlink out of the path: {out}");
    assert!(!link.exists() && !link.is_symlink(), "{out}");
}

#[test]
fn a_symlinked_reap_root_is_refused_at_run_time() {
    let d = tempfile::tempdir().unwrap();
    let real = d.path().join("real");
    std::fs::create_dir(&real).unwrap();
    let victim = file(&real, "v", 30);
    let link = d.path().join("root");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let (rc, out) = sh(&apply_script(&res(link.to_str().unwrap(), "7d", None)));
    assert_ne!(rc, 0, "{out}");
    assert!(victim.exists(), "{out}");
}

#[test]
fn old_empty_directories_go_and_non_empty_ones_stay() {
    let d = tempfile::tempdir().unwrap();
    let empty = d.path().join("empty");
    let full = d.path().join("full");
    std::fs::create_dir(&empty).unwrap();
    std::fs::create_dir(&full).unwrap();
    file(&full, "fresh", 0);
    age(&empty, 10);
    age(&full, 10);
    sh(&apply_script(&res(d.path().to_str().unwrap(), "7d", None)));
    assert!(!empty.exists() && full.exists());
}

#[test]
fn check_fails_while_stale_files_exist_and_passes_after_apply() {
    let d = tempfile::tempdir().unwrap();
    file(d.path(), "old", 10);
    let r = res(d.path().to_str().unwrap(), "7d", None);
    assert_ne!(sh(&check_script(&r)).0, 0);
    sh(&apply_script(&r));
    assert_eq!(sh(&check_script(&r)).0, 0);
}

#[test]
fn the_path_is_shell_quoted() {
    let d = tempfile::tempdir().unwrap();
    let odd = d.path().join("it's here");
    std::fs::create_dir(&odd).unwrap();
    let old = file(&odd, "old", 10);
    let (rc, out) = sh(&apply_script(&res(odd.to_str().unwrap(), "7d", None)));
    assert_eq!(rc, 0, "{out}");
    assert!(!old.exists());
}

#[test]
fn refuses_root_empty_relative_and_dotdot_paths() {
    for bad in [
        "/",
        "",
        "//",
        "///",
        "relative/dir",
        "/tmp/../etc",
        "/tmp/a/..",
    ] {
        let e = spec_of(&res(bad, "7d", None)).unwrap_err();
        assert!(e.contains("path"), "{bad:?}: {e}");
    }
    let mut r = res("/tmp", "7d", None);
    r.path = None;
    assert!(spec_of(&r).is_err());
    assert!(spec_of(&res("/tmp/work/", "7d", None)).is_ok());
}

#[test]
fn refuses_a_bad_max_age_and_a_pattern_with_a_slash() {
    for bad in ["", "0d", "7", "d", "7y", "-1d", "1.5d", "7d7"] {
        assert!(spec_of(&res("/tmp/x", bad, None)).is_err(), "{bad:?}");
    }
    assert!(spec_of(&res("/tmp/x", "7d", Some("../*"))).is_err());
    assert!(spec_of(&res("/tmp/x", "7d", Some(""))).is_err());
    let mut r = res("/tmp/x", "7d", None);
    r.max_age = None;
    assert!(spec_of(&r).is_err());
}

#[test]
fn ages_parse_to_minutes() {
    assert_eq!(parse_age("30m"), Ok(30));
    assert_eq!(parse_age("12h"), Ok(720));
    assert_eq!(parse_age("7d"), Ok(10_080));
    assert_eq!(parse_age("2w"), Ok(20_160));
}

fn validate_yaml(path: &str) -> Vec<String> {
    let yaml = format!(
        "version: \"1.0\"\nname: t\nmachines:\n  m: {{hostname: m, addr: 127.0.0.1}}\nresources:\n  r:\n    type: tmp_reaper\n    machine: m\n    path: \"{path}\"\n    max_age: 7d\n"
    );
    let cfg = crate::core::parser::parse_config(&yaml).unwrap();
    crate::core::parser::validate_config(&cfg)
        .into_iter()
        .map(|e| e.message)
        .collect()
}

#[test]
fn path_slash_is_refused_at_validate_time_and_a_real_path_is_not() {
    let errs = validate_yaml("/");
    assert!(
        errs.iter()
            .any(|m| m.contains("tmp_reaper") && m.contains("/")),
        "{errs:?}"
    );
    assert!(validate_yaml("/var/tmp/work").is_empty());
}
