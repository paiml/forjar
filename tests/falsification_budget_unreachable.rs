//! forjar#669: a target the rules cannot reach must not turn the reaper into a
//! TTL sweep. When freeing EVERY eligible candidate still leaves the filesystem
//! above target, deleting the idle trees fixes nothing and destroys warm
//! caches; the pass must log the shortfall, delete nothing, and still fail.

#[path = "common/budget_harness.rs"]
mod harness;

use forjar::core::types::{ReclaimKind, ReclaimRule};
use harness::*;
use std::fs;

fn fixture(
    tag: &str,
    total_kb: u64,
) -> (
    std::path::PathBuf,
    std::path::PathBuf,
    std::path::PathBuf,
    forjar::core::types::Resource,
) {
    let root = tmpdir(tag);
    let bin = root.join("bin");
    let target = root.join("src/wt/target");
    mk_cargo_target(&target);
    backdate(&root.join("src"));
    install_df_stub_total(&bin, &[(95, 1_000); 12], total_kb);
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

/// THE falsification: 1 TB filesystem at 95%, target 80%, one 4 KiB idle tree.
/// Freeing all of it cannot reach the target, so nothing may be deleted.
#[test]
fn an_unreachable_target_deletes_nothing_and_logs_the_shortfall() {
    let (root, bin, target, res) = fixture("unreach", 1_000_000_000);
    let r = run_reaper(&res, &bin, &root);
    let (code, out) = (r.code, r.stdout);
    assert!(
        target.exists(),
        "unreachable target must not sweep the tree\n{out}"
    );
    assert!(
        out.contains("UNREACHABLE"),
        "must name the shortfall\n{out}"
    );
    assert!(
        out.contains("shortfall"),
        "must report the shortfall\n{out}"
    );
    assert_eq!(
        code, 1,
        "a triggered pass that missed target still fails\n{out}"
    );
    fs::remove_dir_all(&root).ok();
}

/// Control: when the candidates CAN reach the target the reaper still deletes.
#[test]
fn a_reachable_target_still_reclaims() {
    let (root, bin, target, res) = fixture("reach", 40);
    let out = run_reaper(&res, &bin, &root).stdout;
    assert!(
        !target.exists(),
        "reachable target must still reclaim\n{out}"
    );
    assert!(!out.contains("UNREACHABLE"), "{out}");
    fs::remove_dir_all(&root).ok();
}

/// forjar#936: an empty /proc scan means "nobody provably idle", never "nobody
/// is using it". A reachable target must survive when `find /proc` sees nothing.
#[test]
fn an_empty_proc_scan_fails_closed() {
    let (root, bin, target, res) = fixture("emptyscan", 40);
    let shim = "#!/bin/sh\ncase \"$*\" in */proc/*) exit 0;; esac\nexec /usr/bin/find \"$@\"\n";
    let p = bin.join("find");
    fs::write(&p, shim).unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let out = run_reaper(&res, &bin, &root).stdout;
    assert!(
        target.exists(),
        "empty /proc scan must fail closed, nothing deleted\n{out}"
    );
    assert!(
        out.contains("NOT MEASURED"),
        "must say it did not measure\n{out}"
    );
    fs::remove_dir_all(&root).ok();
}
