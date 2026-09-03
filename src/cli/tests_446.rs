//! #446: unit tests for the pure halves of `exec`, `facts` and `doctor --machine`.

use super::doctor_machine::{
    dir_check, disk_check, inode_check, parent_dir, path_check, permission_detail, summary_line,
    verdict, Check, DirStat, Status,
};
use super::exec::{exec_json, permission_hint, shell_join, shell_quote};
use super::facts::{human_kb, parse_disk, parse_facts, parse_tool, render, Disk, Facts};
use crate::transport::ExecOutput;

fn disk(mount: &str, avail_kb: u64, use_pct: u32, inode_use_pct: u32) -> Disk {
    Disk {
        mount: mount.to_string(),
        avail_kb,
        use_pct,
        inode_use_pct,
    }
}

fn facts_with(user: &str, uid: u64, sudo: bool) -> Facts {
    let mut f = parse_facts("");
    f.user = user.to_string();
    f.uid = Some(uid);
    f.sudo = sudo;
    f
}

#[test]
fn shell_quote_keeps_bare_words_and_wraps_the_rest() {
    assert_eq!(shell_quote("plain-word_1.txt"), "plain-word_1.txt");
    assert_eq!(shell_quote("it's $HOME"), "'it'\\''s $HOME'");
    assert_eq!(shell_quote(""), "''");
}

#[test]
fn shell_join_preserves_argv_boundaries() {
    let words = vec!["sh".to_string(), "-c".to_string(), "echo a b".to_string()];
    assert_eq!(shell_join(&words), "sh -c 'echo a b'");
}

#[test]
fn permission_hint_only_on_permission_denied() {
    assert!(
        permission_hint("m1", "curl: (23) Failed writing body: Permission denied")
            .is_some_and(|h| h.contains("doctor --machine m1"))
    );
    assert!(permission_hint("m1", "not found").is_none());
}

#[test]
fn exec_json_carries_the_three_fields() {
    let out = ExecOutput {
        exit_code: 3,
        stdout: "x".into(),
        stderr: "y".into(),
    };
    let v: serde_json::Value = serde_json::from_str(&exec_json("m1", &out)).unwrap();
    assert_eq!(v["machine"], "m1");
    assert_eq!(v["exit_code"], 3);
    assert_eq!(v["stdout"], "x");
    assert_eq!(v["stderr"], "y");
}

#[test]
fn parse_facts_reads_every_kind_of_line_and_tolerates_junk() {
    let text = "hostname=box\nkernel=Linux 6.8\nuid=1000\nsudo=yes\npath=/usr/bin:/bin\n\
                disk=/:1048576:42:7\ndisk=/data:20:99:1\ntool=curl:/usr/bin/curl\ntool=dnf:missing\n\
                garbage line\nuptime_s=notanumber\n";
    let f = parse_facts(text);
    assert_eq!(f.hostname, "box");
    assert_eq!(f.uid, Some(1000));
    assert!(f.sudo);
    assert_eq!(f.path, "/usr/bin:/bin");
    assert_eq!(f.disks.len(), 2);
    assert_eq!(f.disks[1].use_pct, 99);
    assert_eq!(
        f.tools.get("curl").cloned().flatten().as_deref(),
        Some("/usr/bin/curl")
    );
    assert_eq!(f.tools.get("dnf").cloned().flatten(), None);
    assert_eq!(f.uptime_s, 0, "an unparsable number stays at the default");
}

#[test]
fn parse_disk_and_parse_tool_reject_short_lines() {
    assert!(parse_disk("/:100").is_none());
    let d = parse_disk("/home:512:50%:3%").unwrap();
    assert_eq!(
        (d.mount.as_str(), d.avail_kb, d.use_pct, d.inode_use_pct),
        ("/home", 512, 50, 3)
    );
    assert!(parse_tool("curl").is_none());
    assert_eq!(
        parse_tool("git:missing").unwrap(),
        ("git".to_string(), None)
    );
}

#[test]
fn human_kb_scales() {
    assert_eq!(human_kb(2048), "2 MiB");
    assert_eq!(human_kb(3 * 1024 * 1024), "3.0 GiB");
}

#[test]
fn render_names_the_machine_and_its_disks() {
    let mut f = parse_facts("hostname=box\nuser=ci\n");
    f.disks.push(disk("/", 1, 1, 1));
    f.tools.insert("curl".into(), None);
    let text = render("m1", &f);
    assert!(
        text.contains("m1") && text.contains("box") && text.contains("/") && text.contains("curl")
    );
}

