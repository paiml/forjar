//! Shared fixtures for the "replace a binary that is running" falsification.
//!
//! Everything here builds REAL conditions on disk: a real executable actually
//! executing, a real dangling symlink, a real `curl` on PATH serving a real
//! file. Nothing about the defect under test is simulated, because the defect
//! is a kernel refusal (`ETXTBSY`) and a coreutils refusal ("not writing
//! through dangling symlink") — neither of which a mock can produce.

#![allow(dead_code)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

/// A child process holding its own executable open, killed on drop.
///
/// The kill matters: a leaked `sleep` keeps ETXTBSY on the sandbox for its
/// whole lifetime, so a later test in the same run would measure the leak
/// rather than its own fixture.
pub struct RunningBinary {
    child: Child,
}

impl Drop for RunningBinary {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A fresh, empty sandbox directory under the system temp dir.
pub fn sandbox(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("forjar-etxtbsy-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create sandbox");
    dir
}

/// Copy `src` to `dest` and EXECUTE it, so `dest`'s inode is held busy.
///
/// Returns once the kernel is observably holding the file: we wait for
/// `/proc/<pid>/exe` to resolve to `dest` where procfs exists, and otherwise
/// poll until the process has been alive long enough to have exec'd. Without
/// that wait the test races its own fixture and can measure a `cp` that
/// happened *before* the exec, which would pass for the wrong reason.
pub fn hold_running(src: &str, dest: &Path) -> RunningBinary {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).expect("parent dir");
    }
    // Remove first: `dest` may itself be a running binary from a prior step,
    // and this helper must not be the thing that takes ETXTBSY.
    let _ = fs::remove_file(dest);
    fs::copy(src, dest).unwrap_or_else(|e| panic!("copy {src} -> {}: {e}", dest.display()));
    fs::set_permissions(dest, fs::Permissions::from_mode(0o755)).expect("chmod 755");

    let guard = RunningBinary {
        child: spawn_retrying_etxtbsy(dest),
    };
    wait_until_busy(dest, guard.child.id());
    guard
}

/// Start the fixture, retrying the kernel's own ETXTBSY race.
///
/// This is a HARNESS artifact, not the defect: a Rust test binary runs its
/// tests on threads of one process, so a sibling thread's `fs::copy` write
/// descriptor can be inherited across the `fork` behind `spawn` and still be
/// open when this child reaches `execve`. rust-lang/rust#39960. Left
/// unhandled it makes the fixture itself flaky (measured: 2 of 5 runs), and a
/// flaky fixture is indistinguishable from a flaky guard.
fn spawn_retrying_etxtbsy(dest: &Path) -> Child {
    let mut last = String::new();
    for _ in 0..200 {
        match Command::new(dest)
            .arg("600")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => return c,
            Err(e) if e.raw_os_error() == Some(26) => {
                last = e.to_string();
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(e) => panic!("spawn {}: {e}", dest.display()),
        }
    }
    panic!("spawn {} kept returning ETXTBSY: {last}", dest.display());
}

/// Block until the kernel reports the file as the running image, or panic.
fn wait_until_busy(dest: &Path, pid: u32) {
    let exe = PathBuf::from(format!("/proc/{pid}/exe"));
    for _ in 0..200 {
        if exe.exists() {
            if let Ok(target) = fs::read_link(&exe) {
                if target == dest {
                    return;
                }
            }
        } else if !PathBuf::from("/proc").exists() {
            // No procfs (macOS): fall back to a bounded settle.
            std::thread::sleep(std::time::Duration::from_millis(200));
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!(
        "fixture never became busy: /proc/{pid}/exe did not resolve to {}",
        dest.display()
    );
}

/// Write a `curl` stub into a fresh directory and return that directory, for
/// prepending to `PATH`.
///
/// It answers the two calls the `github_release` script makes:
///   `curl -fsSL <api-url>`            -> one release-JSON line naming the asset
///   `curl -fsSL -o <file> <asset-url>` -> the staged asset bytes
///
/// Exactly ONE matching line is emitted deliberately: the generated script
/// pipes through `head -1` under `set -o pipefail`, and a multi-line producer
/// would make the pipeline's own SIGPIPE indistinguishable from the failure
/// under test.
pub fn curl_stub(dir: &Path, asset: &Path, url: &str) -> PathBuf {
    let stub_dir = dir.join("stub-bin");
    fs::create_dir_all(&stub_dir).expect("stub dir");
    let stub = stub_dir.join("curl");
    fs::write(
        &stub,
        format!(
            "#!/bin/sh\n\
             out=''\n\
             while [ $# -gt 0 ]; do\n\
             \x20 case \"$1\" in\n\
             \x20\x20\x20 -o) out=\"$2\"; shift 2 ;;\n\
             \x20\x20\x20 -*) shift ;;\n\
             \x20\x20\x20 *) shift ;;\n\
             \x20 esac\n\
             done\n\
             if [ -n \"$out\" ]; then\n\
             \x20 cat '{asset}' > \"$out\"\n\
             else\n\
             \x20 printf '%s\\n' '  \"browser_download_url\": \"{url}\"'\n\
             fi\n",
            asset = asset.display(),
            url = url,
        ),
    )
    .expect("write curl stub");
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).expect("chmod stub");
    stub_dir
}
