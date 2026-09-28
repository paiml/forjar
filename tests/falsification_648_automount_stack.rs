//! Refs #648: a remount under `x-systemd.automount` must not stack a stale mount.
//!
//! THE FLAW THIS CLOSES. lambda-labs' NAS share is `noauto,x-systemd.automount`.
//! #642's apply detached the gid=1000 mount and ran its own `mount -t cifs`.
//! `mount.cifs` touched the autofs trigger point, systemd's automount fired, and
//! systemd mounted from its generated unit, which still held the OLD options
//! (fstab was rewritten after the mount, and the unit needed a daemon-reload).
//! forjar's mount then landed ON TOP. The check read `findmnt … | tail -1`, the
//! top mount only, and called the stack converged.
//!
//! WHAT THIS TEST MUST NOT BECOME. Asserting that the script says
//! `daemon-reload` stays green if the reload runs AFTER the detach, or after
//! nothing. The fake host below MODELS the kernel stack and systemd: every case
//! RUNS the generated script and asserts on the resulting stack.
//! - `stack` holds one line per mount at the target, `FSTYPE SOURCE OPTIONS`,
//!   top last. `findmnt` prints its column for every line.
//! - `unit` holds the options systemd mounts with. `systemctl daemon-reload`
//!   regenerates it from the target's line in `fstab`.
//! - Touching the path (`ls`, or `mount` itself, as `mount.cifs` does) while
//!   only the autofs trigger is mounted makes systemd mount `unit`'s options.

use forjar::core::types::{Resource, ResourceType};
use forjar::resources::mount::{apply_script, check_script};

const SRC: &str = "//nas/media";
const TGT: &str = "/mnt/unas";
const OLD: &str = "rw,uid=1000,gid=1000,file_mode=0664,dir_mode=0775";
const NEW: &str = "rw,uid=1000,gid=997,file_mode=0664,dir_mode=02775,noauto,x-systemd.automount";

fn cifs(options: &str) -> Resource {
    Resource {
        resource_type: ResourceType::Mount,
        source: Some(SRC.to_string()),
        path: Some(TGT.to_string()),
        fs_type: Some("cifs".to_string()),
        options: Some(options.to_string()),
        ..Default::default()
    }
}

struct Host {
    dir: tempfile::TempDir,
}

impl Host {
    /// A host whose target holds `stack` (bottom first), whose systemd unit
    /// mounts `unit`, and whose fstab holds `fstab_opts` for the target.
    fn new(stack: &[&str], unit: &str, fstab_opts: &str) -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = |n: &str| dir.path().join(n).display().to_string();
        let (st, un, fs, calls) = (p("stack"), p("unit"), p("fstab"), p("calls"));
        let mut s = stack.join("\n");
        if !s.is_empty() {
            s.push('\n');
        }
        std::fs::write(&st, s).expect("stack");
        std::fs::write(&un, unit).expect("unit");
        std::fs::write(&fs, format!("{SRC} {TGT} cifs {fstab_opts} 0 0\n")).expect("fstab");
        // systemd's automount: if only the trigger is mounted, mount the unit.
        let trigger = format!(
            "if grep -q '^autofs ' '{st}' && ! grep -qv '^autofs ' '{st}'; then \
             echo \"cifs {SRC} $(cat '{un}')\" >> '{st}'; echo systemd-automount >> '{calls}'; fi"
        );
        for (name, body) in [
            (
                "findmnt",
                format!(
                    "case \"$*\" in *FSTYPE*) c=1 ;; *OPTIONS*) c=3 ;; *) c=2 ;; esac\n\
                     [ -s '{st}' ] || exit 1\nawk -v c=$c '{{ print $c }}' '{st}'"
                ),
            ),
            (
                "umount",
                format!(
                    "echo \"umount $*\" >> '{calls}'\n[ -e '{busy}' ] && exit 32\n\
                     tail -1 '{st}' | grep -q '^autofs ' && exit 32\n\
                     [ -s '{st}' ] || exit 32\nsed -i '$d' '{st}'",
                    busy = p("busy")
                ),
            ),
            (
                "mount",
                format!("echo \"mount $*\" >> '{calls}'\n{trigger}\necho \"$2 $5 $4\" >> '{st}'"),
            ),
            ("ls", format!("echo \"ls $*\" >> '{calls}'\n{trigger}")),
            (
                "systemctl",
                format!(
                    "echo \"systemctl $*\" >> '{calls}'\n\
                     [ \"$1\" = daemon-reload ] && awk '$2 == \"{TGT}\" {{ printf \"%s\", $4 }}' '{fs}' > '{un}'\nexit 0"
                ),
            ),
            ("mountpoint", format!("[ -s '{st}' ]")),
            ("mkdir", "exit 0".to_string()),
            ("getent", "exit 2".to_string()),
        ] {
            let path = dir.path().join(name);
            std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write fake");
            std::process::Command::new("chmod")
                .arg("+x")
                .arg(&path)
                .status()
                .expect("chmod");
        }
        Host { dir }
    }

    fn run(&self, shell: &str, script: &str) -> i32 {
        let script = script.replace("/etc/fstab", &self.path("fstab"));
        let path = format!(
            "{}:{}",
            self.dir.path().display(),
            std::env::var("PATH").unwrap_or_default()
        );
        std::process::Command::new(shell)
            .arg("-c")
            .arg(script)
            .env("PATH", path)
            .output()
            .expect("spawn shell")
            .status
            .code()
            .unwrap_or(-1)
    }

    fn path(&self, n: &str) -> String {
        self.dir.path().join(n).display().to_string()
    }

    fn read(&self, n: &str) -> String {
        std::fs::read_to_string(self.path(n)).unwrap_or_default()
    }

    /// The filesystem mounts at the target, bottom first, autofs excluded.
    fn fs_mounts(&self) -> Vec<String> {
        self.read("stack")
            .lines()
            .filter(|l| !l.starts_with("autofs "))
            .map(str::to_string)
            .collect()
    }
}

