//! Refs #368 — `apply --plan-file` must run the same preflight gates an
//! ordinary `apply` runs, and `apply --refresh-only` must not launder the one
//! gate that says no flag overrides it.
//!
//! `dispatch_apply_b::apply_mode_exits` returns for `--plan-file` (and for
//! `--refresh-only`) BEFORE `apply_execute`, and `cmd_apply` is the only
//! production caller of `apply_preflight::apply_pre_validate`. So the entire
//! preflight — the BLAKE3 state-integrity gate, the policy engine,
//! `policy.security_gate`, the `policy.pre_apply` hook, the
//! `--confirm-destructive` hard block and the FJ-286 confirmation prompt —
//! was unreachable through a plan file. Measured on v1.24.0:
//!
//! ```text
//!   apply --yes                    -> error: state integrity check failed …
//!                                     No apply flag overrides this check.
//!   apply --plan-file p.json --yes -> Plan applied: 1 converged, 1 unchanged
//!
//!   apply --yes                    -> error: policy violations block apply
//!   apply --plan-file p.json --yes -> Plan applied: 1 converged, 1 unchanged
//!
//!   apply --yes                    -> error: security gate blocks apply
//!   apply --plan-file p.json --yes -> Plan applied  (and the secret was WRITTEN)
//! ```
//!
//! And the same early-return shape one flag over:
//!
//! ```text
//!   apply --yes           -> error: state integrity check failed …
//!   apply --refresh-only  -> Refresh complete   (.b3 REWRITTEN over the tamper)
//!   apply --yes           -> Apply complete: 1 converged
//! ```
//!
//! A plan file is produced by the ungated `forjar plan --out` from the SAME
//! config that declares the policy, so this is the documented two-stage
//! plan/review/apply flow running with every gate off — not a forged artifact.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

