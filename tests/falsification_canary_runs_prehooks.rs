//! forjar#435: `apply --canary-machine` returned from `apply_early_exits` above
//! `apply_pre_checks`, so `--pre-script` and friends parsed, the run succeeded
//! and the knobs did nothing. The falsifier: a `--pre-script` that writes a
//! marker must either run (marker exists) or refuse (nothing converged).

#[path = "common/canary_authz.rs"]
mod harness;
use harness::*;

fn script(sb: &Sandbox, name: &str, body: &str) -> std::path::PathBuf {
    let p = sb.dir.join(name);
    std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).expect("script");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    }
    p
}

#[test]
fn canary_apply_runs_the_pre_script() {
    let sb = Sandbox::fleet("canary-prescript-runs");
    let marker = sb.dir.join("pre-ran.txt");
    let s = script(&sb, "pre.sh", &format!("echo ran > {}", marker.display()));
    let out = sb.canary(
        Some("alice"),
        &["--yes", "--pre-script", s.to_str().unwrap()],
    );
    assert!(
        out.status.success(),
        "canary apply failed: {}",
        stderr(&out)
    );
    assert!(
        marker.exists(),
        "`--canary-machine --pre-script` converged without running the script"
    );
    assert!(sb.canary_file().exists(), "the canary did not converge");
}

#[test]
fn a_failing_pre_script_stops_the_canary_before_anything_converges() {
    let sb = Sandbox::fleet("canary-prescript-gates");
    let s = script(&sb, "pre.sh", "exit 1");
    let out = sb.canary(
        Some("alice"),
        &["--yes", "--pre-script", s.to_str().unwrap()],
    );
    assert!(
        !out.status.success(),
        "a failing --pre-script did not stop the canary apply: {}",
        stdout(&out)
    );
    assert!(
        sb.nothing_was_written(),
        "files were written past a failed pre-script"
    );
}

#[test]
fn the_pre_script_runs_once_for_the_rollout_not_per_machine() {
    let sb = Sandbox::fleet("canary-prescript-once");
    let marker = sb.dir.join("count.txt");
    let s = script(&sb, "pre.sh", &format!("echo x >> {}", marker.display()));
    let out = sb.canary(
        Some("alice"),
        &["--yes", "--pre-script", s.to_str().unwrap()],
    );
    assert!(
        out.status.success(),
        "canary apply failed: {}",
        stderr(&out)
    );
    let n = std::fs::read_to_string(&marker)
        .unwrap_or_default()
        .lines()
        .count();
    assert_eq!(n, 1, "pre-script ran {n} times for one invocation");
}
