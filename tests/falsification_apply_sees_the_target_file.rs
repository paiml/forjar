//! forjar#305: `apply` reported `unchanged` for a file that had drifted on the
//! target, and `drift` did not report it either — so a drifted file was neither
//! detected nor corrected by normal operation.
//!
//! ROOT CAUSE. forjar records two observables per file resource on every apply:
//!
//!   live_hash     content + owner + group + mode + existence, from
//!                 `state_query_script` run ON THE TARGET through the transport.
//!                 Complete and transport-correct.
//!   content_hash  bytes only, hashed on the CONTROLLER, and written only when
//!                 `resource.content.is_some()`.
//!
//! `detect_nonfile_drift` excluded `ResourceType::File`, so the complete oracle
//! was written every run and read by nothing. On this fleet, 320 of 329 locked
//! file resources carried no `content_hash` at all — 97% invisible — while 323
//! carried the unread `live_hash`.
//!
//! WHY THE ASSERTIONS LOOK LIKE THIS. Every assertion below is on the BYTES AT
//! THE PATH, or on a drift finding naming the resource. Never on the summary
//! line: `0 converged, 1 unchanged` is exactly what the defect printed, so a
//! test that counts converged resources passes against the bug. Asserting on
//! generated script text is what let the sibling heredoc defect live for five
//! months.
//!
//! WHY THE MATRIX. `{source:, content:} x {content, mode, delete}`. Testing one
//! cell is how this survived: a sandbox where the controller and the target are
//! the same box makes `content:` look protected, because `hash_file` on the
//! controller happens to find the file. On a real SSH host it does not, and
//! inline content degrades to the same silence as `source:`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forjar() -> &'static str {
    env!("CARGO_BIN_EXE_forjar")
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
    fn state(&self) -> PathBuf {
        self.path("state")
    }

    /// A config with one file resource, declared either by `source:` or by
    /// inline `content:`. Both shapes must converge; see the module docs for
    /// why testing only one is misleading.
    fn write_config(&self, shape: Shape) -> PathBuf {
        let target = self.path("target.txt");
        let cfg = self.path("forjar.yaml");
        let body = match shape {
            Shape::Source => {
                let src = self.path("source.txt");
                fs::write(&src, DECLARED).unwrap();
                format!(
                    "  managed: {{ type: file, machine: local, path: {}, source: {}, mode: \"0644\" }}\n",
                    target.display(),
                    src.display()
                )
            }
            Shape::Content => format!(
                "  managed: {{ type: file, machine: local, path: {}, content: \"{}\", mode: \"0644\" }}\n",
                target.display(),
                DECLARED.replace('\n', "\\n")
            ),
        };
        fs::write(
            &cfg,
            format!(
                "version: \"1.0\"\nname: conv\nmachines: {{ local: {{ hostname: localhost, addr: 127.0.0.1 }} }}\nresources:\n{body}"
            ),
        )
        .unwrap();
        cfg
    }

    fn run(&self, args: &[&str]) -> (i32, String) {
        let out = Command::new(forjar())
            .args(args)
            .output()
            .expect("forjar failed to start");
        let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
        s.push_str(&String::from_utf8_lossy(&out.stderr));
        (out.status.code().unwrap_or(-1), s)
    }

    fn apply(&self, cfg: &Path) -> (i32, String) {
        self.run(&[
            "apply",
            "-f",
            cfg.to_str().unwrap(),
            "--state-dir",
            self.state().to_str().unwrap(),
            "--yes",
        ])
    }

    fn drift(&self, cfg: &Path) -> (i32, String) {
        self.run(&[
            "drift",
            "-f",
            cfg.to_str().unwrap(),
            "--state-dir",
            self.state().to_str().unwrap(),
        ])
    }
}

#[derive(Clone, Copy, Debug)]
enum Shape {
    Source,
    Content,
}

const DECLARED: &str = "DECLARED\n";

