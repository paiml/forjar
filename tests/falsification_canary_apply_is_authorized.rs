//! forjar#374: `apply --canary-machine` converged the WHOLE FLEET past the
//! operator gate, and did it with `--yes` nobody typed.
//!
//! `src/cli/dispatch_apply_b.rs` ran `check_operator_auth` as the first line of
//! `apply_execute` — the LAST stage. `apply_early_exits` returns above it for
//! `--canary-machine`, and `apply_mode_exits` returns above it for
//! `--refresh-only`, so both reached real convergence with the gate unread.
//! Measured on 1.24.0 (f0cbf635) with two machines, both `allowed_operators:
//! [alice]`:
//!
//! ```text
//!   forjar apply --operator mallory --yes                  -> not authorized  EXIT=1
//!   forjar apply --canary-machine sandbox --operator mallory
//!                    -> "Canary: applying to 'sandbox'" ... "Fleet deploy
//!                       complete (2 machines)."                       EXIT=0
//!                       canary.txt CREATED, prod.txt CREATED
//!   forjar apply --refresh-only --operator mallory
//!                    -> "Refresh complete: N resources queried"       EXIT=0
//! ```
//!
//! Two distinct defects, and the second needs no misconfiguration at all:
//!
//! 1. the gate was positioned below four early exits (#370 fixed exactly one of
//!    them, `--plan-file`, at its own call site — this fixes the position); and
//! 2. `cmd_apply_canary_machine` hard-coded `yes = true` into both legs, so a
//!    flag whose whole promise is "one machine first" rolled the remaining
//!    fleet out unconfirmed for AUTHORIZED operators too. No `--yes` appears on
//!    the command line above.
//!
//! Everything below drives the shipped binary. An in-process test of
//! `cmd_apply_canary_machine` misses defect 1 entirely: the hole is in which
//! dispatcher branch reaches the gate, not in the gate.

use std::path::PathBuf;
use std::process::{Command, Output};

const FORJAR: &str = env!("CARGO_BIN_EXE_forjar");

struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    /// `machines` is `(machine, allowed_operators)`; an empty operator list
    /// restricts nobody on that machine.
    fn new(name: &str, machines: &[(&str, &[&str])]) -> Self {
        // Per-process: two concurrent runs of this suite on one box would
        // otherwise share `forjar-374-<name>`, and each `Drop` deletes the
        // other's fixture out from under it mid-run.
        let dir = std::env::temp_dir().join(format!("forjar-374-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("sandbox");
        let me = Self { dir };
        me.write_config(machines);
        me
    }

    /// The default two-machine fleet: both restricted to `alice`.
    fn fleet(name: &str) -> Self {
        Self::new(name, &[("sandbox", &["alice"]), ("prod", &["alice"])])
    }

    fn cfg(&self) -> PathBuf {
        self.dir.join("forjar.yaml")
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    fn canary_file(&self) -> PathBuf {
        self.dir.join("canary.txt")
    }

    fn prod_file(&self) -> PathBuf {
        self.dir.join("prod.txt")
    }

    fn write_config(&self, machines: &[(&str, &[&str])]) {
        let d = self.dir.display();
        let mut yaml = String::from("version: \"1.0\"\nname: canary-authz\nmachines:\n");
        for (m, ops) in machines {
            yaml.push_str(&format!("  {m}:\n    hostname: {m}\n    addr: 127.0.0.1\n"));
            if !ops.is_empty() {
                yaml.push_str("    allowed_operators:\n");
                for o in *ops {
                    yaml.push_str(&format!("      - {o}\n"));
                }
            }
        }
        yaml.push_str(&format!(
            "resources:\n  \
             canary_file:\n    type: file\n    machine: sandbox\n    \
             path: {d}/canary.txt\n    content: \"canary\"\n  \
             prod_file:\n    type: file\n    machine: prod\n    \
             path: {d}/prod.txt\n    content: \"prod\"\n"
        ));
        std::fs::write(self.cfg(), yaml).expect("config");
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(FORJAR)
            .args(args)
            .current_dir(&self.dir)
            .output()
            .expect("run forjar")
    }

    /// `forjar apply -f <cfg> --state-dir <sd> <extra…>`, plus `--operator` when given.
    fn apply(&self, operator: Option<&str>, extra: &[&str]) -> Output {
        let cfg = self.cfg();
        let sd = self.state();
        let mut args = vec![
            "apply",
            "-f",
            cfg.to_str().unwrap(),
            "--state-dir",
            sd.to_str().unwrap(),
        ];
        args.extend_from_slice(extra);
        if let Some(op) = operator {
            args.push("--operator");
            args.push(op);
        }
        self.run(&args)
    }

    fn canary(&self, operator: Option<&str>, extra: &[&str]) -> Output {
        let mut args = vec!["--canary-machine", "sandbox"];
        args.extend_from_slice(extra);
        self.apply(operator, &args)
    }

    fn nothing_was_written(&self) -> bool {
        !self.canary_file().exists() && !self.prod_file().exists()
    }

    fn reset_targets(&self) {
        let _ = std::fs::remove_file(self.canary_file());
        let _ = std::fs::remove_file(self.prod_file());
        let _ = std::fs::remove_dir_all(self.state());
    }

    /// Every file under the state dir, with its modification time. `--refresh-only`
    /// calls `state::save_lock` unconditionally, so a refusal must leave this equal.
    fn state_fingerprint(&self) -> Vec<(PathBuf, std::time::SystemTime, u64)> {
        let mut out = Vec::new();
        let Ok(rd) = std::fs::read_dir(self.state()) else {
            return out;
        };
        for e in rd.flatten() {
            if let Ok(md) = e.metadata() {
                if md.is_file() {
                    out.push((e.path(), md.modified().expect("mtime"), md.len()));
                }
            }
        }
        out.sort();
        out
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

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn refused(o: &Output, what: &str) {
    assert!(
        !o.status.success(),
        "{what} exited {:?}; the operator gate was never reached.\nstdout: {}",
        o.status.code(),
        stdout(o)
    );
    assert!(
        stderr(o).contains("not authorized"),
        "{what} must give the SAME refusal the ordinary path gives.\nstderr: {}",
        stderr(o)
    );
}

// ── The control: the gate exists on the ordinary path ───────────────────────

/// Without this, "the canary path refuses" could mean the fixture is broken.
#[test]
fn control_the_ordinary_apply_refuses_an_unlisted_operator() {
    let sb = Sandbox::fleet("control");
    let out = sb.apply(Some("mallory"), &["--yes"]);
    refused(&out, "`apply --yes --operator mallory`");
    assert!(sb.nothing_was_written(), "a refused apply wrote files");
}

// ── Defect 1: the gate sat below the early exits ────────────────────────────
//
// Every test in this section passes `--yes` explicitly, so that defect 2's fix
// cannot mask defect 1's: without it, the restored confirmation prompt aborts
// the canary leg on EOF and the "nothing was written" assertions below go green
// against an authorization gate that is still never reached.

#[test]
fn canary_apply_refuses_an_unlisted_operator() {
    let sb = Sandbox::fleet("canary-refuses");
    let out = sb.canary(Some("mallory"), &["--yes"]);
    refused(
        &out,
        "`apply --canary-machine sandbox --operator mallory --yes`",
    );
}

#[test]
fn canary_apply_writes_nothing_when_refused() {
    let sb = Sandbox::fleet("canary-writes");
    let _ = sb.canary(Some("mallory"), &["--yes"]);
    assert!(
        !sb.canary_file().exists(),
        "the canary machine converged for an operator the config does not list"
    );
}

/// The blast radius. This fails even against a fix that only gates the canary
/// machine itself: `cmd_apply_canary_machine` loops `cmd_apply` over every
/// OTHER machine in the config.
#[test]
fn canary_apply_does_not_converge_the_rest_of_the_fleet() {
    let sb = Sandbox::fleet("canary-fleet");
    let _ = sb.canary(Some("mallory"), &["--yes"]);
    assert!(
        !sb.prod_file().exists(),
        "an unlisted operator converged 'prod' — a machine they never named — \
         through a flag documented as applying to ONE machine first"
    );
}

/// Defect 2, and it is not fixed by the gate. Run as the AUTHORIZED operator,
/// with no `--yes` anywhere on the command line: the fleet leg must not
/// converge unconfirmed. stdin is closed here exactly as it is in CI.
#[test]
fn canary_apply_does_not_imply_yes() {
    let sb = Sandbox::fleet("canary-yes");
    let out = sb.canary(Some("alice"), &[]);
    assert!(
        !sb.prod_file().exists(),
        "`--canary-machine` rolled the remaining fleet out with a `--yes` the \
         operator never typed (apply_variants.rs hard-coded it into both legs).\n\
         exit {:?}\nstdout: {}",
        out.status.code(),
        stdout(&out)
    );
}

/// `--refresh-only` is NOT a read: it resolves data sources, shells out to every
/// managed host and calls `state::save_lock` unconditionally per machine.
#[test]
fn refresh_only_refuses_an_unlisted_operator() {
    let sb = Sandbox::fleet("refresh");
    let converge = sb.apply(Some("alice"), &["--yes"]);
    assert!(
        converge.status.success() && !sb.state_fingerprint().is_empty(),
        "fixture: the authorized apply had to produce lock files first: {}",
        stderr(&converge)
    );
    let before = sb.state_fingerprint();

    let out = sb.apply(Some("mallory"), &["--refresh-only"]);
    refused(&out, "`apply --refresh-only --operator mallory`");
    assert_eq!(
        before,
        sb.state_fingerprint(),
        "a refused refresh rewrote the lock files anyway"
    );
}

/// The pre-hook window: `apply_pre_checks` ran the user's `--pre-script` and
/// only then refused. The refusal was honest about the apply and silent about
/// the script it had already executed.
#[test]
fn an_unauthorized_apply_does_not_run_the_pre_script() {
    let sb = Sandbox::fleet("pre-script");
    let script = sb.dir.join("pre.sh");
    let sentinel = sb.dir.join("pre-script-ran");
    std::fs::write(
        &script,
        format!("#!/bin/bash\ntouch {}\n", sentinel.display()),
    )
    .expect("script");

    let out = sb.apply(Some("mallory"), &["--yes", "--pre-script", "pre.sh"]);
    refused(&out, "`apply --pre-script --operator mallory`");
    assert!(
        !sentinel.exists(),
        "the apply was refused AFTER running the operator's pre-script"
    );
}

/// The OTHER door of the same pre-hook window, and the read-mode exemption is
/// what opens it. `--check` exits from `apply_mode_exits`, which sits BELOW
/// `apply_pre_checks` — so an exemption written as "is `--check` set?" skips the
/// gate, `apply_pre_checks` runs the operator's `--pre-script` to completion,
/// and the check results print with no refusal anywhere, because the read modes
/// are deliberately ungated. Measured against the first cut of this fix:
/// `apply --check --pre-script pre.sh --operator mallory` created the sentinel
/// and exited on "2 check(s) failed". A read mode carrying a hook is not a read.
#[test]
fn a_read_mode_does_not_launder_the_pre_script_hook() {
    let sb = Sandbox::fleet("read-pre-script");
    let script = sb.dir.join("pre.sh");
    let sentinel = sb.dir.join("pre-script-ran");
    std::fs::write(
        &script,
        format!("#!/bin/bash\ntouch {}\n", sentinel.display()),
    )
    .expect("script");

    let out = sb.apply(Some("mallory"), &["--check", "--pre-script", "pre.sh"]);
    assert!(
        !sentinel.exists(),
        "`apply --check --pre-script` ran an unlisted operator's script: the \
         read-mode exemption skipped the gate and `apply_pre_checks` runs \
         BELOW it.\nexit {:?}\nstdout: {}",
        out.status.code(),
        stdout(&out)
    );
    refused(&out, "`apply --check --pre-script --operator mallory`");
}

/// The over-correction guard for the test above: carrying a hook must gate the
/// read, not forbid it. alice is listed, so her hook runs and her check runs.
#[test]
fn a_listed_operator_still_gets_a_read_with_a_hook() {
    let sb = Sandbox::fleet("read-pre-script-ok");
    let script = sb.dir.join("pre.sh");
    let sentinel = sb.dir.join("pre-script-ran");
    std::fs::write(
        &script,
        format!("#!/bin/bash\ntouch {}\n", sentinel.display()),
    )
    .expect("script");

    let out = sb.apply(Some("alice"), &["--check", "--pre-script", "pre.sh"]);
    assert!(
        !stderr(&out).contains("not authorized"),
        "alice is listed on every machine: {}",
        stderr(&out)
    );
    assert!(
        sentinel.exists(),
        "an AUTHORIZED operator's --pre-script must still run: {}",
        stderr(&out)
    );
    assert!(sb.nothing_was_written(), "`--check` converged something");
}

/// Precedence, not a set. `apply_early_exits` runs BEFORE `apply_mode_exits`,
/// so `--canary-machine --check` converges the fleet and prints no check
/// results at all. A read-mode exemption written as "is `--check` set?" would
/// therefore reopen the exact hole it was added beside.
#[test]
fn a_read_flag_does_not_launder_a_canary_rollout() {
    let sb = Sandbox::fleet("launder");
    let out = sb.canary(Some("mallory"), &["--yes", "--check"]);
    refused(
        &out,
        "`apply --canary-machine sandbox --check --operator mallory --yes`",
    );
    assert!(
        sb.nothing_was_written(),
        "a read flag alongside --canary-machine converged the fleet"
    );
}

// ── The guards against over-correcting ──────────────────────────────────────

/// "Fixed" must not mean "`--canary-machine` never applies anything".
#[test]
fn a_listed_operator_still_gets_a_canary_rollout() {
    let sb = Sandbox::fleet("allowed");
    sb.reset_targets();
    let out = sb.canary(Some("alice"), &["--yes"]);
    assert!(
        out.status.success(),
        "an AUTHORIZED operator must still get a canary rollout: {}",
        stderr(&out)
    );
    assert!(sb.canary_file().exists(), "canary machine did not converge");
    assert!(sb.prod_file().exists(), "fleet leg did not converge");
}

/// An empty `allowed_operators` restricts nobody — on this path as on any other.
#[test]
fn an_unrestricted_config_is_unaffected() {
    let sb = Sandbox::new("open", &[("sandbox", &[]), ("prod", &[])]);
    let out = sb.canary(Some("anyone-at-all"), &["--yes"]);
    assert!(
        out.status.success(),
        "an empty allowed_operators must gate nothing: {}",
        stderr(&out)
    );
    assert!(sb.canary_file().exists() && sb.prod_file().exists());
}

// ── The read/execute line, pinned as a decision rather than an accident ─────

/// forjar#374 asks whether the hoist should gate the read-only apply modes too.
/// Decided: no, and this test is the decision.
///
/// `allowed_operators` is an apply-time gate — #370 pinned the same line when
/// it left `plan --out` ungated. `--check`, `--diff-only`, `--output-scripts`
/// and the `--dry-run-{graph,cost,verbose}` family change nothing and print
/// exactly what the ungated `forjar check` / `plan` / `graph` verbs already
/// print to anyone, none of which accepts `--operator` at all. Gating them buys
/// no confidentiality and costs the refusal pinned below.
///
/// The assertion is the refusal STRING, not the exit code: `apply --check`
/// legitimately exits non-zero on an unconverged fixture ("N check(s) failed"),
/// so an exit-code assertion here would be vacuous in both directions.
#[test]
fn the_read_only_apply_modes_stay_ungated() {
    let sb = Sandbox::fleet("reads");
    let scripts = sb.dir.join("scripts");
    for mode in [
        vec!["--check"],
        vec!["--diff-only"],
        vec!["--dry-run-graph"],
        vec!["--dry-run-cost"],
        vec!["--dry-run-verbose"],
        vec!["--output-scripts", scripts.to_str().unwrap()],
    ] {
        let out = sb.apply(Some("mallory"), &mode);
        assert!(
            !stderr(&out).contains("not authorized"),
            "`apply {}` is a read and must not be gated — the same information \
             is available ungated from `forjar check` / `plan` / `graph`: {}",
            mode.join(" "),
            stderr(&out)
        );
        assert!(
            sb.nothing_was_written(),
            "`apply {}` converged something; it is not a read at all",
            mode.join(" ")
        );
    }
}

/// The regression the unscoped hoist would have caused, pinned.
///
/// `check_operator_auth` iterates EVERY machine in the config regardless of
/// `--machine`, so gating the read modes would turn this working read into
/// "not authorized for machine 'prod'" for an operator scoped to one machine.
#[test]
fn a_machine_scoped_operator_can_still_read_its_own_machine() {
    let sb = Sandbox::new("scoped", &[("sandbox", &["bob"]), ("prod", &["alice"])]);
    let out = sb.apply(Some("bob"), &["--machine", "sandbox", "--check"]);
    assert!(
        !stderr(&out).contains("not authorized"),
        "bob is listed on 'sandbox' and asked about 'sandbox' only: {}",
        stderr(&out)
    );
}
