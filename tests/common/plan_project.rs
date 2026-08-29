//! Refs #358 — the two-resource fixture the plan-file tests are all written
//! against.
//!
//! Two file resources on one localhost machine, tagged and grouped apart, so
//! that:
//!
//! * the stack can be PARTIALLY converged, which is the state the reported
//!   evasion needs and a one-resource fixture cannot reach;
//! * every selector (`-m`, `-r`, `-t`, `-g`) has something to select and
//!   something to miss;
//! * an apply is real but harmless — the only side effect is two files under a
//!   `tempdir`.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

pub fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

/// stdout and stderr together — refusals land on both.
pub fn combined(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

pub struct Project {
    pub cfg: PathBuf,
    pub state: PathBuf,
    pub alpha: PathBuf,
    pub bravo: PathBuf,
}

pub fn project(dir: &Path) -> Project {
    let alpha = dir.join("alpha.txt");
    let bravo = dir.join("bravo.txt");
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\n\
             name: partial\n\
             machines:\n\
             \x20 web:\n\
             \x20   hostname: localhost\n\
             \x20   addr: 127.0.0.1\n\
             resources:\n\
             \x20 alpha:\n\
             \x20   type: file\n\
             \x20   machine: web\n\
             \x20   path: {}\n\
             \x20   content: \"A\"\n\
             \x20   tags: [t_alpha]\n\
             \x20   resource_group: ga\n\
             \x20 bravo:\n\
             \x20   type: file\n\
             \x20   machine: web\n\
             \x20   path: {}\n\
             \x20   content: \"B\"\n\
             \x20   tags: [t_bravo]\n\
             \x20   resource_group: gb\n",
            alpha.display(),
            bravo.display()
        ),
    )
    .expect("write config");
    Project {
        cfg,
        state: dir.join("state"),
        alpha,
        bravo,
    }
}

impl Project {
    /// `forjar plan [extra] --out <out>`.
    pub fn plan(&self, out: &Path, extra: &[&str]) -> std::process::Output {
        forjar()
            .args(["plan", "-f"])
            .arg(&self.cfg)
            .arg("--state-dir")
            .arg(&self.state)
            .args(extra)
            .arg("--out")
            .arg(out)
            .output()
            .expect("run plan")
    }

    /// The same, asserting success — for the many tests where writing the plan
    /// is setup rather than the thing under test.
    pub fn save_plan(&self, out: &Path) {
        let o = self.plan(out, &[]);
        assert!(o.status.success(), "plan --out: {}", combined(&o));
    }

    /// `forjar apply --plan-file <plan> --yes [extra]`.
    pub fn apply_plan(&self, plan: &Path, extra: &[&str]) -> std::process::Output {
        forjar()
            .args(["apply", "-f"])
            .arg(&self.cfg)
            .arg("--state-dir")
            .arg(&self.state)
            .arg("--plan-file")
            .arg(plan)
            .arg("--yes")
            .args(extra)
            .output()
            .expect("run apply")
    }

    /// `forjar apply [extra] --yes`, no plan file.
    pub fn apply(&self, extra: &[&str]) -> std::process::Output {
        forjar()
            .args(["apply", "-f"])
            .arg(&self.cfg)
            .arg("--state-dir")
            .arg(&self.state)
            .arg("--yes")
            .args(extra)
            .output()
            .expect("run apply")
    }

    /// Which of the two managed files exist.
    pub fn converged(&self) -> (bool, bool) {
        (self.alpha.exists(), self.bravo.exists())
    }

    /// Converge `bravo` only, leaving `alpha` pending — a PARTIALLY converged
    /// stack, which is every real deployment and the state the reported evasion
    /// needs.
    pub fn converge_bravo(&self) {
        let out = self.apply(&["-r", "bravo"]);
        assert!(out.status.success(), "seed: {}", combined(&out));
        assert_eq!(self.converged(), (false, true), "partial stack");
    }
}

/// A fresh project with a sealed whole-stack plan naming both pending creates.
pub fn planned(dir: &Path) -> (Project, PathBuf) {
    let p = project(dir);
    let plan_path = dir.join("p.json");
    p.save_plan(&plan_path);
    (p, plan_path)
}