/// Strip ANSI so assertions do not depend on colour.
fn plain(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for c2 in chars.by_ref() {
                if c2.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Converge, then hand back the config and the managed path.
fn converged(sb: &Sandbox, shape: Shape) -> (PathBuf, PathBuf) {
    let cfg = sb.write_config(shape);
    let (ec, out) = sb.apply(&cfg);
    assert_eq!(ec, 0, "first apply did not converge:\n{}", plain(&out));
    let target = sb.path("target.txt");
    assert_eq!(
        fs::read_to_string(&target).unwrap(),
        DECLARED,
        "first apply did not write the declared bytes"
    );
    (cfg, target)
}

// ── DETECTION ───────────────────────────────────────────────────────────────
// `drift` must name the resource. The assertion is on the finding, not on an
// exit code alone: exit codes are shared with "could not read the config".

fn assert_drift_reports(sb: &Sandbox, cfg: &Path, why: &str) {
    let (_, out) = sb.drift(cfg);
    let out = plain(&out);
    assert!(
        out.contains("DRIFTED") && out.contains("managed"),
        "drift did not report {why}. It printed:\n{out}"
    );
}

#[test]
fn drift_sees_content_tamper_source() {
    let sb = Sandbox::new();
    let (cfg, target) = converged(&sb, Shape::Source);
    fs::write(&target, "TAMPERED\n").unwrap();
    assert_drift_reports(&sb, &cfg, "a content change on a source: file");
}

#[test]
fn drift_sees_content_tamper_inline() {
    let sb = Sandbox::new();
    let (cfg, target) = converged(&sb, Shape::Content);
    fs::write(&target, "TAMPERED\n").unwrap();
    assert_drift_reports(&sb, &cfg, "a content change on an inline content: file");
}

#[test]
fn drift_sees_mode_tamper() {
    let sb = Sandbox::new();
    let (cfg, target) = converged(&sb, Shape::Source);
    // Content is untouched — only the mode moves. A bytes-only comparison
    // cannot see this, which is why `live_hash` (owner+group+mode+content) is
    // the stronger oracle and the one worth reading.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&target, fs::Permissions::from_mode(0o777)).unwrap();
    }
    assert_drift_reports(&sb, &cfg, "a mode change with identical content");
}

#[test]
fn drift_sees_deletion() {
    let sb = Sandbox::new();
    let (cfg, target) = converged(&sb, Shape::Source);
    fs::remove_file(&target).unwrap();
    assert_drift_reports(&sb, &cfg, "a deleted managed file");
}

// ── CONVERGENCE ─────────────────────────────────────────────────────────────
// The whole point. `apply` must leave the declared bytes at the path.

fn assert_apply_converges(shape: Shape, tamper: &dyn Fn(&Path), why: &str) {
    let sb = Sandbox::new();
    let (cfg, target) = converged(&sb, shape);
    tamper(&target);

    let (_, out) = sb.apply(&cfg);

    // THE assertion. Not the summary line — `0 converged, 1 unchanged` is what
    // the defect printed while leaving the tamper in place.
    let actual = fs::read_to_string(&target).unwrap_or_else(|e| {
        panic!(
            "{why}: the managed file is unreadable after apply ({e}):\n{}",
            plain(&out)
        )
    });
    assert_eq!(
        actual,
        DECLARED,
        "{why}: apply did not restore the declared bytes ({shape:?}). It printed:\n{}",
        plain(&out)
    );
}

#[test]
fn apply_restores_a_tampered_source_file() {
    assert_apply_converges(
        Shape::Source,
        &|p| fs::write(p, "TAMPERED\n").unwrap(),
        "content tamper",
    );
}

#[test]
fn apply_restores_a_tampered_inline_content_file() {
    assert_apply_converges(
        Shape::Content,
        &|p| fs::write(p, "TAMPERED\n").unwrap(),
        "content tamper",
    );
}

#[test]
fn apply_recreates_a_deleted_file() {
    assert_apply_converges(Shape::Source, &|p| fs::remove_file(p).unwrap(), "deletion");
}

// ── THE CONTROL, AND THE TEST THAT KILLS THE WRONG FIX ──────────────────────

#[test]
fn an_untampered_file_still_reports_unchanged() {
    // Without this, "always rewrite everything" passes every test above. A fix
    // that converges by never trusting the lock is not a fix — it is `--force`
    // on by default, and it would re-run every resource on every apply.
    let sb = Sandbox::new();
    let (cfg, _) = converged(&sb, Shape::Source);
    let (ec, out) = sb.apply(&cfg);
    let out = plain(&out);
    assert_eq!(ec, 0, "a converged stack failed to re-apply:\n{out}");
    assert!(
        out.contains("unchanged"),
        "a converged stack did not report unchanged — the fix re-runs everything:\n{out}"
    );
}

#[test]
fn a_managed_directory_does_not_drift_when_its_contents_change() {
    // THE ANTI-BRICK TEST. `state_query_script` folds `stat`'s `size=%s` into
    // live_hash, and a directory's size grows with its entry count — measured
    // 4096 -> 12288 at 400 entries. So wiring live_hash into drift detection
    // without excluding size for directories marks EVERY managed directory
    // permanently drifted, and once the apply gate reads drift, permanently
    // un-appliable. A directory's identity under forjar is
    // owner/group/mode/existence; how many files someone put inside it is not
    // drift.
    //
    // This test must stay GREEN. If it goes red, the fix has bricked every
    // managed directory on the fleet.
    let sb = Sandbox::new();
    let dir = sb.path("managed-dir");
    let cfg = sb.path("forjar.yaml");
    fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: conv\nmachines: {{ local: {{ hostname: localhost, addr: 127.0.0.1 }} }}\nresources:\n  d: {{ type: file, machine: local, path: {}, state: directory, mode: \"0755\" }}\n",
            dir.display()
        ),
    )
    .unwrap();

    let (ec, out) = sb.apply(&cfg);
    assert_eq!(ec, 0, "directory did not converge:\n{}", plain(&out));

    // Enough entries to force the directory past one block.
    for i in 0..400 {
        fs::write(dir.join(format!("f{i}")), b"").unwrap();
    }

    let (ec, out) = sb.drift(&cfg);
    let out = plain(&out);
    assert!(
        !out.contains("DRIFTED"),
        "a managed directory was reported as drifted merely because files were \
         added inside it — this would brick every managed directory:\n{out}"
    );
    assert_eq!(ec, 0, "drift failed on an unchanged directory:\n{out}");
}