fn autofs() -> String {
    "autofs systemd-1 rw,direct".to_string()
}

fn mounted(opts: &str) -> String {
    format!("cifs {SRC} {opts}")
}

#[test]
fn a_stale_mount_stacked_under_the_right_one_is_drift() {
    let h = Host::new(&[&autofs(), &mounted(OLD), &mounted(NEW)], OLD, NEW);
    assert_ne!(
        h.run("sh", &check_script(&cifs(NEW))),
        0,
        "stacked must be red"
    );
    let h = Host::new(&[&autofs(), &mounted(NEW)], NEW, NEW);
    assert_eq!(
        h.run("sh", &check_script(&cifs(NEW))),
        0,
        "single must be green"
    );
}

#[test]
fn an_automount_ownership_remount_leaves_one_mount_with_the_declared_options() {
    // The live lambda-labs state before infra#1213: stale unit, stale fstab.
    let h = Host::new(&[&autofs(), &mounted(OLD)], OLD, OLD);
    let rc = h.run("bash", &apply_script(&cifs(NEW)));
    let calls = h.read("calls");
    assert_eq!(rc, 0, "{calls}");
    assert_eq!(h.fs_mounts(), vec![mounted(NEW)], "{calls}");
    assert!(
        !calls.contains("mount -t"),
        "systemd owns the mount: {calls}"
    );
    assert_eq!(h.run("sh", &check_script(&cifs(NEW))), 0, "{calls}");
}

#[test]
fn apply_clears_an_existing_stack() {
    // What 1.33.0-rc.1 left on lambda-labs: the stale mount under the right one.
    let h = Host::new(&[&autofs(), &mounted(OLD), &mounted(NEW)], OLD, NEW);
    let rc = h.run("bash", &apply_script(&cifs(NEW)));
    let calls = h.read("calls");
    assert_eq!(rc, 0, "{calls}");
    assert_eq!(h.fs_mounts(), vec![mounted(NEW)], "{calls}");
}

#[test]
fn an_untriggered_automount_is_mounted_by_systemd() {
    let h = Host::new(&[&autofs()], OLD, OLD);
    let rc = h.run("bash", &apply_script(&cifs(NEW)));
    let calls = h.read("calls");
    assert_eq!(rc, 0, "{calls}");
    assert_eq!(h.fs_mounts(), vec![mounted(NEW)], "{calls}");
}

#[test]
fn a_busy_stack_fails_loudly_and_mounts_nothing() {
    let h = Host::new(&[&autofs(), &mounted(OLD)], OLD, OLD);
    std::fs::write(h.path("busy"), "").expect("busy");
    let rc = h.run("bash", &apply_script(&cifs(NEW)));
    let calls = h.read("calls");
    assert_ne!(rc, 0, "{calls}");
    assert_eq!(h.fs_mounts(), vec![mounted(OLD)], "{calls}");
    assert!(!calls.contains("umount -l"), "never a lazy detach: {calls}");
}

#[test]
fn a_busy_wrong_source_under_automount_fails_loudly_and_never_lazy_detaches() {
    let stale = "cifs //nas/old rw,uid=1000,gid=1000";
    let h = Host::new(&[&autofs(), stale], OLD, OLD);
    std::fs::write(h.path("busy"), "").expect("busy");
    let rc = h.run("bash", &apply_script(&cifs(NEW)));
    let calls = h.read("calls");
    assert_ne!(rc, 0, "{calls}");
    assert!(!calls.contains("umount -l"), "never a lazy detach: {calls}");
    assert_eq!(h.fs_mounts(), vec![stale.to_string()], "{calls}");
}

#[test]
fn a_plain_mount_still_uses_mount_and_never_systemctl() {
    let plain = "rw,uid=1000,gid=997,file_mode=0664,dir_mode=02775";
    let h = Host::new(&[&mounted(OLD)], OLD, OLD);
    let rc = h.run("bash", &apply_script(&cifs(plain)));
    let calls = h.read("calls");
    assert_eq!(rc, 0, "{calls}");
    assert_eq!(
        h.fs_mounts(),
        vec![format!("cifs {SRC} {plain}")],
        "{calls}"
    );
    assert!(!calls.contains("systemctl"), "{calls}");
}
