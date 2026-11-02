//! forjar#847 falsifiers for the pure inode-pressure logic.

use super::*;

const HDR: &str = "Filesystem Type Inodes IUsed IFree IUse% Mounted on\n";

fn df(rows: &[&str]) -> String {
    format!("{HDR}{}\n", rows.join("\n"))
}

fn fixture(root: &str, tmp: &str) -> String {
    df(&[
        &format!("/dev/nvme0n1p2 ext4 1000 900 100 {root} /"),
        &format!("tmpfs tmpfs 1000 900 100 {tmp} /tmp"),
        "/dev/sda1 ext4 1000 999 1 99% /data",
        "efivarfs efivarfs - - - - /sys/firmware/efi/efivars",
    ])
}

fn findings(cfg: &InodePressure, out: &str) -> Result<Vec<InodeFinding>, String> {
    evaluate(cfg, &parse_df_i(out))
}

#[test]
fn parse_reads_mount_type_and_percent() {
    let r = parse_df_i(&fixture("10%", "99%"));
    assert_eq!(r.len(), 4);
    assert_eq!(r[1].mount, "/tmp");
    assert_eq!(r[1].fstype, "tmpfs");
    assert_eq!(r[1].iuse_pct, Some(99));
    assert_eq!(r[3].iuse_pct, None, "a `-` row has no inode accounting");
}

#[test]
fn parse_keeps_a_mount_point_with_spaces() {
    let r = parse_df_i(&df(&["/dev/sdb1 ext4 10 1 9 10% /mnt/my disk"]));
    assert_eq!(r[0].mount, "/mnt/my disk");
}

#[test]
fn drift_flags_a_mount_at_99_and_passes_the_same_mount_at_10() {
    let cfg = InodePressure::default();
    let bad = findings(&cfg, &fixture("10%", "99%")).unwrap();
    assert_eq!(bad.len(), 1, "{bad:?}");
    assert_eq!(bad[0].mount, "/tmp");
    assert_eq!(bad[0].iuse_pct, 99);
    let good = findings(&cfg, &fixture("10%", "10%")).unwrap();
    assert!(good.is_empty(), "{good:?}");
}

#[test]
fn default_red_line_is_90_inclusive() {
    let cfg = InodePressure::default();
    assert_eq!(cfg.threshold_pct, 90);
    assert_eq!(findings(&cfg, &fixture("90%", "10%")).unwrap().len(), 1);
    assert!(findings(&cfg, &fixture("89%", "10%")).unwrap().is_empty());
}

#[test]
fn default_watch_is_root_and_tmpfs_only() {
    // /data is at 99% on ext4 and is NOT watched by default.
    let f = findings(&InodePressure::default(), &fixture("95%", "95%")).unwrap();
    let mounts: Vec<&str> = f.iter().map(|x| x.mount.as_str()).collect();
    assert_eq!(mounts, ["/", "/tmp"]);
}

#[test]
fn per_mount_override_replaces_the_default() {
    let cfg = InodePressure {
        threshold_pct: 90,
        mounts: vec![
            WatchedMount {
                path: "/tmp".into(),
                threshold_pct: Some(50),
            },
            WatchedMount {
                path: "/data".into(),
                threshold_pct: None,
            },
        ],
    };
    let f = findings(&cfg, &fixture("10%", "60%")).unwrap();
    assert_eq!(f.len(), 2, "{f:?}");
    assert_eq!((f[0].mount.as_str(), f[0].threshold_pct), ("/tmp", 50));
    assert_eq!((f[1].mount.as_str(), f[1].threshold_pct), ("/data", 90));
}

#[test]
fn a_watched_mount_that_df_does_not_list_is_not_measured() {
    let cfg = InodePressure {
        threshold_pct: 90,
        mounts: vec![WatchedMount {
            path: "/nope".into(),
            threshold_pct: None,
        }],
    };
    let e = findings(&cfg, &fixture("10%", "10%")).unwrap_err();
    assert!(e.contains("/nope") && e.contains("not measured"), "{e}");
}

#[test]
fn no_root_row_in_default_mode_is_not_measured() {
    let e = findings(
        &InodePressure::default(),
        &df(&["tmpfs tmpfs 10 1 9 10% /tmp"]),
    )
    .unwrap_err();
    assert!(e.contains("not measured"), "{e}");
}

#[test]
fn finding_names_the_mount_and_the_percentage() {
    let f = findings(&InodePressure::default(), &fixture("10%", "99%")).unwrap();
    let s = f[0].to_string();
    assert!(
        s.contains("/tmp") && s.contains("99%") && s.contains("90%"),
        "{s}"
    );
}

#[test]
fn config_thresholds_must_be_1_to_100_and_mounts_absolute() {
    assert!(validate_config(&InodePressure::default()).is_empty());
    for bad in [0u8, 101] {
        let cfg = InodePressure {
            threshold_pct: bad,
            mounts: vec![],
        };
        assert_eq!(validate_config(&cfg).len(), 1, "threshold {bad}");
    }
    let cfg = InodePressure {
        threshold_pct: 90,
        mounts: vec![
            WatchedMount {
                path: "tmp".into(),
                threshold_pct: None,
            },
            WatchedMount {
                path: "/ok".into(),
                threshold_pct: Some(0),
            },
        ],
    };
    assert_eq!(validate_config(&cfg).len(), 2);
}

fn btrfs_root() -> String {
    df(&[
        "/dev/sda2 btrfs 0 0 0 - /",
        "tmpfs tmpfs 1000 100 900 10% /tmp",
    ])
}

#[test]
fn a_dash_on_root_is_not_red_and_is_reported_as_unmeasured() {
    let readings = parse_df_i(&btrfs_root());
    assert_eq!(
        evaluate(&InodePressure::default(), &readings),
        Ok(vec![]),
        "a '-' IUse% must never be a finding"
    );
    assert_eq!(
        unmeasured_mounts(&InodePressure::default(), &readings),
        vec!["/".to_string()]
    );
}

#[test]
fn a_dash_on_an_explicitly_watched_mount_is_unmeasured_not_an_error() {
    let cfg = InodePressure {
        threshold_pct: 90,
        mounts: vec![WatchedMount {
            path: "/".into(),
            threshold_pct: None,
        }],
    };
    let readings = parse_df_i(&btrfs_root());
    assert_eq!(evaluate(&cfg, &readings), Ok(vec![]));
    assert_eq!(unmeasured_mounts(&cfg, &readings), vec!["/".to_string()]);
}

#[test]
fn unwatched_dash_rows_are_not_reported() {
    // efivarfs prints '-' in the fixture and is neither / nor tmpfs.
    assert!(unmeasured_mounts(
        &InodePressure::default(),
        &parse_df_i(&fixture("10%", "10%"))
    )
    .is_empty());
}