fn combined(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

struct Fx {
    _dir: tempfile::TempDir,
    cfg: PathBuf,
    state: PathBuf,
    alpha: PathBuf,
    bravo: PathBuf,
    plan: PathBuf,
}

/// Two file resources on one machine, plus whatever top-level YAML the gate
/// under test needs. `alpha_absent` re-declares `alpha` as `state: absent`,
/// which is how a destroy is put into the reviewed delta.
fn write_cfg(fx: &Fx, alpha_content: &str, alpha_absent: bool, extra: &str) {
    let alpha_state = if alpha_absent { "absent" } else { "file" };
    let yaml = format!(
        "version: \"1.0\"\n\
         name: gateparity\n\
         machines:\n\
         \x20 box:\n\
         \x20   hostname: localhost\n\
         \x20   addr: 127.0.0.1\n\
         resources:\n\
         \x20 alpha:\n\
         \x20   type: file\n\
         \x20   machine: box\n\
         \x20   path: {}\n\
         \x20   state: {}\n\
         \x20   content: \"{}\"\n\
         \x20 bravo:\n\
         \x20   type: file\n\
         \x20   machine: box\n\
         \x20   path: {}\n\
         \x20   state: file\n\
         \x20   content: \"B\"\n\
         {}",
        fx.alpha.display(),
        alpha_state,
        alpha_content,
        fx.bravo.display(),
        extra
    );
    std::fs::write(&fx.cfg, yaml).expect("write config");
}

fn fixture() -> Fx {
    let dir = tempfile::tempdir().expect("tempdir");
    let fx = Fx {
        cfg: dir.path().join("forjar.yaml"),
        state: dir.path().join("state"),
        alpha: dir.path().join("alpha.txt"),
        bravo: dir.path().join("bravo.txt"),
        plan: dir.path().join("p.json"),
        _dir: dir,
    };
    write_cfg(&fx, "A", false, "");
    fx
}

impl Fx {
    fn plan_out(&self) -> std::process::Output {
        forjar()
            .args(["plan", "-f"])
            .arg(&self.cfg)
            .arg("--state-dir")
            .arg(&self.state)
            .arg("--out")
            .arg(&self.plan)
            .output()
            .expect("run plan")
    }

    fn save_plan(&self) {
        let o = self.plan_out();
        assert!(o.status.success(), "plan --out: {}", combined(&o));
    }

    fn apply(&self, extra: &[&str]) -> std::process::Output {
        forjar()
            .args(["apply", "-f"])
            .arg(&self.cfg)
            .arg("--state-dir")
            .arg(&self.state)
            .args(extra)
            .stdin(Stdio::null())
            .output()
            .expect("run apply")
    }

    fn apply_plan(&self, extra: &[&str]) -> std::process::Output {
        forjar()
            .args(["apply", "-f"])
            .arg(&self.cfg)
            .arg("--state-dir")
            .arg(&self.state)
            .arg("--plan-file")
            .arg(&self.plan)
            .args(extra)
            .stdin(Stdio::null())
            .output()
            .expect("run apply --plan-file")
    }

    fn converge(&self) {
        let o = self.apply(&["--yes"]);
        assert!(o.status.success(), "setup apply: {}", combined(&o));
    }

    fn lock(&self) -> PathBuf {
        self.state.join("box").join("state.lock.yaml")
    }

    fn sidecar(&self) -> PathBuf {
        self.state.join("box").join("state.lock.yaml.b3")
    }
}

fn corrupt_sidecar(p: &Path) {
    std::fs::write(p, "deadbeef").expect("write .b3");
}

fn tamper_lock_body(p: &Path) {
    let body = std::fs::read_to_string(p).expect("read lock");
    let tampered = body.replace("content_hash: ", "content_hash: X");
    assert_ne!(body, tampered, "the fixture lock must carry a content_hash");
    std::fs::write(p, tampered).expect("write lock");
}

// ── FALSIFY-FLAG-A18: the BLAKE3 gate, through a plan file ──────────────────

/// The sharpest of the set: the gate's own refusal text asserts "No apply flag
/// overrides this check", and `--plan-file` overrode it.
///
/// Only the `.b3` sidecar is corrupted, never the lock body —
/// `plan_seal::digest::state_leg` hashes the BODY, so the seal still verifies
/// and the plan loads. The gate that would have caught the tamper simply never
/// ran.
#[test]
fn plan_file_apply_refuses_a_tampered_state_sidecar() {
    let fx = fixture();
    fx.converge();
    write_cfg(&fx, "A", true, ""); // alpha now `state: absent` -> a destroy
    fx.save_plan();
    corrupt_sidecar(&fx.sidecar());

    let out = fx.apply_plan(&["--yes"]);
    let text = combined(&out);
    assert!(
        !out.status.success(),
        "a tampered state sidecar must refuse the apply: {text}"
    );
    assert!(
        text.contains("state integrity check failed"),
        "the refusal must name the integrity gate: {text}"
    );
    assert!(
        fx.alpha.exists(),
        "nothing may be destroyed once the gate has refused"
    );
}

// ── FALSIFY-FLAG-A19: --refresh-only must not launder the same gate ─────────

/// `--refresh-only` takes the same `apply_mode_exits` early return and writes
/// state through `state::save_lock` with no gate at all, so it re-seals a
/// tampered lock and the NEXT ordinary apply sails through.
#[test]
fn refresh_only_cannot_launder_a_tampered_lock() {
    let fx = fixture();
    fx.converge();
    tamper_lock_body(&fx.lock());

    let first = fx.apply(&["--yes"]);
    assert!(
        !first.status.success(),
        "a tampered lock body must refuse an ordinary apply: {}",
        combined(&first)
    );

    let before = std::fs::read(fx.sidecar()).expect("read .b3");
    let refresh = fx.apply(&["--refresh-only"]);
    let after = std::fs::read(fx.sidecar()).expect("read .b3");
    assert_eq!(
        before,
        after,
        "--refresh-only re-sealed a tampered lock: {}",
        combined(&refresh)
    );

    let again = fx.apply(&["--yes"]);
    assert!(
        !again.status.success(),
        "the integrity gate must STILL refuse after a refresh: {}",
        combined(&again)
    );
}

// ── The config-side gates ──────────────────────────────────────────────────

/// A blocking `require` policy declared in the very config the plan was written
/// from. Fresh state, so both resources are creates the plan names.
#[test]
fn plan_file_apply_runs_the_policy_engine() {
    let fx = fixture();
    write_cfg(
        &fx,
        "A",
        false,
        "policies:\n\
         \x20 - type: require\n\
         \x20   message: \"files must declare an owner\"\n\
         \x20   resource_type: file\n\
         \x20   field: owner\n",
    );
    fx.save_plan();

    let out = fx.apply_plan(&["--yes"]);
    let text = combined(&out);
    assert!(
        !out.status.success(),
        "a blocking policy must refuse a plan-file apply: {text}"
    );
    assert!(
        text.contains("policy violations block apply"),
        "the refusal must name the policy engine: {text}"
    );
    assert!(
        !fx.alpha.exists() && !fx.bravo.exists(),
        "nothing may be written once the policy engine has refused"
    );
}

/// `policy.security_gate` — the worst of the six, because the plan path did not
/// merely skip the check, it WROTE the secret the gate exists to stop.
#[test]
fn plan_file_apply_runs_the_security_gate() {
    let fx = fixture();
    write_cfg(
        &fx,
        "password=hunter2",
        false,
        "policy:\n\x20 security_gate: critical\n",
    );
    fx.save_plan();

    let out = fx.apply_plan(&["--yes"]);
    let text = combined(&out);
    assert!(
        !out.status.success(),
        "a critical security finding must refuse a plan-file apply: {text}"
    );
    assert!(
        text.contains("security gate blocks apply"),
        "the refusal must name the security gate: {text}"
    );
    assert!(
        !fx.alpha.exists(),
        "the secret must not be written once the gate has refused"
    );
}

/// The `policy.pre_apply` hook is not a refusal but a side effect the operator
/// declared: it ran on `apply --yes` and did not run on `apply --plan-file`.
#[test]
fn plan_file_apply_runs_the_pre_apply_hook() {
    let fx = fixture();
    let marker = fx.state.parent().expect("parent").join("HOOK_RAN");
    write_cfg(
        &fx,
        "A",
        false,
        &format!("policy:\n\x20 pre_apply: \"touch {}\"\n", marker.display()),
    );
    fx.save_plan();

    let out = fx.apply_plan(&["--yes"]);
    assert!(
        out.status.success(),
        "apply --plan-file: {}",
        combined(&out)
    );
    assert!(
        marker.exists(),
        "the declared pre_apply hook must have run before the plan converged"
    );
}

/// `--confirm-destructive` is a HARD BLOCK, not a prompt: it returns an error
/// without reading stdin. Through a plan file it blocked nothing.
#[test]
fn plan_file_apply_honours_confirm_destructive() {
    let fx = fixture();
    fx.converge();
    write_cfg(&fx, "A", true, "");
    fx.save_plan();

    let out = fx.apply_plan(&["--confirm-destructive"]);
    let text = combined(&out);
    assert!(
        !out.status.success(),
        "--confirm-destructive must block the destroy: {text}"
    );
    assert!(
        text.contains("blocked by --confirm-destructive"),
        "the refusal must name the flag the operator passed: {text}"
    );
    assert!(
        fx.alpha.exists(),
        "the blocked destroy must not have happened"
    );
}

/// FJ-286: without `--yes`, a run holding a destroy asks first. Through a plan
/// file it destroyed the file and never asked — with stdin at EOF, which is
/// what a pipeline looks like.
#[test]
fn plan_file_apply_asks_before_destroying() {
    let fx = fixture();
    fx.converge();
    write_cfg(&fx, "A", true, "");
    fx.save_plan();

    let out = fx.apply_plan(&[]);
    let text = combined(&out);
    assert!(
        !out.status.success(),
        "an unconfirmed destroy must not proceed: {text}"
    );
    assert!(
        fx.alpha.exists(),
        "the unconfirmed destroy must not have happened"
    );
}

// ── GREEN GUARDS ───────────────────────────────────────────────────────────

/// "Fixed" must not mean "`--plan-file` refuses everything". A gate-free config
/// with `--yes` still applies its reviewed delta.
#[test]
fn a_gate_free_plan_file_apply_still_applies() {
    let fx = fixture();
    fx.save_plan();

    let out = fx.apply_plan(&["--yes"]);
    assert!(
        out.status.success(),
        "apply --plan-file: {}",
        combined(&out)
    );
    assert_eq!(std::fs::read_to_string(&fx.alpha).expect("alpha"), "A");
    assert_eq!(std::fs::read_to_string(&fx.bravo).expect("bravo"), "B");
}

/// GREEN GUARD: the operator gate `--refresh-only` gained must honour the flag
/// that answers it.
///
/// Adding `check_operator_auth` to this mode is right — it writes state. Wiring
/// it as `check_operator_auth(file, None)` is not: `OperatorIdentity::resolve`
/// falls back to `$USER@$(hostname)` when the flag is dropped, so the gate
/// refuses the very operator it exists to admit. Measured before this
/// assertion was wired:
///
/// ```text
///   apply --yes --operator alice           -> Apply complete: 1 converged
///   apply --refresh-only --operator alice  -> error: operator
///       'noah@noah-Lambda-Vector' not authorized for machine 'box'   rc=1
/// ```
///
/// That is forjar#358's defect — a mode dropping what the operator typed —
/// one flag over from the one this file closes.
#[test]
fn refresh_only_honours_the_operator_flag() {
    let fx = fixture();
    let yaml = std::fs::read_to_string(&fx.cfg).expect("read cfg").replace(
        "\x20   addr: 127.0.0.1\n",
        "\x20   addr: 127.0.0.1\n\x20   allowed_operators:\n\x20     - alice\n",
    );
    std::fs::write(&fx.cfg, &yaml).expect("write cfg");
    assert!(
        yaml.contains("allowed_operators"),
        "the fixture must actually restrict the machine"
    );

    let converge = fx.apply(&["--yes", "--operator", "alice"]);
    assert!(
        converge.status.success(),
        "an authorized operator must be able to apply: {}",
        combined(&converge)
    );

    let out = fx.apply(&["--refresh-only", "--operator", "alice"]);
    let text = combined(&out);
    assert!(
        out.status.success(),
        "the authorized operator must be able to refresh too: {text}"
    );

    // And the gate is not inert: an operator the machine does not name is still
    // refused, so the assertion above is not passing because the check is off.
    let denied = fx.apply(&["--refresh-only", "--operator", "mallory"]);
    let denied_text = combined(&denied);
    assert!(
        !denied.status.success(),
        "an unauthorized operator must still be refused: {denied_text}"
    );
    assert!(
        denied_text.contains("not authorized"),
        "the refusal must name the operator gate: {denied_text}"
    );
}

/// And `--refresh-only` over an untampered state still refreshes.
#[test]
fn refresh_only_still_works_on_honest_state() {
    let fx = fixture();
    fx.converge();
    let out = fx.apply(&["--refresh-only"]);
    let text = combined(&out);
    assert!(out.status.success(), "apply --refresh-only: {text}");
    assert!(text.contains("Refresh complete"), "{text}");
}
