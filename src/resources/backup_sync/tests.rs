//! FJ-037: backup_sync handler tests.

use super::*;
use crate::core::types::{BackupSpec, MachineTarget, ResourceType};
use std::collections::HashMap;

fn res() -> Resource {
    let mut rc = HashMap::new();
    rc.insert("scope".to_string(), "drive.file".to_string());
    Resource {
        resource_type: ResourceType::BackupSync,
        machine: MachineTarget::Single("lambda-labs".into()),
        home: Some("/home/noah".into()),
        backup: BackupSpec {
            remote: Some("gdrive:lambda-labs-media".into()),
            remote_config: rc,
            token: Some(r#"{"access_token":"x","refresh_token":"y"}"#.into()),
            source: vec![
                "/mnt/nvme-raid0/RecordedCourses".into(),
                "/mnt/nvme-raid0/home-Videos".into(),
            ],
            ..Default::default()
        },
        ..Default::default()
    }
}

#[test]
fn defaults_are_documented_values() {
    let c = backup_of(&res()).unwrap();
    assert_eq!(c.schedule, "daily");
    assert_eq!(c.verify_pct, 99);
    assert_eq!(c.daily_cap_gb, 700);
}

#[test]
fn a_local_destination_is_rejected_at_codegen() {
    // The predecessor's exact shape: DEST=/videos.
    let r = Resource {
        backup: BackupSpec {
            remote: Some("/videos".into()),
            ..res().backup
        },
        ..res()
    };
    assert!(backup_of(&r).is_err());
    let s = apply_script(&r);
    assert!(s.contains("ERROR"));
    assert!(s.contains("LOCAL path"));
}

#[test]
fn missing_remote_is_rejected() {
    let r = Resource {
        backup: BackupSpec {
            remote: None,
            ..res().backup
        },
        ..res()
    };
    assert!(backup_of(&r).is_err());
    assert!(apply_script(&r).contains("ERROR"));
}

#[test]
fn an_empty_source_list_is_rejected() {
    let r = Resource {
        backup: BackupSpec {
            source: vec![],
            ..res().backup
        },
        ..res()
    };
    assert!(backup_of(&r).is_err());
}

#[test]
fn apply_does_not_run_the_sync() {
    // §0.2: if the deployer runs the job, it writes the status file that is
    // supposed to be evidence the SERVICE ran. Arm the timer, nothing more.
    let s = apply_script(&res());
    assert!(s.contains("systemctl restart forjar-backup-gdrive-lambda-labs-media.timer"));
    assert!(
        !s.contains("\n/usr/local/sbin/forjar-backup-gdrive-lambda-labs-media.sh\n"),
        "apply must not invoke the sync script"
    );
    assert!(s.contains("first pass runs on the timer"));
}

#[test]
fn state_query_takes_execution_evidence_from_the_journal() {
    // A /run/*.json can be written by the deployer; a journal entry cannot.
    let s = state_query_script(&res());
    assert!(s.contains("journalctl -u forjar-backup-gdrive-lambda-labs-media.service"));
    assert!(s.contains("backup_ever_ran="));
}

#[test]
fn state_query_publishes_classes_not_raw_counters() {
    let s = state_query_script(&res());
    for k in [
        "backup_health=",
        "backup_heartbeat=",
        "backup_timer=",
        "backup_ever_ran=",
    ] {
        assert!(s.contains(k), "missing drift-visible class {k}");
    }
    let raw = s.find("backup_coverage_pct=").expect("raw line");
    assert!(
        s[raw..].contains(">&2"),
        "coverage counters must go to stderr, not into the drift hash"
    );
}

#[test]
fn removal_stops_before_it_deletes() {
    // PMAT-219: deleting a unit file out from under a loaded unit leaves it
    // Active: failed with "Unit to trigger vanished".
    let r = Resource {
        state: Some("absent".into()),
        ..res()
    };
    let s = apply_script(&r);
    let stop = s.find("systemctl stop").expect("stop");
    let disable = s.find("systemctl disable").expect("disable");
    let rm = s.find("rm -f").expect("rm");
    let reload = s.find("daemon-reload").expect("reload");
    assert!(stop < disable && disable < rm && rm < reload, "order: {s}");
    assert!(s.contains("reset-failed"));
}

#[test]
fn an_unresolved_secret_template_is_refused() {
    let r = Resource {
        backup: BackupSpec {
            token: Some("{{secrets.rclone-gdrive-token}}".into()),
            ..res().backup
        },
        ..res()
    };
    let s = apply_script(&r);
    assert!(s.contains("ERROR"));
    assert!(s.contains("unresolved template"));
    // ...and must never write that literal into a config.
    assert!(!s.contains("token = {{secrets"));
}

#[test]
fn apply_installs_the_rclone_config_forjar_manages() {
    let s = apply_script(&res());
    assert!(s.contains("/home/noah/.config/rclone/rclone.conf"));
    assert!(s.contains("[gdrive]"));
    assert!(s.contains("type = drive"));
    assert!(s.contains("scope = drive.file"));
    assert!(s.contains("umask 077"));
}

#[test]
fn artifact_paths_are_namespaced_per_remote() {
    assert_ne!(script_path("gdrive:a"), script_path("gdrive:b"));
    assert_ne!(service_path("gdrive:a"), service_path("gdrive:b"));
    assert_ne!(status_json("gdrive:a"), status_json("gdrive:b"));
}

#[test]
fn slug_is_filesystem_safe() {
    assert_eq!(slug("gdrive:lambda-labs-media"), "gdrive-lambda-labs-media");
    assert_eq!(slug(":::"), "backup");
}

#[test]
fn heartbeat_window_scales_with_cadence() {
    assert_eq!(stale_secs("daily"), 86400 * 3);
    assert_eq!(stale_secs("hourly"), 3600 * 3);
    assert_eq!(stale_secs("weekly"), 604_800 * 3);
    // Unknown expressions fall back to daily, never to zero.
    assert_eq!(stale_secs("Mon *-*-* 04:00:00"), 86400 * 3);
}

#[test]
fn check_distinguishes_absent_from_unverified() {
    let s = check_script(&res());
    assert!(s.contains("'absent'"));
    assert!(s.contains("'present'"));
    assert!(s.contains("'unverified'"));
}

#[test]
fn every_declared_source_reaches_the_sync() {
    let s = apply_script(&res());
    assert!(s.contains("/mnt/nvme-raid0/RecordedCourses"));
    assert!(s.contains("/mnt/nvme-raid0/home-Videos"));
    assert!(s.contains("'gdrive:lambda-labs-media/RecordedCourses'"));
    assert!(s.contains("'gdrive:lambda-labs-media/home-Videos'"));
}

/// Every emitted script must survive forjar's own I8 purification gate — the
/// test whose absence let six violations ship in `disk_budget`.
#[test]
fn every_emitted_script_passes_the_purifier() {
    use crate::core::purifier::validate_script;
    let r = res();
    for (what, script) in [
        ("apply", apply_script(&r)),
        ("check", check_script(&r)),
        ("state_query", state_query_script(&r)),
    ] {
        assert!(
            validate_script(&script).is_ok(),
            "{what}_script fails I8 purification, so `forjar apply` rejects this \
             resource on every machine:\n{:?}",
            validate_script(&script).unwrap_err()
        );
    }
    let absent = Resource {
        state: Some("absent".into()),
        ..r
    };
    assert!(validate_script(&apply_script(&absent)).is_ok());
}
