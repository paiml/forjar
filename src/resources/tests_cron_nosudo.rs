//! forjar#848: a host with no sudo at all. Real shells, real messages.

use super::cron::*;
use crate::core::types::{MachineTarget, Resource, ResourceType};
use std::path::{Path, PathBuf};

const CMD: &str = "/usr/local/bin/backup.sh";

/// A hermetic PATH holding a fake `crontab` and `id` (non-root) and symlinks to
/// a few real tools, and NO sudo of any kind.
struct NoSudo {
    dir: PathBuf,
}

impl NoSudo {
    fn new(shell: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "forjar-cron-nosudo-{}-{}",
            shell.replace('/', "_"),
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("bin")).expect("bin dir");
        std::fs::create_dir_all(dir.join("tools")).expect("tools dir");
        write_exe(&dir.join("bin/id"), "#!/bin/sh\necho 1000\n");
        write_exe(&dir.join("bin/crontab"), "#!/bin/sh\nexit 1\n");
        NoSudo { dir }
    }
}

fn write_exe(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, body).expect("write exe");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
}

fn cron_resource() -> Resource {
    Resource {
        resource_type: ResourceType::Cron,
        machine: MachineTarget::Single("m1".to_string()),
        owner: Some("root".to_string()),
        name: Some("backup".to_string()),
        schedule: Some("0 3 * * *".to_string()),
        command: Some(CMD.to_string()),
        ..Default::default()
    }
}

/// A host with no sudo at all: the message is the REAL shell's, not a made-up
/// string. sudo is absent from PATH (no fake sudo); bash prints
/// `bash: line 1: sudo: command not found`, dash prints `sh: 1: sudo: not found`.
#[test]
fn fj848_check_skips_when_sudo_is_absent_under_real_shells() {
    let mut ran = 0;
    for shell in ["/bin/bash", "/bin/sh"] {
        let fx = NoSudo::new(shell);
        let tools = fx.dir.join("tools");
        for t in [
            "grep", "awk", "cat", "sed", "printf", "env", "mktemp", "tr", "rm",
        ] {
            let found = std::env::var("PATH")
                .unwrap_or_default()
                .split(':')
                .map(|d| Path::new(d).join(t))
                .find(|c| c.is_file());
            if let Some(f) = found {
                let _ = std::os::unix::fs::symlink(f, tools.join(t));
            }
        }
        let path = format!("{}:{}", fx.dir.join("bin").display(), tools.display());
        let script = check_script(&cron_resource());
        let out = std::process::Command::new(shell)
            .arg("-c")
            .arg(&script)
            .env("PATH", &path)
            .output()
            .expect("shell runs");
        // Prove the premise: this shell really cannot find sudo.
        let probe = std::process::Command::new(shell)
            .arg("-c")
            .arg("sudo -n true")
            .env("PATH", &path)
            .output()
            .expect("probe");
        let msg = String::from_utf8_lossy(&probe.stderr).to_string();
        assert!(
            msg.contains("sudo") && msg.contains("not found"),
            "{shell}: {msg:?}"
        );
        let err = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{shell} ({msg:?}): {err}");
        assert!(String::from_utf8_lossy(&out.stdout).contains("FORJAR_SKIP"));
        ran += 1;
    }
    assert_eq!(ran, 2);
}
