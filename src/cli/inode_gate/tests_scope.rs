//! forjar#847 fix round 3: a container that does not exist yet is deferred,
//! one unmeasured host does not abort drift, and validate names what it skips.

use super::*;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// A runtime whose container is `state` ("true" running, "false" stopped,
/// "missing" absent). Every `exec` is logged to `<dir>/exec.log`.
fn runtime(dir: &Path, state: &str, root_pct: &str) -> PathBuf {
    let p = dir.join("rt");
    let log = dir.join("exec.log");
    let inspect = if state == "missing" {
        "exit 1".to_string()
    } else {
        format!("echo {state}; exit 0")
    };
    std::fs::write(
        &p,
        format!(
            "#!/bin/sh\ncase \"$1\" in inspect) {inspect};; esac\n\
             echo \"$1\" >> '{}'\ncat >/dev/null\nprintf '%s\\n' 'Filesystem Type Inodes IUsed IFree IUse% Mounted on' \
             'overlay overlay 1000 990 10 {root_pct} /'\n",
            log.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    p
}

fn cfg(rt: &Path) -> ForjarConfig {
    let y = format!(
        "version: \"1.0\"\nname: t\nmachines:\n  box:\n    hostname: box\n    addr: container\n    \
         transport: container\n    container:\n      runtime: {}\n      image: x\n      ephemeral: true\n\
         resources: {{}}\n",
        rt.display()
    );
    crate::core::parser::parse_config(&y).unwrap()
}

#[test]
fn a_container_that_does_not_exist_yet_is_deferred_not_refused() {
    for state in ["missing", "false"] {
        let d = tempfile::tempdir().unwrap();
        let c = cfg(&runtime(d.path(), state, "99%"));
        let targets = collect(&c, None, false);
        let report = assess(&c, &targets).unwrap_or_else(|e| panic!("{state}: refused: {e}"));
        assert!(report.findings.is_empty(), "{state}");
        assert!(
            report
                .warnings
                .iter()
                .any(|w| w.contains("box") && w.contains("deferred")),
            "{state}: {:?}",
            report.warnings
        );
        assert!(pre_check(&c, &targets, "apply", false).is_ok(), "{state}");
        let log = std::fs::read_to_string(d.path().join("exec.log")).unwrap_or_default();
        assert!(
            !log.contains("exec"),
            "{state}: df was run in a container that is not there"
        );
    }
}

#[test]
fn a_running_container_that_cannot_answer_still_fails_closed() {
    let d = tempfile::tempdir().unwrap();
    let rt = runtime(d.path(), "true", "99%");
    // Running, but `exec` prints nothing: that IS an unmeasured host.
    std::fs::write(
        &rt,
        "#!/bin/sh\ncase \"$1\" in inspect) echo true; exit 0;; esac\nexit 1\n",
    )
    .unwrap();
    let c = cfg(&rt);
    let e = pre_check(&c, &collect(&c, None, false), "apply", false).unwrap_err();
    assert!(e.contains("not measured") && e.contains("box"), "{e}");
}

#[test]
fn drift_reports_an_unmeasured_host_and_still_judges_the_others() {
    let c = ForjarConfig::default();
    let good = "Filesystem Type Inodes IUsed IFree IUse% Mounted on\nx ext4 1000 990 10 99% /\n";
    let targets: Vec<Target> = vec![
        ("down".into(), Err("not measured: ssh refused".into())),
        ("full".into(), Ok(good.into())),
    ];
    let (findings, unmeasured) =
        drift_count(&c, &targets, false).expect("one unmeasured host must not abort drift");
    assert_eq!((findings, unmeasured), (1, 1));
}

#[test]
fn validate_names_every_machine_it_cannot_measure() {
    let d = tempfile::tempdir().unwrap();
    let mut c = cfg(&runtime(d.path(), "true", "10%"));
    let ssh = crate::core::parser::parse_config(
        "version: \"1.0\"\nname: t\nmachines:\n  far:\n    hostname: far\n    addr: 192.0.2.7\nresources: {}\n",
    )
    .unwrap();
    c.machines.extend(ssh.machines);
    let w = skipped_by_validate(&c);
    assert_eq!(w.len(), 2, "{w:?}");
    assert!(
        w.iter()
            .any(|m| m.contains("box") && m.contains("not measured")),
        "{w:?}"
    );
    assert!(
        w.iter()
            .any(|m| m.contains("far") && m.contains("not measured")),
        "{w:?}"
    );
}
