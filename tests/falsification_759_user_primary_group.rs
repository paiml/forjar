//! forjar#759: a user that lists its PRIMARY group in `groups:` must converge.
//!
//! `group: course, groups: [course, courses]`: live `id -Gn` minus the primary
//! can never contain `course`, so a declared list that kept it read as drift
//! forever. These tests EXECUTE the generated check against a fake `id` and
//! `getent` fed from a fixture table, through forjar's public API.

use forjar::core::types::{MachineTarget, Resource, ResourceType};
use forjar::resources::user::check_script;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

fn write_exe(path: &Path, body: &str) {
    std::fs::write(path, body).expect("write fake");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
}

fn declared(groups: &[&str]) -> Resource {
    Resource {
        resource_type: ResourceType::User,
        machine: MachineTarget::Single("m1".to_string()),
        name: Some("course".to_string()),
        uid: Some(1100),
        shell: Some("/bin/bash".to_string()),
        home: Some("/srv/course".to_string()),
        group: Some("course".to_string()),
        groups: groups.iter().map(|s| s.to_string()).collect(),
        ..Default::default()
    }
}

fn run(r: &Resource) -> (i32, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let d = dir.path();
    std::fs::write(
        d.join("passwd"),
        "course:x:1100:1100::/srv/course:/bin/bash\n",
    )
    .unwrap();
    std::fs::write(d.join("groups"), "course courses,cop-inbox\n").unwrap();
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

#[test]
fn a_declared_primary_group_in_groups_still_converges() {
    let (rc, out) = run(&declared(&["course", "courses", "cop-inbox"]));
    assert_eq!(
        rc, 0,
        "primary group listed in groups: must not read as drift: {out}"
    );
}

#[test]
fn a_real_groups_drift_is_still_caught() {
    let (rc, out) = run(&declared(&["course", "courses", "docker"]));
    assert_ne!(
        rc, 0,
        "a missing/extra supplementary group must still diverge: {out}"
    );
}
