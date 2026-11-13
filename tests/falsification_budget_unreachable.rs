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

fn shim(bin: &std::path::Path, name: &str, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    let p = bin.join(name);
    fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
}

/// A maps scan that produced nothing must not be "measured": FB_OPEN would be
/// empty, fb_in_use would answer "not in use" for everything, and the reaper
/// would delete. Fail closed.
#[test]
fn an_empty_maps_scan_fails_closed() {
    let (root, bin, target, res) = fixture("emptymaps", 40);
    shim(
        &bin,
        "cat",
        "case \"$1\" in /proc/*/maps) exit 0;; esac\nexec /usr/bin/cat \"$@\"",
    );
    let out = run_reaper(&res, &bin, &root).stdout;
    assert!(
        target.exists(),
        "empty maps scan must delete nothing\n{out}"
    );
    assert!(out.contains("maps scan produced nothing"), "{out}");
    fs::remove_dir_all(&root).ok();
}

/// An idle check that timed out is NOT MEASURED: the candidate must not be
/// silently dropped from the reach sum and the pass declared UNREACHABLE.
#[test]
fn a_timed_out_idle_check_is_not_unreachable() {
    let (root, bin, target, res) = fixture("idleto", 1_000_000_000);
    shim(
        &bin,
        "find",
        "case \"$*\" in *-mmin*) exit 124;; esac\nexec /usr/bin/find \"$@\"",
    );
    let out = run_reaper(&res, &bin, &root).stdout;
    assert!(
        !out.contains("UNREACHABLE"),
        "timeout decided unreachable\n{out}"
    );
    assert!(target.exists(), "{out}");
    fs::remove_dir_all(&root).ok();
}

/// Two rules over the same tree must count its bytes once. du is stubbed to
/// 1 MB per candidate; the need sits between 1x and 2x of that.
#[test]
fn overlapping_rules_count_bytes_once() {
    let root = tmpdir("overlap");
    let bin = root.join("bin");
    let target = root.join("src/wt/target");
    mk_cargo_target(&target);
    backdate(&root.join("src"));
    install_df_stub_total(&bin, &[(95, 1_000); 12], 9_766);
    shim(
        &bin,
        "du",
        "for a; do p=\"$a\"; done\nprintf '1000000\\t%s\\n' \"$p\"",
    );
    let rule = |n: &str| ReclaimRule {
        name: n.into(),
        roots: vec![root.join("src").to_string_lossy().into_owned()],
        kind: ReclaimKind::CargoTarget,
        min_idle_minutes: 60,
    };
    let res = budget_resource(&root, vec![rule("a"), rule("b")]);
    let out = run_reaper(&res, &bin, &root).stdout;
    assert!(
        out.contains("UNREACHABLE"),
        "double count hid the shortfall\n{out}"
    );
    assert!(target.exists(), "{out}");
    fs::remove_dir_all(&root).ok();
}
