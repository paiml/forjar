//! forjar#600: `apply --refresh` must write a file whose CONTENT or MODE is
//! stale, not certify it because something exists at the path.
//!
//! A `state: file` resource's `check_script` was `test -f`. `--refresh`
//! ("re-run check scripts, only re-apply what fails") therefore re-applied
//! nothing for a file that existed with the wrong bytes, through two doors:
//!
//! - NO lock entry: refresh seeding asked the same check and seeded the stale
//!   file as converged (the issue's reproduction);
//! - a LOCK entry: measured on a fleet host, where one run printed
//!   `drift: … pv.pin content changed` and then `0 converged, 2 unchanged`,
//!   leaving `0.65.2` where `0.68.2` was declared.
//!
//! Every assertion here is on the FILE, not on what forjar printed about it.
//! The last test is the negative control: a file that already matches must NOT
//! be rewritten, so the fix cannot be "always report divergence".

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const DECLARED: &str = "0.68.2\n";
const STALE: &str = "0.65.2\n";

fn forjar() -> &'static str {
    env!("CARGO_BIN_EXE_forjar")
}

#[derive(Clone, Copy, Debug)]
enum Shape {
    Content,
    Source,
}

struct Sandbox {
    dir: tempfile::TempDir,
}

impl Sandbox {
    fn new() -> Self {
        Self {
            dir: tempfile::tempdir().expect("tempdir"),
        }
    }
    fn path(&self, rel: &str) -> PathBuf {
        self.dir.path().join(rel)
    }
    fn target(&self) -> PathBuf {
        self.path("pv.pin")
    }
    fn config(&self, shape: Shape) -> PathBuf {
        let body = match shape {
            Shape::Content => format!(
                "  pin: {{ type: file, machine: local, path: {}, content: \"{}\", mode: \"0644\" }}\n",
                self.target().display(),
                DECLARED.replace('\n', "\\n")
            ),
            Shape::Source => {
                let src = self.path("pin.src");
                fs::write(&src, DECLARED).unwrap();
                format!(
                    "  pin: {{ type: file, machine: local, path: {}, source: {}, mode: \"0644\" }}\n",
                    self.target().display(),
                    src.display()
                )
            }
        };
        let cfg = self.path("forjar.yaml");
        fs::write(
            &cfg,
            format!(
                "version: \"1.0\"\nname: refresh600\nmachines: {{ local: {{ hostname: localhost, addr: 127.0.0.1 }} }}\nresources:\n{body}"
            ),
        )
        .unwrap();
        cfg
    }
    fn apply(&self, cfg: &Path, refresh: bool) -> String {
        let state = self.path("state");
        let mut args = vec![
            "apply",
            "-f",
            cfg.to_str().unwrap(),
            "--state-dir",
            state.to_str().unwrap(),
            "--yes",
        ];
        if refresh {
            args.push("--refresh");
        }
        let out = Command::new(forjar())
            .args(&args)
            .output()
            .expect("forjar failed to start");
        let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
        s.push_str(&String::from_utf8_lossy(&out.stderr));
        assert!(out.status.success(), "forjar apply failed:\n{s}");
        s
    }
    fn stale(&self) {
        fs::write(self.target(), STALE).unwrap();
    }
    fn content(&self) -> String {
        fs::read_to_string(self.target()).unwrap()
    }
    fn mode(&self) -> u32 {
        fs::metadata(self.target()).unwrap().permissions().mode() & 0o7777
    }
}

/// The issue's reproduction: the stale file is there, the lock is empty.
#[test]
fn refresh_writes_a_stale_file_that_has_no_lock_entry() {
    for shape in [Shape::Content, Shape::Source] {
        let sb = Sandbox::new();
        let cfg = sb.config(shape);
        sb.stale();
        let out = sb.apply(&cfg, true);
        assert_eq!(
            sb.content(),
            DECLARED,
            "{shape:?}: --refresh left the stale file in place\n{out}"
        );
    }
}

/// The fleet's path: converged once, then another writer changes the file.
#[test]
fn refresh_writes_a_stale_file_the_lock_calls_converged() {
    for shape in [Shape::Content, Shape::Source] {
        let sb = Sandbox::new();
        let cfg = sb.config(shape);
        sb.apply(&cfg, false);
        assert_eq!(sb.content(), DECLARED, "{shape:?}: first apply");
        sb.stale();
        let out = sb.apply(&cfg, true);
        assert_eq!(
            sb.content(),
            DECLARED,
            "{shape:?}: --refresh reported and did not write\n{out}"
        );
    }
}

/// Mode is part of the declaration too.
#[test]
fn refresh_restores_a_drifted_mode() {
    let sb = Sandbox::new();
    let cfg = sb.config(Shape::Content);
    sb.apply(&cfg, false);
    fs::set_permissions(sb.target(), fs::Permissions::from_mode(0o600)).unwrap();
    let out = sb.apply(&cfg, true);
    assert_eq!(sb.mode(), 0o644, "--refresh left mode 0600\n{out}");
}

/// NEGATIVE CONTROL: a file that already matches is not rewritten, so the fix
/// cannot be passed by a check that always reports divergence.
#[test]
fn refresh_leaves_a_matching_file_alone() {
    let sb = Sandbox::new();
    let cfg = sb.config(Shape::Content);
    sb.apply(&cfg, false);
    let before = fs::metadata(sb.target()).unwrap().modified().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let out = sb.apply(&cfg, true);
    let after = fs::metadata(sb.target()).unwrap().modified().unwrap();
    assert_eq!(sb.content(), DECLARED);
    assert_eq!(
        before, after,
        "--refresh rewrote a file that already matched\n{out}"
    );
}
