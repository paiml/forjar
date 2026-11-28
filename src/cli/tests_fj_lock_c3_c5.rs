//! FJ-LOCK-001 C3/C4/C5 (#923, #924, #925), through the real `forjar apply`
//! dispatch: one hung machine does not block another, and the new flags parse.

use super::commands::*;
use super::dispatch_apply_b::dispatch_apply_cmd;
use std::path::{Path, PathBuf};

#[derive(clap::Parser)]
#[command(name = "forjar")]
struct TestCli {
    #[command(subcommand)]
    cmd: Commands,
}

fn parse(extra: &[&str]) -> Commands {
    let mut argv = vec!["forjar", "apply"];
    argv.extend_from_slice(extra);
    <TestCli as clap::Parser>::try_parse_from(argv)
        .expect("clap parse")
        .cmd
}

fn run_apply(extra: &[&str]) -> Result<(), String> {
    let argv: Vec<String> = extra.iter().map(|s| s.to_string()).collect();
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let argv: Vec<&str> = argv.iter().map(String::as_str).collect();
            dispatch_apply_cmd(parse(&argv), false)
        })
        .expect("spawn")
        .join()
        .expect("join")
}

/// Two localhost machines, each with one file resource under `dir`.
fn two_machine_config(dir: &Path) -> PathBuf {
    let yaml = format!(
        "version: \"1.0\"\nname: two\nmachines:\n  intel:\n    hostname: intel\n    addr: 127.0.0.1\n  lambda:\n    hostname: lambda\n    addr: 127.0.0.1\nresources:\n  fi:\n    type: file\n    machine: intel\n    path: {i}\n    content: i\n  fl:\n    type: file\n    machine: lambda\n    path: {l}\n    content: l\n",
        i = dir.join("intel.txt").display(),
        l = dir.join("lambda.txt").display()
    );
    let p = dir.join("forjar.yaml");
    std::fs::write(&p, yaml).unwrap();
    p
}

/// A lock a live process other than us (pid 1) holds on `machine`.
fn hang_machine(state_dir: &Path, machine: &str, id: &str) -> PathBuf {
    let lock = state_dir.join(machine).join(".forjar.lock");
    std::fs::create_dir_all(lock.parent().unwrap()).unwrap();
    let info = crate::core::state::LockInfo {
        id: id.into(),
        pid: 1,
        starttime: String::new(),
        command: "forjar apply -m intel".into(),
        ..crate::core::state::LockInfo::current()
    };
    std::fs::write(&lock, info.render()).unwrap();
    lock
}

fn s(p: &Path) -> String {
    p.display().to_string()
}

/// The #918 incident, end to end: a hung apply on intel (lock held by a live
/// PID) must not stop an apply that only declares lambda; an apply that
/// declares intel is still refused, and the refusal names the holder.
#[test]
fn falsify_c4_hung_intel_apply_does_not_block_lambda() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = two_machine_config(dir.path());
    let sd = dir.path().join("state");
    let lock = hang_machine(&sd, "intel", "hung-intel-id");

    let ok = run_apply(&["-f", &s(&cfg), "--state-dir", &s(&sd), "-m", "lambda", "--yes"]);
    assert!(ok.is_ok(), "lambda blocked by a hung intel apply: {ok:?}");
    assert!(dir.path().join("lambda.txt").exists(), "lambda not converged");
    assert!(!dir.path().join("intel.txt").exists(), "intel was touched");

    let refused = run_apply(&["-f", &s(&cfg), "--state-dir", &s(&sd), "-m", "intel", "--yes"])
        .unwrap_err();
    assert!(refused.contains("locked by PID 1"), "{refused}");
    assert!(refused.contains("id=hung-intel-id"), "LockInfo missing: {refused}");
    assert!(lock.exists(), "the refused apply removed the live lock");
    assert!(!dir.path().join("intel.txt").exists());
}

/// `--force-unlock <ID>` on a live local holder is refused through the CLI.
#[test]
fn falsify_c3_cli_force_unlock_id_refuses_live_holder() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = two_machine_config(dir.path());
    let sd = dir.path().join("state");
    let lock = hang_machine(&sd, "intel", "live-id");
    let err = run_apply(&[
        "-f", &s(&cfg), "--state-dir", &s(&sd), "-m", "intel", "--yes", "--force-unlock", "live-id",
    ])
    .unwrap_err();
    assert!(err.contains("holder is alive"), "{err}");
    assert!(lock.exists());
    // The bare flag is refused for a live holder too.
    let bare = run_apply(&[
        "-f", &s(&cfg), "--state-dir", &s(&sd), "-m", "intel", "--yes", "--force-unlock",
    ])
    .unwrap_err();
    assert!(bare.contains("holder is alive"), "{bare}");
    assert!(lock.exists());
}

