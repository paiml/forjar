use super::*;
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};

// Execution-level: run the generated reaper against a temp tree. A dead
// target is reclaimed; one whose cwd a live process holds is not; and with a
// `timeout` that always times out, NOTHING is deleted.

fn script_for(root: &str, kind: ReclaimKind) -> String {
    let mut r = res();
    r.budget_reclaim = vec![ReclaimRule {
        name: "fixture".into(),
        roots: vec![root.to_string()],
        kind,
        min_idle_minutes: 1,
    }];
    // Trigger at 2% used, demand 99% free: always triggered, never met, so
    // every provably-dead idle candidate is a deletion.
    r.budget_high_watermark_pct = Some(2);
    r.budget_target_free_pct = Some(99);
    reaper_script(&r).expect("script")
}

fn mk_target(root: &std::path::Path, name: &str) -> std::path::PathBuf {
    let d = root.join(name);
    std::fs::create_dir_all(&d).unwrap();
    let m = d.join(".rustc_info.json");
    std::fs::write(&m, "{}").unwrap();
    let old = Command::new("touch")
        .args(["-d", "2 days ago"])
        .arg(&m)
        .status()
        .unwrap();
    assert!(old.success());
    d
}

/// forjar#669: the pass first asks whether the target is reachable at all. These
/// fixtures demand 99% free of a real filesystem, which no fixture tree can
/// reach, so a 4 KiB `df` stub sits first on PATH and keeps the target reachable;
/// what they exercise is liveness and the timeout fail-safe, not reachability.
/// Poll /proc until the child is really inside `dir` (cwd or exe): a fixed sleep raced
/// the exec on a loaded box (intel runs at load 500+).
fn wait_until_inside(child: &std::process::Child, dir: &std::path::Path) {
    for _ in 0..200 {
        for l in ["cwd", "exe"] {
            if let Ok(t) = std::fs::read_link(format!("/proc/{}/{l}", child.id())) {
                if t.starts_with(dir) {
                    return;
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("child never entered {}", dir.display());
}

fn run(script: &std::path::Path, status: &std::path::Path, path: &str) -> String {
    let dfdir = script.parent().unwrap().join("dfstub");
    std::fs::create_dir_all(&dfdir).unwrap();
    let df = dfdir.join("df");
    std::fs::write(
        &df,
        "#!/bin/sh\necho 'Filesystem 1024-blocks Used Available Capacity Mounted on'\necho '/dev/stub 4 1 1 81% /'\n",
    )
    .unwrap();
    std::fs::set_permissions(&df, std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = &format!("{}:{path}", dfdir.display());
    let out = Command::new("sh")
        .arg(script)
        .env("PATH", path)
        .env("FORJAR_BUDGET_EXECUTE", "1")
        .env("FORJAR_BUDGET_STATUS", status)
        .output()
        .expect("run reaper");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn reclaims_only_the_provably_dead_and_fails_safe_on_timeout() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("src");
    let dead = mk_target(&root, "dead");
    let live = mk_target(&root, "live");
    let script = tmp.path().join("reaper.sh");
    // Glob kind consults the /proc scan for liveness; cargo kind (case 3)
    // exercises the bounded marker find.
    let glob = format!("{}/*", root.display());
    std::fs::write(&script, script_for(&glob, ReclaimKind::Glob)).unwrap();
    let cargo_script = tmp.path().join("cargo-reaper.sh");
    let root_s = root.to_string_lossy().into_owned();
    std::fs::write(&cargo_script, script_for(&root_s, ReclaimKind::CargoTarget)).unwrap();
    let status = tmp.path().join("status.json");
    let sys_path = std::env::var("PATH").unwrap();

    // A live process holding a cwd inside `live`.
    let mut holder = Command::new("sh")
        .args(["-c", "cd \"$1\" && exec sleep 60", "sh"])
        .arg(&live)
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    wait_until_inside(&holder, &live);

    // 1. timeout(1) that always times out: nothing may be deleted.
    let stub = tmp.path().join("stub");
    std::fs::create_dir_all(&stub).unwrap();
    let fake = stub.join("timeout");
    std::fs::write(&fake, "#!/bin/sh\nexit 124\n").unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    let log = run(&script, &status, &format!("{}:{sys_path}", stub.display()));
    assert!(log.contains("NOT MEASURED"), "{log}");
    assert!(dead.exists(), "timed-out scan must delete nothing: {log}");
    assert!(live.exists(), "{log}");

    // 3. Only the marker find times out (the /proc scan is fine): the cargo
    //    rule must still offer and delete nothing.
    let sel = stub.join("timeout");
    std::fs::write(
        &sel,
        "#!/bin/sh\nfor a in \"$@\"; do [ \"$a\" = -mindepth ] && exit 124; done\nexec /usr/bin/timeout \"$@\"\n",
    )
    .unwrap();
    let log = run(
        &cargo_script,
        &status,
        &format!("{}:{sys_path}", stub.display()),
    );
    assert!(log.contains("find over"), "{log}");
    assert!(log.contains("NOT MEASURED"), "{log}");
    assert!(dead.exists(), "timed-out find must delete nothing: {log}");

    // 3. Only the marker find times out (the /proc scan is fine): the cargo
    //    rule must offer and delete nothing.
    std::fs::write(
        &fake,
        "#!/bin/sh\nfor a in \"$@\"; do [ \"$a\" = -mindepth ] && exit 124; done\nexec /usr/bin/timeout \"$@\"\n",
    )
    .unwrap();
    let log = run(
        &cargo_script,
        &status,
        &format!("{}:{sys_path}", stub.display()),
    );
    assert!(log.contains("find over"), "{log}");
    assert!(log.contains("NOT MEASURED"), "{log}");
    assert!(dead.exists(), "timed-out find must delete nothing: {log}");

    // 2. Real run: dead goes, live stays, and it completes fast.
    let t0 = std::time::Instant::now();
    let log = run(&script, &status, &sys_path);
    assert!(t0.elapsed().as_secs() < 30, "{log}");
    assert!(
        !dead.exists(),
        "provably dead target must be reclaimed: {log}"
    );
    assert!(
        live.exists(),
        "a target a live process holds must be kept: {log}"
    );

    holder.kill().ok();
    holder.wait().ok();
}

/// forjar#667: a `cargo_target` dir whose BINARY is running (exe inside the dir,
/// cwd elsewhere, nothing held open by cwd/fd) is in use and must survive; a
/// dormant sibling is still reclaimed.
#[test]
fn a_cargo_target_whose_binary_is_running_survives() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("src");
    let live = mk_target(&root, "live");
    let dead = mk_target(&root, "dead");
    let exe = live.join("served");
    std::fs::copy("/bin/sleep", &exe).unwrap();
    let script = tmp.path().join("cargo-reaper.sh");
    std::fs::write(
        &script,
        script_for(&root.to_string_lossy(), ReclaimKind::CargoTarget),
    )
    .unwrap();
    // cwd is NOT inside the dir: only /proc/<pid>/exe points into it.
    let mut holder = Command::new(&exe)
        .arg("60")
        .current_dir(tmp.path())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    wait_until_inside(&holder, &live);
    let log = run(
        &script,
        &tmp.path().join("status.json"),
        &std::env::var("PATH").unwrap(),
    );
    holder.kill().ok();
    holder.wait().ok();
    assert!(
        live.exists(),
        "a running binary's target must be kept: {log}"
    );
    assert!(!dead.exists(), "the dormant sibling must go: {log}");
}

/// forjar#667: the in-use check is part of the LOOP, so every `ReclaimKind`
/// gets it. The exhaustive match makes a new kind fail to compile here.
#[test]
fn every_reclaim_kind_runs_the_in_use_check_in_the_loop() {
    for kind in [
        ReclaimKind::Glob,
        ReclaimKind::CargoTarget,
        ReclaimKind::AbandonedWorktree,
        ReclaimKind::ClaudeScratchpad,
    ] {
        match kind {
            ReclaimKind::Glob
            | ReclaimKind::CargoTarget
            | ReclaimKind::AbandonedWorktree
            | ReclaimKind::ClaudeScratchpad => {}
        }
        let s = script_for("/tmp/x/*", kind);
        let loop_at = s.find("while IFS= read -r cand").expect("loop");
        let rm_at = s.find("rm -rf -- \"$cand\"").expect("rm");
        let body = &s[loop_at..rm_at];
        assert!(
            body.contains(r#"fb_in_use "$cand""#),
            "{kind:?}: no loop-level in-use check"
        );
    }
}
