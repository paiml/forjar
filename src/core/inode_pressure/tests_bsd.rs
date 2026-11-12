//! forjar#995 falsifiers: BSD/macOS `df -i` has no type column and puts the
//! inode columns after the block columns; the probe picks the form by `uname`.

use super::*;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// `df -i` measured on mini (macOS 26.6.2), 2026-11-12, verbatim. Note the
/// filesystem name with a space (`map auto_home`) and its `-` %iused.
pub(crate) const MAC_DF_I: &str = "\
Filesystem     512-blocks      Used Available Capacity iused     ifree %iused  Mounted on
/dev/disk3s1s1  965595304  24686192 121663192    17%  458732 608315960    0%   /
devfs                 395       395         0   100%     684         0  100%   /dev
/dev/disk3s6    965595304        40 121663192     1%       0 608315960    0%   /System/Volumes/VM
/dev/disk3s5    965595304 798778552 121663192    87% 4704530 608315960    1%   /System/Volumes/Data
map auto_home           0         0         0   100%       0         0     -   /System/Volumes/Data/home
";

/// The same box with `/` out of inodes.
pub(crate) fn mac_df_root(pct: &str) -> String {
    format!(
        "Filesystem 512-blocks Used Available Capacity iused ifree %iused  Mounted on\n\
         /dev/disk3s1s1 965595304 24686192 121663192 17% 950 50 {pct} /\n"
    )
}

#[test]
fn bsd_rows_read_mount_and_percent_without_a_type_column() {
    let r = parse_df_i(MAC_DF_I);
    assert_eq!(r.len(), 5, "{r:?}");
    assert_eq!(r[0].mount, "/");
    assert_eq!(r[0].iuse_pct, Some(0), "%iused, not Capacity (17%)");
    assert_eq!(r[1].mount, "/dev");
    assert_eq!(r[1].iuse_pct, Some(100));
    assert_eq!(r[3].mount, "/System/Volumes/Data");
    assert_eq!(r[3].iuse_pct, Some(1), "%iused, not Capacity (87%)");
    assert!(r.iter().all(|x| x.fstype.is_empty()), "BSD df has no type");
}

#[test]
fn bsd_filesystem_name_with_a_space_and_a_dash() {
    let r = parse_df_i(MAC_DF_I);
    assert_eq!(r[4].mount, "/System/Volumes/Data/home");
    assert_eq!(r[4].iuse_pct, None, "a `-` %iused has no inode accounting");
}

#[test]
fn bsd_default_policy_judges_root_red_and_green() {
    let p = InodePressure::default();
    assert!(evaluate(&p, &parse_df_i(MAC_DF_I)).unwrap().is_empty());
    let red = evaluate(&p, &parse_df_i(&mac_df_root("95%"))).unwrap();
    assert_eq!(red.len(), 1, "{red:?}");
    assert_eq!((red[0].mount.as_str(), red[0].iuse_pct), ("/", 95));
}

#[test]
fn bsd_header_with_no_rows_is_not_measured() {
    let hdr = MAC_DF_I.lines().next().unwrap();
    assert!(evaluate(&InodePressure::default(), &parse_df_i(hdr)).is_err());
}

/// A fake `uname` and `df` that behave like the named platform: BSD `df`
/// refuses `-T` without an argument (exit 64) and drops inodes under `-P`.
fn fake_bin(dir: &Path, os: &str) {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let df = if os == "Darwin" {
        format!(
            "#!/bin/sh\nfor a in \"$@\"; do case \"$a\" in -T) echo 'df: option requires an argument -- T' >&2; exit 64;; \
             -P) echo 'Filesystem 512-blocks Used Available Capacity Mounted on'; exit 0;; esac; done\ncat <<'EOF'\n{MAC_DF_I}EOF\n"
        )
    } else {
        "#!/bin/sh\necho \"argv: $*\"\necho '/dev/sda1 ext4 10 1 9 10% /'\n".to_string()
    };
    for (name, body) in [("uname", format!("#!/bin/sh\necho {os}\n")), ("df", df)] {
        let p = bin.join(name);
        std::fs::write(&p, body).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

fn run_probe(os: &str) -> String {
    let d = tempfile::tempdir().unwrap();
    fake_bin(d.path(), os);
    let path = format!("{}:/usr/bin:/bin", d.path().join("bin").display());
    let out = std::process::Command::new("sh")
        .args(["-c", DF_INODES_CMD])
        .env("PATH", path)
        .output()
        .unwrap();
    assert!(out.status.success(), "{os}: {out:?}");
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn the_probe_on_darwin_runs_bsd_df_i_and_parses() {
    let text = run_probe("Darwin");
    let r = parse_df_i(&text);
    assert!(
        r.iter().any(|x| x.mount == "/" && x.iuse_pct == Some(0)),
        "{text}"
    );
}

#[test]
fn the_probe_on_linux_keeps_gnu_df_i_p_t() {
    assert!(run_probe("Linux").starts_with("argv: -i -P -T\n"));
}

#[test]
fn the_probe_passes_the_exec_validator() {
    crate::core::purifier::validate_script(DF_INODES_CMD).unwrap();
}