#[test]
fn c3_c5_flags_parse() {
    let got = std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(|| {
            let pick = |extra: &[&str]| match parse(extra) {
                Commands::Apply(a) => (a.lock.force_unlock, a.lock.lock_timeout),
                _ => panic!("not apply"),
            };
            [
                pick(&["-f", "x.yaml", "--force-unlock", "abc", "--lock-timeout", "5s"]),
                pick(&["-f", "x.yaml", "--force-unlock"]),
                pick(&["-f", "x.yaml"]),
            ]
        })
        .unwrap()
        .join()
        .unwrap();
    assert_eq!(got[0], (Some("abc".to_string()), Some("5s".to_string())));
    assert_eq!(got[1], (Some(String::new()), None), "bare flag");
    assert_eq!(got[2], (None, None));
}

#[test]
fn c5_invalid_lock_timeout_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = two_machine_config(dir.path());
    let sd = dir.path().join("state");
    let err = run_apply(&["-f", &s(&cfg), "--state-dir", &s(&sd), "--yes", "--lock-timeout", "soon"])
        .unwrap_err();
    assert!(err.contains("invalid --lock-timeout"), "{err}");
}

/// FJ-LOCK-001 C4 (#1307): a live holder of the SINGLE state-dir key does not
/// refuse an apply to any machine. The preflight probes each machine's lock.
#[test]
fn falsify_c4_single_state_dir_key_does_not_block_machine_applies() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = two_machine_config(dir.path());
    let sd = dir.path().join("state");
    std::fs::create_dir_all(&sd).unwrap();
    let root = sd.join(".forjar.lock");
    let info = crate::core::state::LockInfo {
        id: "root-key-id".into(),
        pid: 1,
        starttime: String::new(),
        command: "forjar apply".into(),
        ..crate::core::state::LockInfo::current()
    };
    std::fs::write(&root, info.render()).unwrap();

    let lambda = run_apply(&["-f", &s(&cfg), "--state-dir", &s(&sd), "-m", "lambda", "--yes"]);
    assert!(lambda.is_ok(), "single state-dir key refused lambda: {lambda:?}");
    assert!(dir.path().join("lambda.txt").exists(), "lambda not converged");

    let all = run_apply(&["-f", &s(&cfg), "--state-dir", &s(&sd), "--yes"]);
    assert!(all.is_ok(), "single state-dir key refused the full apply: {all:?}");
    assert!(root.exists(), "the root key was removed by an apply");
}

/// FJ-LOCK-001 C4 (#1307): a selector-scoped apply takes only the locks of the
/// machines it converges, so a live foreign holder on an UNRELATED machine must
/// not refuse it. Reverting the `lock_targets` probe in `apply_state_gates`
/// (back to every declared machine) fails this test.
#[test]
fn falsify_c4_selector_scoped_apply_ignores_unrelated_locked_machine() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = two_machine_config(dir.path());
    let sd = dir.path().join("state");
    let lock = hang_machine(&sd, "intel", "hung-intel-id");

    let only = run_apply(&["-f", &s(&cfg), "--state-dir", &s(&sd), "--only-machine", "lambda", "--yes"]);
    assert!(only.is_ok(), "--only-machine lambda refused by intel's lock: {only:?}");
    assert!(dir.path().join("lambda.txt").exists(), "lambda not converged");

    let excl = run_apply(&["-f", &s(&cfg), "--state-dir", &s(&sd), "--exclude-machine", "intel", "--yes"]);
    assert!(excl.is_ok(), "--exclude-machine intel refused by intel's lock: {excl:?}");
    assert!(lock.exists(), "the live lock was removed");
}

/// FJ-LOCK-001 C4 (#1307): `--dry-run` and `--refresh-only` take no lock, so
/// their refusal of a live holder comes only from the preflight probe
/// (`machine_lock_refusal`). Replacing it with `Ok(None)` fails this test.
#[test]
fn falsify_c4_dry_run_and_refresh_refuse_a_held_machine() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = two_machine_config(dir.path());
    let sd = dir.path().join("state");
    let lock = hang_machine(&sd, "intel", "hung-intel-id");

    let dry = run_apply(&["-f", &s(&cfg), "--state-dir", &s(&sd), "-m", "intel", "--dry-run", "--yes"])
        .unwrap_err();
    assert!(dry.contains("locked by PID 1"), "dry-run not refused: {dry}");

    let refresh = run_apply(&["-f", &s(&cfg), "--state-dir", &s(&sd), "-m", "intel", "--refresh-only", "--yes"])
        .unwrap_err();
    assert!(refresh.contains("locked by PID 1"), "refresh-only not refused: {refresh}");

    let dry_lambda = run_apply(&["-f", &s(&cfg), "--state-dir", &s(&sd), "-m", "lambda", "--dry-run", "--yes"]);
    assert!(dry_lambda.is_ok(), "dry-run of a free machine refused: {dry_lambda:?}");
    assert!(lock.exists());
    assert!(!dir.path().join("intel.txt").exists(), "intel was touched");
}