#[test]
fn path_check_warns_when_the_usual_dirs_are_missing() {
    assert_eq!(
        path_check("/usr/local/bin:/usr/sbin:/usr/bin:/bin").status,
        Status::Pass
    );
    let c = path_check("/usr/bin:/bin");
    assert_eq!(c.status, Status::Warn);
    assert!(c.detail.contains("/usr/local/bin") && c.detail.contains("/usr/sbin"));
}

#[test]
fn disk_check_thresholds_at_the_boundaries() {
    assert_eq!(
        disk_check(&[disk("/", 50 * 1024 * 1024, 50, 1)]).status,
        Status::Pass
    );
    assert_eq!(
        disk_check(&[disk("/", 50 * 1024 * 1024, 91, 1)]).status,
        Status::Warn
    );
    assert_eq!(
        disk_check(&[disk("/", 512, 10, 1)]).status,
        Status::Warn,
        "under 1 GiB warns"
    );
    assert_eq!(disk_check(&[disk("/", 1, 99, 1)]).status, Status::Fail);
    assert_eq!(
        disk_check(&[]).status,
        Status::Warn,
        "no filesystem reported is a finding"
    );
}

#[test]
fn inode_check_warns_below_five_percent_free() {
    assert_eq!(inode_check(&[disk("/", 1, 1, 90)]).status, Status::Pass);
    assert_eq!(inode_check(&[disk("/", 1, 1, 96)]).status, Status::Warn);
}

#[test]
fn parent_dir_of_a_path() {
    assert_eq!(parent_dir("/etc/app/app.conf"), "/etc/app");
    assert_eq!(parent_dir("/etc"), "/");
    assert_eq!(parent_dir("relative.txt"), ".");
}

#[test]
fn dir_check_names_owner_mode_and_identity_when_unwritable() {
    let stat = DirStat {
        path: "/opt/app".into(),
        exists: true,
        owner: "root".into(),
        group: "root".into(),
        mode: "755".into(),
        writable: false,
    };
    let f = facts_with("ci", 1000, false);
    let detail = permission_detail(&stat, &f);
    for needle in ["/opt/app", "root:root", "755", "ci", "1000", "sudo: no"] {
        assert!(detail.contains(needle), "{detail} lacks {needle}");
    }
    assert_eq!(dir_check(&stat, &f).status, Status::Fail);
    let ok = DirStat {
        writable: true,
        ..stat.clone()
    };
    assert_eq!(dir_check(&ok, &f).status, Status::Pass);
    let missing = DirStat {
        exists: false,
        ..stat
    };
    assert_ne!(dir_check(&missing, &f).status, Status::Pass);
}

#[test]
fn summary_and_verdict_follow_the_worst_check() {
    let checks = vec![
        Check::new("a", Status::Pass, ""),
        Check::new("b", Status::Warn, ""),
        Check::new("c", Status::Fail, "bad"),
    ];
    assert_eq!(summary_line(&checks), "3 checks: 1 pass, 1 warn, 1 fail");
    assert!(verdict("m1", &checks).is_err());
    assert!(verdict("m1", &checks[..2]).is_ok());
}

#[test]
fn a_uid_that_does_not_parse_is_unknown_not_root() {
    let f = parse_facts("user=ci\nuid=abc\nnproc=many\n");
    assert_eq!(f.uid, None);
    assert_eq!(f.nproc, 0, "an unparsable count keeps its default");
    assert!(f.identity().contains("uid ?"), "{}", f.identity());
    let stat = DirStat {
        path: "/opt/app".into(),
        exists: true,
        owner: "root".into(),
        group: "root".into(),
        mode: "755".into(),
        writable: false,
    };
    assert!(permission_detail(&stat, &f).contains("uid ?"));
}

#[test]
fn ipv4_lines_are_collected_and_rendered() {
    let f = parse_facts("hostname=box\nipv4=10.0.0.5\nipv4=192.168.1.9\nipv4=\n");
    assert_eq!(f.ipv4, vec!["10.0.0.5", "192.168.1.9"]);
    let text = render("m1", &f);
    assert!(
        text.contains("10.0.0.5") && text.contains("192.168.1.9"),
        "{text}"
    );
}
