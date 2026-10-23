//! The user check asked only `id <name>`, i.e. PRESENCE.
//!
//! A declared `uid`, `shell`, `home`, primary `group` or supplementary
//! `groups` was applied by `useradd`/`usermod` and then never asked about
//! again: a user moved into another group by hand still checked
//! `exists:<name>`, exit 0, "converged". For an isolation user (one that must
//! NOT be in the owner's group), presence-only is the whole defect. `state:
//! absent` was also judged backwards: `id` succeeding meant converged.
//!
//! These tests RUN the generated check against a fake `id` and `getent` fed
//! from a fixture passwd/group table, and assert the exit code and markers.

use super::user::*;
use crate::core::types::{MachineTarget, Resource, ResourceType};
use std::path::Path;

fn user(name: &str) -> Resource {
    Resource {
        resource_type: ResourceType::User,
        machine: MachineTarget::Single("m1".to_string()),
        name: Some(name.to_string()),
        ..Default::default()
    }
}

fn write_exe(path: &Path, body: &str) {
    std::fs::write(path, body).expect("write fake");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    }
}

/// Fixture box: `course` uid 1100, primary `course`, supplementary
/// `courses,cop-inbox`, shell /bin/bash, home /srv/course.
/// `extra` is appended to course's supplementary groups (the drift).
fn run(r: &Resource, extra: &str) -> (i32, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let d = dir.path();
    let groups = format!("courses,cop-inbox{extra}");
    std::fs::write(
        d.join("passwd"),
        "course:x:1100:1100::/srv/course:/bin/bash\n",
    )
    .unwrap();
    std::fs::write(d.join("groups"), format!("course {groups}\n")).unwrap();
    // id [-u|-g|-gn|-Gn] NAME ; bare `id -u` (the SUDO preamble) answers 1000.
    write_exe(
        &d.join("id"),
        &format!(
            r#"#!/bin/bash
db={d}
if [ $# -eq 1 ] && [ "$1" = -u ]; then echo 1000; exit 0; fi
flag=""; [ $# -eq 2 ] && {{ flag=$1; shift; }}
line=$(grep "^$1:" "$db/passwd") || exit 1
case "$flag" in
  -u) echo "$line" | cut -d: -f3 ;;
  -g) echo "$line" | cut -d: -f4 ;;
  -gn) awk -v u="$1" '$1==u{{print $1}}' "$db/groups" ;;
  -Gn) awk '{{g=$2; gsub(",", " ", g); print $1, g}}' "$db/groups" ;;
  *) echo "uid=x" ;;
esac
"#,
            d = d.display()
        ),
    );
    write_exe(
        &d.join("getent"),
        &format!(
            "#!/bin/bash\n[ \"$1\" = passwd ] || exit 2\ngrep \"^$2:\" {}/passwd\n",
            d.display()
        ),
    );
    let out = std::process::Command::new("bash")
        .arg("-c")
        .arg(check_script(r))
        .env("PATH", format!("{}:/usr/bin:/bin", d.display()))
        .output()
        .expect("bash");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

fn declared() -> Resource {
    let mut r = user("course");
    r.uid = Some(1100);
    r.shell = Some("/bin/bash".to_string());
    r.home = Some("/srv/course".to_string());
    r.groups = vec!["cop-inbox".to_string(), "courses".to_string()];
    r
}

#[test]
fn matching_user_converges() {
    let (rc, out) = run(&declared(), "");
    assert_eq!(
        rc, 0,
        "a user matching every declared field converges: {out}"
    );
}

#[test]
fn an_extra_supplementary_group_diverges() {
    // the isolation case: course hand-added to the owner's group
    let (rc, out) = run(&declared(), ",noah");
    assert_ne!(rc, 0, "course in group noah must NOT read converged: {out}");
    assert!(out.contains("groups"), "the marker names the field: {out}");
}

#[test]
fn a_missing_supplementary_group_diverges() {
    let mut r = declared();
    r.groups.push("video".to_string());
    let (rc, out) = run(&r, "");
    assert_ne!(rc, 0, "declared group video absent on the box: {out}");
}

#[test]
fn wrong_uid_shell_home_each_diverge() {
    for (field, set) in [
        (
            "uid",
            (|r: &mut Resource| r.uid = Some(1200)) as fn(&mut Resource),
        ),
        ("shell", |r: &mut Resource| {
            r.shell = Some("/bin/zsh".to_string())
        }),
        ("home", |r: &mut Resource| {
            r.home = Some("/home/course".to_string())
        }),
    ] {
        let mut r = declared();
        set(&mut r);
        let (rc, out) = run(&r, "");
        assert_ne!(rc, 0, "declared {field} differs from the box: {out}");
        assert!(out.contains(field), "marker names {field}: {out}");
    }
}

#[test]
fn wrong_primary_group_diverges() {
    let mut r = declared();
    r.group = Some("staff".to_string());
    let (rc, out) = run(&r, "");
    assert_ne!(rc, 0, "{out}");
}

#[test]
fn undeclared_fields_are_not_asserted() {
    // only presence declared: an existing user converges whatever its groups
    let (rc, out) = run(&user("course"), ",noah");
    assert_eq!(rc, 0, "{out}");
}

#[test]
fn missing_user_diverges() {
    let (rc, _) = run(&user("ghost"), "");
    assert_ne!(rc, 0);
}

#[test]
fn absent_is_judged_the_right_way_round() {
    let mut gone = user("ghost");
    gone.state = Some("absent".to_string());
    assert_eq!(run(&gone, "").0, 0, "absent + no such user = converged");
    let mut present = user("course");
    present.state = Some("absent".to_string());
    assert_ne!(run(&present, "").0, 0, "absent + user exists = diverged");
}
