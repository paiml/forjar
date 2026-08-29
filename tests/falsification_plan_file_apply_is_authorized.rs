//! forjar#370: `apply --plan-file` never reached `check_operator_auth`.
//!
//! `src/cli/dispatch_apply_b.rs` runs the operator gate as the FIRST line of
//! `apply_execute` (:266). `apply_mode_exits` returns for `--plan-file` before
//! that call, so the whole of FJ-2300's `allowed_operators` was skippable by
//! routing through a plan file. Measured on 1.21.0 with
//! `allowed_operators: [alice]`, as a non-alice operator:
//!
//! ```text
//!   forjar apply --yes                                        -> not authorized  EXIT=1
//!   forjar plan --out p.json                                  -> EXIT=0
//!   forjar apply --plan-file p.json --yes                     -> 2 converged     EXIT=0
//!   forjar apply --plan-file p2.json --operator mallory --yes -> Plan applied    EXIT=0
//! ```
//!
//! A plan file is UNAUTHENTICATED — any user can write one in a text editor —
//! so the bypass needed no privilege of its own.
//!
//! Everything below drives the shipped binary. An in-process test of
//! `cmd_apply_from_plan` would have missed this defect entirely: the hole was in
//! which branch of the dispatcher reached the gate, not in the gate.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const FORJAR: &str = env!("CARGO_BIN_EXE_forjar");

struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    /// `operators` empty means the config restricts nobody.
    fn new(name: &str, operators: &[&str]) -> Self {
        let dir = std::env::temp_dir().join(format!("forjar-370-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("sandbox");
        let me = Self { dir };
        me.write_config(operators);
        me
    }

    fn cfg(&self) -> PathBuf {
        self.dir.join("forjar.yaml")
    }

    fn plan_file(&self) -> PathBuf {
        self.dir.join("p.json")
    }

    fn managed(&self) -> [PathBuf; 2] {
        [self.dir.join("f1.txt"), self.dir.join("f2.txt")]
    }

    fn write_config(&self, operators: &[&str]) {
        let allow = if operators.is_empty() {
            String::new()
        } else {
            let list = operators
                .iter()
                .map(|o| format!("      - {o}\n"))
                .collect::<String>();
            format!("    allowed_operators:\n{list}")
        };
        let d = self.dir.display();
        std::fs::write(
            self.cfg(),
            format!(
                r#"version: "1.0"
name: authz
machines:
  sandbox:
    hostname: sandbox
    addr: 127.0.0.1
{allow}resources:
  f1:
    type: file
    machine: sandbox
    path: {d}/f1.txt
    content: "one"
  f2:
    type: file
    machine: sandbox
    path: {d}/f2.txt
    content: "two"
"#
            ),
        )
        .expect("config");
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(FORJAR)
            .args(args)
            .current_dir(&self.dir)
            .output()
            .expect("run forjar")
    }

    /// Produce the plan artifact. Deliberately NOT gated — see below.
    fn save_plan(&self) -> Output {
        let cfg = self.cfg();
        self.run(&[
            "plan",
            "-f",
            cfg.to_str().unwrap(),
            "--out",
            self.plan_file().to_str().unwrap(),
        ])
    }

    fn apply_from_plan(&self, operator: Option<&str>) -> Output {
        let cfg = self.cfg();
        let pf = self.plan_file();
        let mut args = vec![
            "apply",
            "-f",
            cfg.to_str().unwrap(),
            "--plan-file",
            pf.to_str().unwrap(),
            "--yes",
        ];
        if let Some(op) = operator {
            args.push("--operator");
            args.push(op);
        }
        self.run(&args)
    }

    fn nothing_was_written(&self) -> bool {
        self.managed().iter().all(|p| !p.exists())
    }

    fn reset_targets(&self) {
        for p in self.managed() {
            let _ = std::fs::remove_file(p);
        }
        let _ = std::fs::remove_dir_all(self.dir.join("state"));
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn assert_saved(o: &Output, plan: &Path) {
    assert!(
        o.status.success() && plan.exists(),
        "the fixture could not produce a plan file, so the test below would be \
         vacuous: exit {:?}, stderr: {}",
        o.status.code(),
        stderr(o)
    );
}

// ── The control: the gate exists on the ordinary path ───────────────────────

/// Without this, "the plan path refuses" could mean the config was malformed.
#[test]
fn control_the_ordinary_apply_refuses_an_unlisted_operator() {
    let sb = Sandbox::new("control", &["alice"]);
    let cfg = sb.cfg();
    let out = sb.run(&[
        "apply",
        "--yes",
        "-f",
        cfg.to_str().unwrap(),
        "--operator",
        "mallory",
    ]);
    assert!(!out.status.success(), "FJ-2300's gate is not armed at all");
    assert!(
        stderr(&out).contains("not authorized"),
        "expected an authorization refusal: {}",
        stderr(&out)
    );
    assert!(sb.nothing_was_written(), "a refused apply wrote files");
}

// ── The defect ──────────────────────────────────────────────────────────────

/// The bypass, with an explicit `--operator` the config does not list.
#[test]
fn a_plan_file_apply_refuses_an_unlisted_operator() {
    let sb = Sandbox::new("named", &["alice"]);
    assert_saved(&sb.save_plan(), &sb.plan_file());

    let out = sb.apply_from_plan(Some("mallory"));
    assert!(
        !out.status.success(),
        "`apply --plan-file --operator mallory` applied the plan; a plan file is \
         unauthenticated, so this bypasses allowed_operators with no privilege. \
         stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        stderr(&out).contains("not authorized"),
        "the refusal must be the SAME one the ordinary path gives: {}",
        stderr(&out)
    );
    assert!(
        sb.nothing_was_written(),
        "the apply was refused but the files were written anyway"
    );
}

/// The same bypass with no `--operator` at all, which is how it would actually
/// be reached: the ambient identity is whoever is logged in.
#[test]
fn a_plan_file_apply_refuses_the_ambient_operator_too() {
    // Nobody's `$USER@$(hostname)` is this.
    let sb = Sandbox::new("ambient", &["nobody-has-this-identity"]);
    assert_saved(&sb.save_plan(), &sb.plan_file());

    let out = sb.apply_from_plan(None);
    assert!(
        !out.status.success(),
        "an unauthorized user applied a plan file without naming an operator: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(sb.nothing_was_written());
}

// ── The guards against over-correcting ──────────────────────────────────────

/// "Fixed" must not mean "`--plan-file` never applies anything".
#[test]
fn a_listed_operator_still_applies_the_plan() {
    let sb = Sandbox::new("allowed", &["alice"]);
    assert_saved(&sb.save_plan(), &sb.plan_file());
    sb.reset_targets();

    let out = sb.apply_from_plan(Some("alice"));
    assert!(
        out.status.success(),
        "an AUTHORIZED operator must still be able to execute a saved plan: {}",
        stderr(&out)
    );
    for p in sb.managed() {
        assert!(p.exists(), "{} was not created", p.display());
    }
}

/// A config that lists no operators restricts nobody — on this path exactly as
/// on the ordinary one.
#[test]
fn an_unrestricted_config_is_unaffected() {
    let sb = Sandbox::new("open", &[]);
    assert_saved(&sb.save_plan(), &sb.plan_file());
    sb.reset_targets();

    let out = sb.apply_from_plan(Some("anyone-at-all"));
    assert!(
        out.status.success(),
        "an empty allowed_operators must gate nothing: {}",
        stderr(&out)
    );
}

// ── The decision about `plan --out`, pinned ─────────────────────────────────

/// forjar#370 asks whether PRODUCING the artifact should be authorized too.
/// Decided: no, and this test is the decision rather than an accident.
///
/// A plan file is unauthenticated data — any user can write one in a text
/// editor — so gating `plan --out` stops nothing an attacker cannot route
/// around, while breaking a real contract: `plan` is one of the nine verbs
/// published `readOnlyHint: true` (`src/verb/registry.rs`), and
/// `allowed_operators` is an apply-time gate. The check that is load-bearing is
/// the one at execution, and `a_plan_file_apply_refuses_an_unlisted_operator`
/// above holds it there.
#[test]
fn producing_a_plan_file_is_deliberately_not_gated() {
    let sb = Sandbox::new("produce", &["alice"]);
    let out = sb.save_plan();
    assert!(
        out.status.success() && sb.plan_file().exists(),
        "reading the config into a plan artifact is a READ; gating it would make \
         `plan` refuse for an unauthorized reader: {}",
        stderr(&out)
    );
    assert!(
        sb.nothing_was_written(),
        "planning must remain a read — it changed the machine"
    );
}
