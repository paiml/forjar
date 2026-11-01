//! IMG-001 (forjar#756): the diff layer carries Added, Changed AND Deleted.

use super::container_diff::*;
use super::overlay_export::{merge_overlay_entries, scan_overlay_upper};
use crate::core::types::WhiteoutEntry;
use std::os::unix::fs::PermissionsExt;

const DIFF: &str = "C /etc\nA /etc/new.conf\nC /etc/os-release\nD /usr/share/doc\nD /etc/old.conf\nC /var\nD /srv/data\nA /srv/data/fresh\n";

#[test]
fn parse_diff_reads_all_three_kinds() {
    let e = parse_diff(DIFF);
    assert_eq!(e.len(), 8);
    assert_eq!(e[2], (DiffKind::Changed, "/etc/os-release".to_string()));
    assert_eq!(e[4], (DiffKind::Deleted, "/etc/old.conf".to_string()));
}

#[test]
fn plan_keeps_changed_and_whiteouts_deleted() {
    let plan = plan_diff(&parse_diff(DIFF));
    assert!(
        plan.copy.contains(&"/etc/os-release".to_string()),
        "C file lost"
    );
    assert!(plan.copy.contains(&"/etc/new.conf".to_string()));
    assert_eq!(plan.whiteouts, vec!["/usr/share/doc", "/etc/old.conf"]);
    // deleted then rebuilt: opaque, not a whiteout
    assert_eq!(plan.opaque_dirs, vec!["/srv/data"]);
}

/// A fake container runtime: `diff` prints `diff`, `exec ... stat -c "%F %a" p`
/// answers `directory <mode>` for `dirs` and `symbolic link 777` for `links`
/// (stat does not follow links), and `cp ctr:/p dest` makes a symlink to /etc for
/// `links` and a file holding "content of /p" otherwise.
fn fake_runtime_for(dir: &std::path::Path, diff: &str, dirs: &str, links: &str) -> String {
    let p = dir.join("fakedocker");
    std::fs::write(
        &p,
        format!(
            "#!/bin/bash\ncase \"$1\" in\n diff) printf '%s' '{diff}' ;;\n exec) case \" {dirs} \" in *\" $6 \"*) echo \"directory 750\"; exit 0;; esac\n  case \" {links} \" in *\" $6 \"*) echo \"symbolic link 777\"; exit 0;; esac\n  echo \"regular file 644\" ;;\n cp) src=\"${{2#*:}}\"; case \" {links} \" in *\" $src \"*) ln -s /etc \"$3\" ;; *) echo \"content of $src\" > \"$3\" ;; esac ;;\nesac\n"
        ),
    )
    .unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    p.to_string_lossy().to_string()
}

fn fake_runtime(dir: &std::path::Path) -> String {
    fake_runtime_for(dir, DIFF, "/etc /var", "")
}

#[test]
fn extracted_layer_holds_changed_file_and_whiteout_for_deleted() {
    let tmp = tempfile::tempdir().unwrap();
    let rt = fake_runtime(tmp.path());
    let out = tmp.path().join("extract");
    std::fs::create_dir_all(&out).unwrap();

    let n = extract_changes(&rt, "ctr", &out).unwrap();
    assert!(n >= 5, "count {n}");

    let scan = scan_overlay_upper(&out, &out).unwrap();
    let paths: Vec<String> = merge_overlay_entries(&scan)
        .iter()
        .map(|e| e.path.clone())
        .collect();
    assert!(
        paths.iter().any(|p| p.ends_with("etc/os-release")),
        "changed file: {paths:?}"
    );
    assert!(
        paths.iter().any(|p| p.ends_with("etc/new.conf")),
        "added file: {paths:?}"
    );
    assert!(
        paths.iter().any(|p| p.ends_with("etc/.wh.old.conf")),
        "whiteout: {paths:?}"
    );
    assert!(
        paths.iter().any(|p| p.ends_with("usr/share/.wh.doc")),
        "dir whiteout: {paths:?}"
    );
    assert!(
        scan.whiteouts.contains(&WhiteoutEntry::OpaqueDir {
            path: "srv/data".into()
        }),
        "opaque: {:?}",
        scan.whiteouts
    );
    // a changed directory is a marker, never copied whole
    assert!(!paths.iter().any(|p| p == "var"), "{paths:?}");
}

const PKG_DIFF: &str = "C /usr\nC /usr/bin\nA /usr/bin/cc\nA /usr/lib/gcc-data\nA /var/lib/pkg\nA /var/lib/pkg/emptydir\n";

#[test]
fn symlink_to_a_directory_and_empty_dirs_reach_the_layer() {
    let tmp = tempfile::tempdir().unwrap();
    // /usr/bin/cc and /usr/lib/gcc-data are symlinks (the latter to a DIRECTORY);
    // /var/lib/pkg/emptydir is an Added empty directory.
    let rt = fake_runtime_for(
        tmp.path(),
        PKG_DIFF,
        "/usr /usr/bin /var/lib/pkg /var/lib/pkg/emptydir",
        "/usr/bin/cc /usr/lib/gcc-data",
    );
    let out = tmp.path().join("extract");
    std::fs::create_dir_all(&out).unwrap();
    extract_changes(&rt, "ctr", &out).unwrap();

    let scan = scan_overlay_upper(&out, &out).unwrap();
    let all = merge_overlay_entries(&scan);
    let find = |p: &str| all.iter().find(|e| e.path == p);
    assert_eq!(
        find("usr/lib/gcc-data").and_then(|e| e.link_target.as_deref()),
        Some("/etc"),
        "symlink-to-dir dropped: {all:?}"
    );
    assert!(find("usr/bin/cc").is_some_and(|e| e.link_target.is_some()));
    let empty = find("var/lib/pkg/emptydir/").expect("empty dir lost");
    assert!(empty.is_dir);
    assert_eq!(empty.mode, 0o750, "dir mode lost");
}
