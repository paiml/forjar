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

/// A fake container runtime: `diff` prints DIFF, `exec ... test -d` succeeds
/// for directories, `cp ctr:/p dest` writes "content of /p".
fn fake_runtime(dir: &std::path::Path) -> String {
    let p = dir.join("fakedocker");
    std::fs::write(
        &p,
        format!(
            "#!/bin/bash\ncase \"$1\" in\n diff) printf '%s' '{}' ;;\n exec) case \"$5\" in /etc|/var) exit 0;; *) exit 1;; esac ;;\n cp) src=\"${{2#*:}}\"; echo \"content of $src\" > \"$3\" ;;\nesac\n",
            DIFF
        ),
    )
    .unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    p.to_string_lossy().to_string()
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