/// FJ-LOCK-001 C4 (#1307): an implicit `localhost` (no `machines:` block) takes
/// `<sd>/localhost/.forjar.lock`, so a dry run must refuse a live holder of it.
#[test]
fn falsify_c4_implicit_localhost_dry_run_refuses_a_held_lock() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: local\nresources:\n  f:\n    type: file\n    path: {p}\n    content: x\n",
            p = dir.path().join("f.txt").display()
        ),
    )
    .unwrap();
    let sd = dir.path().join("state");
    let lock = hang_machine(&sd, "localhost", "hung-local-id");
    let dry = run_apply(&["-f", &s(&cfg), "--state-dir", &s(&sd), "--dry-run", "--yes"]).unwrap_err();
    assert!(dry.contains("locked by PID 1"), "implicit localhost not refused: {dry}");
    assert!(lock.exists());
}

/// FJ-LOCK-001 C4 (#1307): `--refresh-only` with a selector re-seals only the
/// machines the selection names. `intel` is excluded and held by a live foreign
/// PID, so its lock files must not be rewritten. A selector-excluded resource
/// is pruned from the config, so a refresh of intel writes back identical bytes;
/// the observable is the file's mtime. Reverting the load in
/// `cmd_refresh_only_with` to every `-m`-matching machine re-saves intel's lock,
/// which this test catches.
#[test]
fn falsify_c4_refresh_only_with_selector_leaves_excluded_held_lock_alone() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = two_machine_config(dir.path());
    let sd = dir.path().join("state");
    let seeded = run_apply(&["-f", &s(&cfg), "--state-dir", &s(&sd), "-m", "intel", "--yes"]);
    assert!(seeded.is_ok(), "seed apply for intel failed: {seeded:?}");
    let yaml = sd.join("intel").join("state.lock.yaml");
    let b3 = sd.join("intel").join("state.lock.yaml.b3");
    let mtime = |p: &Path| std::fs::metadata(p).unwrap().modified().unwrap();
    let (yaml_before, b3_before) = (mtime(&yaml), mtime(&b3));
    let lock = hang_machine(&sd, "intel", "hung-intel-id");
    // Let a rewrite land on a later timestamp tick than the seed apply.
    std::thread::sleep(std::time::Duration::from_millis(50));

    let r = run_apply(&[
        "-f",
        &s(&cfg),
        "--state-dir",
        &s(&sd),
        "--refresh-only",
        "--exclude-machine",
        "intel",
        "--yes",
    ]);
    assert!(r.is_ok(), "refresh-only --exclude-machine intel refused: {r:?}");
    assert_eq!(mtime(&yaml), yaml_before, "excluded intel state.lock.yaml was rewritten");
    assert_eq!(mtime(&b3), b3_before, "excluded intel state.lock.yaml.b3 was rewritten");
    assert!(lock.exists(), "the live lock was removed");
}

/// FJ-LOCK-001 C4 (#1307): `apply --plan-file --dry-run` of a lambda-only plan
/// probes the plan's machines, not every machine the config declares. `intel`
/// is outside the plan and held by a live foreign PID, so the dry run must pass.
/// Reverting `apply_state_gates` to probe `lock_targets(config, None)` refuses
/// it on intel's lock, which this test catches.
#[test]
fn falsify_c4_plan_file_dry_run_ignores_a_machine_outside_the_plan() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = two_machine_config(dir.path());
    let sd = dir.path().join("state");
    // A plan needs a state dir to be authoritative; seed intel so the plan is
    // not "no state", leaving lambda as the only pending create.
    let seeded = run_apply(&["-f", &s(&cfg), "--state-dir", &s(&sd), "-m", "intel", "--yes"]);
    assert!(seeded.is_ok(), "seed apply for intel failed: {seeded:?}");
    let plan = dir.path().join("lambda-plan.json");
    super::plan::cmd_plan(
        &cfg,
        &sd,
        Some("lambda"),
        None,
        None,
        false,
        false,
        None,
        None,
        None,
        false,
        None,
        false,
        &[],
        Some(&plan),
        false,
        None,
    )
    .expect("plan for lambda");
    let lock = hang_machine(&sd, "intel", "hung-intel-id");

    let dry = run_apply(&[
        "-f",
        &s(&cfg),
        "--state-dir",
        &s(&sd),
        "--plan-file",
        &s(&plan),
        "--dry-run",
        "--yes",
    ]);
    assert!(dry.is_ok(), "lambda-only plan dry-run refused by intel's lock: {dry:?}");
    assert!(lock.exists(), "the live lock was removed");
    assert!(!dir.path().join("lambda.txt").exists(), "--dry-run wrote lambda");
}
