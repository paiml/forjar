//! forjar#449 (found by the S4 dogfood): `destroy` recorded no generation, so
//! the destroy→undo roundtrip the contract `destroy-undo-roundtrip-v1` names
//! could not happen.
//!
//! Measured on the integration binary (main + every open PR), a /tmp-only
//! local sandbox with `policy.snapshot_generations: 3`:
//!
//! ```text
//!   apply   -> state/generations: 0 current
//!   destroy -> file gone; state/generations: 0 current     (no new generation)
//!   undo    -> exit 1: generation 0 is current, so only 0 earlier generation(s) exist
//! ```
//!
//! The control passed: apply(one) → apply(two) → undo restores `one`, with
//! generations 0, 1. So `undo` and `apply` are fine; `destroy` mutates the
//! state directory without recording the generation it replaces. These cases
//! drive the real binary through apply → destroy → undo and assert the BYTES
//! at the managed path, never a summary line.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const FORJAR: &str = env!("CARGO_BIN_EXE_forjar");

struct Sbx {
    dir: PathBuf,
}

impl Sbx {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("forjar-449-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let me = Self { dir };
        me.write_config("dogfood");
        me
    }
    fn cfg(&self) -> PathBuf {
        self.dir.join("forjar.yaml")
    }
    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }
    fn target(&self) -> PathBuf {
        self.dir.join("out.txt")
    }
    fn write_config(&self, content: &str) {
        let yaml = format!(
            "version: \"1.0\"\nname: undo\npolicy: {{ snapshot_generations: 3 }}\n\
             machines: {{ local: {{ hostname: localhost, addr: 127.0.0.1 }} }}\n\
             resources:\n  a: {{ type: file, machine: local, path: {}, content: \"{content}\" }}\n",
            self.target().display()
        );
        std::fs::write(self.cfg(), yaml).unwrap();
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(FORJAR)
            .args(args)
            .arg("-f")
            .arg(self.cfg())
            .arg("--state-dir")
            .arg(self.state())
            .current_dir(&self.dir)
            .output()
            .expect("run forjar")
    }
    fn generations(&self) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(self.state().join("generations"))
            .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect())
            .unwrap_or_default();
        v.retain(|n| n.chars().all(|c| c.is_ascii_digit()));
        v.sort();
        v
    }
}

impl Drop for Sbx {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn combined(o: &Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))
}

fn assert_bytes(p: &Path, want: &str, what: &str) {
    let got = std::fs::read_to_string(p).unwrap_or_default();
    assert_eq!(got, want, "{what}: the managed file's bytes are wrong");
}

/// The control, so the roundtrip case cannot pass on a broken `undo`.
#[test]
fn control_apply_apply_undo_restores_the_earlier_generation() {
    let sb = Sbx::new("control");
    sb.write_config("one");
    assert!(sb.run(&["apply", "--yes"]).status.success());
    sb.write_config("two");
    assert!(sb.run(&["apply", "--yes"]).status.success());
    assert_bytes(&sb.target(), "two", "after the second apply");
    let out = sb.run(&["undo", "--yes"]);
    assert!(out.status.success(), "undo across two applies failed:\n{}", combined(&out));
    assert_bytes(&sb.target(), "one", "after undo");
}

/// #449: destroy must record the generation it replaces, exactly as apply does.
#[test]
fn destroy_records_a_generation() {
    let sb = Sbx::new("gen");
    assert!(sb.run(&["apply", "--yes"]).status.success());
    let before = sb.generations();
    assert_eq!(before, vec!["0".to_string()], "one generation after the first apply");
    let out = sb.run(&["destroy", "--yes"]);
    assert!(out.status.success(), "destroy failed:\n{}", combined(&out));
    assert!(!sb.target().exists(), "destroy did not remove the managed file");
    let after = sb.generations();
    assert!(
        after.len() > before.len(),
        "destroy recorded no generation: {before:?} -> {after:?} (undo has nothing to rewind to)"
    );
}

/// The contract's own sentence: a destroy followed by `forjar undo` restores
/// the prior generation's state — measured at the path.
#[test]
fn destroy_then_undo_restores_the_managed_file() {
    let sb = Sbx::new("roundtrip");
    assert!(sb.run(&["apply", "--yes"]).status.success());
    assert_bytes(&sb.target(), "dogfood", "after apply");
    assert!(sb.run(&["destroy", "--yes"]).status.success());
    assert!(!sb.target().exists(), "destroy did not remove the managed file");
    let out = sb.run(&["undo", "--yes"]);
    assert!(
        out.status.success(),
        "undo after destroy failed (the contract destroy-undo-roundtrip-v1 says it restores the prior generation):\n{}",
        combined(&out)
    );
    assert_bytes(&sb.target(), "dogfood", "after destroy → undo");
}
