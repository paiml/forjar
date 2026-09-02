//! forjar#404 (CRUX audit E02): the pre-apply drift gate opened one FRESH SSH
//! handshake per locked resource, BEFORE anything started ControlMaster.
//!
//! WHAT WAS OBSERVABLY WRONG. `apply_machine` starts the ControlMaster
//! (`executor/machine.rs:78`), but the drift gate runs three frames earlier
//! (`apply_preflight.rs` → `apply_drift.rs`). So `build_ssh_args`
//! (`transport/ssh.rs:199`) found no socket for every single gate query and
//! emitted a full handshake — measured to localhost at a 306 ms median against
//! 6.7 ms multiplexed, 45×. The same gate was also SEQUENTIAL across machines
//! (`for (machine_name, lock) in &locks`) while `forjar drift` had used a
//! `std::thread::scope` fan-out for the identical work since FJ-1396, and it
//! was UNSCOPED — `apply -r one-resource` still probed every locked resource on
//! the machine and wrote `status: drifted` over resources the run would then
//! skip.
//!
//! WHY THESE ASSERTIONS AND NOT A SUMMARY LINE. forjar prints no per-connection
//! diagnostic, and a timing threshold on a shared build box is a coin flip. The
//! oracle here is the argv of every `ssh` process the binary actually spawns,
//! captured by a shim first on `PATH`:
//!
//!   * `ssh_master_opens_before_any_gate_query` — the FIRST spawn must be the
//!     ControlMaster open, and every later remote command (`… bash`) must carry
//!     a `ControlPath=`. Unfixed, the first spawns are bare gate queries.
//!   * `gate_is_scoped_by_the_resource_filter` — bytes in `state.lock.yaml`.
//!     An out-of-scope resource must not be probed, and must not come back
//!     `drifted`.
//!   * `gate_fans_out_across_machines` — the shim brackets each drift query
//!     with `S`/`E` markers and holds it open. Overlapping brackets are
//!     STRUCTURALLY impossible under a sequential loop, so this cannot pass
//!     vacuously.

use std::collections::HashSet;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forjar() -> &'static str {
    env!("CARGO_BIN_EXE_forjar")
}

/// The nth TEST-NET-1 address of a test's private block.
fn addr(base: u8, i: usize) -> String {
    format!("192.0.2.{}", base as usize + i)
}

/// Delete any ControlMaster socket left behind for these addresses.
///
/// Without this a socket from an earlier run makes `build_ssh_args` emit
/// `ControlPath=` for free, and the multiplexing assertion below would hold
/// without the fix having done anything.
fn clear_sockets(base: u8, count: usize) {
    for i in 0..count {
        let _ = fs::remove_file(format!("/tmp/forjar-ssh/root@{}", addr(base, i)));
    }
}

/// A `ssh` that never leaves the box.
///
/// It records its own argv, materialises the ControlMaster socket when asked to
/// open one (so `build_ssh_args` can see it exactly as real ssh would), and —
/// only for the drift gate's file query, which is identifiable by the `__DIR__`
/// probe it pipes on stdin — brackets itself in an overlap log while holding
/// the "connection" open.
const SSH_SHIM: &str = r#"#!/bin/sh
printf '%s\n' "$*" >> "$FJ_SSH_LOG"

sock=""
master=""
for a in "$@"; do
  case "$a" in
    ControlPath=*) sock="${a#ControlPath=}" ;;
    ControlMaster=yes) master=1 ;;
  esac
done
if [ -n "$master" ] && [ -n "$sock" ]; then
  : > "$sock"
fi

prev=""
last=""
for a in "$@"; do prev="$last"; last="$a"; done
if [ "$last" != "bash" ]; then
  exit 0
fi

script=$(cat)
if [ -n "$FJ_SSH_OVERLAP_LOG" ]; then
  case "$script" in
    *__DIR__*)
      printf 'S %s\n' "$prev" >> "$FJ_SSH_OVERLAP_LOG"
      sleep 0.6
      printf 'E %s\n' "$prev" >> "$FJ_SSH_OVERLAP_LOG"
      ;;
  esac
fi
exit 0
"#;

struct Fleet {
    dir: tempfile::TempDir,
}

impl Fleet {
    fn new() -> Self {
        Self {
            dir: tempfile::tempdir().expect("tempdir"),
        }
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.dir.path().join(rel)
    }

    /// Install the shim as `ssh` in a directory that will be prepended to PATH.
    fn shim_bin(&self) -> PathBuf {
        let bin = self.path("bin");
        fs::create_dir_all(&bin).expect("create bin");
        let ssh = bin.join("ssh");
        fs::write(&ssh, SSH_SHIM).expect("write ssh shim");
        fs::set_permissions(&ssh, fs::Permissions::from_mode(0o755)).expect("chmod ssh shim");
        bin
    }

    /// One config with `machines` remote SSH targets, each declaring `ids`
    /// file resources. `192.0.2.x` is TEST-NET-1: unroutable by RFC 5737, so a
    /// shim that failed to intercept cannot reach anything.
    ///
    /// `base` gives each test its own addresses. The ControlMaster socket path
    /// is derived from `user@addr` under a process-wide `/tmp/forjar-ssh`, so
    /// two tests sharing an address would share a socket and could hand each
    /// other a multiplexed connection nobody in that test opened.
    fn write_config(&self, machines: &[&str], base: u8, ids: &[&str]) -> PathBuf {
        let cfg = self.path("forjar.yaml");
        let mut m = String::new();
        for (i, name) in machines.iter().enumerate() {
            m.push_str(&format!(
                "  {name}: {{ hostname: {name}, addr: {}, user: root }}\n",
                addr(base, i)
            ));
        }
        let mut r = String::new();
        for name in machines {
            for id in ids {
                r.push_str(&format!(
                    "  {name}-{id}: {{ type: file, machine: {name}, path: /srv/{id}.txt, content: \"x\\n\", mode: \"0644\" }}\n"
                ));
            }
        }
        fs::write(
            &cfg,
            format!("version: \"1.0\"\nname: e02\nmachines:\n{m}resources:\n{r}"),
        )
        .expect("write config");
        cfg
    }

    /// Seed a converged lock so the gate has something to probe. Every entry
    /// carries the `path`/`content_hash` pair `detect_drift_with_lifecycle`
    /// needs, so each one costs exactly one remote query.
    fn seed_lock(&self, machine: &str, ids: &[&str]) {
        let dir = self.path("state").join(machine);
        fs::create_dir_all(&dir).expect("create machine state dir");
        let mut body = String::new();
        for id in ids {
            body.push_str(&format!(
                "  {machine}-{id}:\n    type: file\n    status: converged\n    hash: \"seeded\"\n    details:\n      path: /srv/{id}.txt\n      content_hash: \"0000000000000000000000000000000000000000000000000000000000000000\"\n"
            ));
        }
        fs::write(
            dir.join("state.lock.yaml"),
            format!(
                "schema: \"1.0\"\nmachine: {machine}\nhostname: {machine}\n\
                 generated_at: \"2026-01-01T00:00:00Z\"\ngenerator: e02-fixture\n\
                 blake3_version: \"1.5\"\nresources:\n{body}"
            ),
        )
        .expect("write seeded lock");
    }

    fn lock_text(&self, machine: &str) -> String {
        fs::read_to_string(self.path("state").join(machine).join("state.lock.yaml"))
            .expect("read lock")
    }

    fn run(&self, cfg: &Path, args: &[&str], overlap_log: Option<&Path>) -> Run {
        let bin = self.shim_bin();
        let ssh_log = self.path("ssh.log");
        let _ = fs::remove_file(&ssh_log);
        let path = format!(
            "{}:{}",
            bin.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let mut cmd = Command::new(forjar());
        cmd.arg("apply")
            .arg("-f")
            .arg(cfg)
            .arg("--state-dir")
            .arg(self.path("state"))
            .args(args)
            .env("PATH", path)
            .env("FJ_SSH_LOG", &ssh_log)
            .env("HOME", self.dir.path());
        if let Some(log) = overlap_log {
            cmd.env("FJ_SSH_OVERLAP_LOG", log);
        }
        let out = cmd.output().expect("run forjar apply");
        Run {
            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
            ssh: fs::read_to_string(&ssh_log).unwrap_or_default(),
        }
    }
}

struct Run {
    stderr: String,
    /// One line per `ssh` spawn, the argv joined by spaces.
    ssh: String,
}

impl Run {
    fn spawns(&self) -> Vec<&str> {
        self.ssh.lines().filter(|l| !l.trim().is_empty()).collect()
    }

    /// The spawns that carry a remote command — `build_ssh_args` always ends
    /// them with the literal `bash`. Master opens (`-N -f`) and control
    /// operations (`-O check` / `-O exit`) do not.
    fn remote_commands(&self) -> Vec<&str> {
        self.spawns()
            .into_iter()
            .filter(|l| l.split_whitespace().next_back() == Some("bash"))
            .collect()
    }
}

// ---------------------------------------------------------------------------

/// The gate must run THROUGH the multiplexed connection, not before it exists.
#[test]
fn ssh_master_opens_before_any_gate_query() {
    let fleet = Fleet::new();
    clear_sockets(10, 1);
    let cfg = fleet.write_config(&["alpha"], 10, &["a", "b", "c", "d"]);
    fleet.seed_lock("alpha", &["a", "b", "c", "d"]);

    let run = fleet.run(&cfg, &["--yes"], None);

    let spawns = run.spawns();
    assert!(
        !spawns.is_empty(),
        "fixture is inert: forjar spawned no ssh at all.\nstderr:\n{}",
        run.stderr
    );
    assert!(
        spawns[0].contains("ControlMaster=yes"),
        "the FIRST ssh spawn must be the ControlMaster open, so every later \
         query rides the socket. Got:\n  {}\nall spawns:\n{}",
        spawns[0],
        run.ssh
    );

    let remote = run.remote_commands();
    assert!(
        remote.len() >= 4,
        "expected one remote query per locked resource, got {}:\n{}",
        remote.len(),
        run.ssh
    );
    let fresh: Vec<&&str> = remote
        .iter()
        .filter(|l| !l.contains("ControlPath="))
        .collect();
    assert!(
        fresh.is_empty(),
        "{} remote command(s) opened a FRESH handshake instead of reusing the \
         ControlMaster socket:\n{}",
        fresh.len(),
        fresh
            .iter()
            .map(|l| format!("  {l}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// `-r` scopes what is APPLIED; it must scope what is PROBED and RECORDED too.
#[test]
fn gate_is_scoped_by_the_resource_filter() {
    let fleet = Fleet::new();
    clear_sockets(30, 1);
    let cfg = fleet.write_config(&["alpha"], 30, &["a", "b"]);
    fleet.seed_lock("alpha", &["a", "b"]);

    let run = fleet.run(&cfg, &["--yes", "-r", "alpha-a"], None);

    assert!(
        !run.stderr.contains("alpha-b"),
        "the gate probed alpha-b, which `-r alpha-a` excluded from the run:\n{}",
        run.stderr
    );

    let lock = fleet.lock_text("alpha");
    let b_block = lock
        .split("alpha-b:")
        .nth(1)
        .unwrap_or_else(|| panic!("alpha-b vanished from the lock:\n{lock}"));
    let b_status = b_block
        .lines()
        .find(|l| l.trim_start().starts_with("status:"))
        .unwrap_or_else(|| panic!("alpha-b has no status:\n{lock}"));
    assert!(
        b_status.contains("converged"),
        "an out-of-scope resource was rewritten as drifted by a gate that \
         `-r alpha-a` should have kept away from it: {}\nlock:\n{lock}",
        b_status.trim()
    );
}

/// One machine must not wait on another machine's handshakes.
#[test]
fn gate_fans_out_across_machines() {
    let fleet = Fleet::new();
    clear_sockets(50, 3);
    let cfg = fleet.write_config(&["alpha", "bravo", "charlie"], 50, &["a"]);
    for m in ["alpha", "bravo", "charlie"] {
        fleet.seed_lock(m, &["a"]);
    }
    let overlap = fleet.path("overlap.log");

    // `--dry-run` so the ONLY remote work in the run is the gate itself; the
    // executor returns before it reaches a machine.
    let run = fleet.run(&cfg, &["--dry-run"], Some(&overlap));

    let log = fs::read_to_string(&overlap).unwrap_or_default();
    let marks: Vec<&str> = log.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(
        marks.len(),
        6,
        "expected an S/E bracket per machine (3 machines): got {marks:?}\nssh:\n{}\nstderr:\n{}",
        run.ssh,
        run.stderr
    );

    let mut open: HashSet<&str> = HashSet::new();
    let mut overlapped = false;
    for m in &marks {
        let (kind, who) = m.split_at(1);
        let who = who.trim();
        if kind == "S" {
            if !open.is_empty() {
                overlapped = true;
            }
            open.insert(who);
        } else {
            open.remove(who);
        }
    }
    assert!(
        overlapped,
        "the drift gate probed the three machines strictly one after another — \
         no two queries were ever open at the same time. Markers:\n{log}"
    );
}
/// The SAME must hold for `-t` and `-g`, whose branches of the gate's
/// predicate are separate code from `-r`'s.
///
/// ADVERSARIAL REVIEW (forjar#404): `-t` shipped untested, and `-g` was not
/// scoped AT ALL — `group_filter` was not even a parameter of
/// `apply_pre_validate`, so `GateScope` could not have honoured it in
/// principle. Measured on that build, `apply -g net` printed
/// `drift: [alpha] alpha-b` for an out-of-group resource and left
/// `status: drifted` in its lock entry while the same run reported it skipped.
///
/// One oracle, two callers: an excluded resource must be neither PROBED (each
/// probe is a remote query, which is what this issue costs) nor RECORDED as
/// drifted (a lock that claims a repair no run performed).
fn assert_gate_left_alpha_b_alone(fleet: &Fleet, cfg: &Path, args: &[&str], excluded_by: &str) {
    let run = fleet.run(cfg, args, None);

    assert!(
        !run.stderr.contains("alpha-b"),
        "the gate probed alpha-b, which `{excluded_by}` excluded from the run:\n{}",
        run.stderr
    );

    let lock = fleet.lock_text("alpha");
    let b_block = lock
        .split("alpha-b:")
        .nth(1)
        .unwrap_or_else(|| panic!("alpha-b vanished from the lock:\n{lock}"));
    let b_status = b_block
        .lines()
        .find(|l| l.trim_start().starts_with("status:"))
        .unwrap_or_else(|| panic!("alpha-b has no status:\n{lock}"));
    assert!(
        b_status.contains("converged"),
        "an out-of-scope resource was rewritten as drifted by a gate that \
         `{excluded_by}` should have kept away from it: {}\nlock:\n{lock}",
        b_status.trim()
    );
}

/// Two resources, one selector each, so a filter can exclude exactly one.
fn write_selector_config(fleet: &Fleet, base: u8, a_sel: &str, b_sel: &str) -> PathBuf {
    let cfg = fleet.path("forjar.yaml");
    fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: e02-sel\nmachines:\n  \
             alpha: {{ hostname: alpha, addr: {}, user: root }}\nresources:\n  \
             alpha-a: {{ type: file, machine: alpha, path: /srv/a.txt, content: \"x\\n\", mode: \"0644\", {a_sel} }}\n  \
             alpha-b: {{ type: file, machine: alpha, path: /srv/b.txt, content: \"x\\n\", mode: \"0644\", {b_sel} }}\n",
            addr(base, 0)
        ),
    )
    .expect("write selector config");
    cfg
}

#[test]
fn gate_is_scoped_by_the_tag_filter() {
    let fleet = Fleet::new();
    clear_sockets(70, 1);
    let cfg = write_selector_config(&fleet, 70, "tags: [web]", "tags: [db]");
    fleet.seed_lock("alpha", &["a", "b"]);
    assert_gate_left_alpha_b_alone(&fleet, &cfg, &["--yes", "-t", "web"], "-t web");
}

#[test]
fn gate_is_scoped_by_the_group_filter() {
    let fleet = Fleet::new();
    clear_sockets(110, 1);
    let cfg = write_selector_config(&fleet, 110, "resource_group: net", "resource_group: db");
    fleet.seed_lock("alpha", &["a", "b"]);
    assert_gate_left_alpha_b_alone(&fleet, &cfg, &["--yes", "-g", "net"], "-g net");
}

/// A ControlMaster this run did NOT open belongs to somebody else.
///
/// ADVERSARIAL REVIEW (forjar#404): the fix hoists the fleet's masters up into
/// `cmd_apply`, and `executor::machine` recorded `Ok(_) => true` — it claimed
/// ownership of a socket it had merely FOUND (`start_control_master` returns
/// `Ok(false)` for "one is already running"). With the hoist in place that
/// makes the executor tear down a socket the run-level guard, a concurrent
/// apply, or a live `ControlPersist` window still owns. The commit changed it
/// to `Ok(started) => started` and shipped no test for that hunk; this is it.
#[test]
fn a_control_master_this_run_did_not_open_survives_the_apply() {
    let fleet = Fleet::new();
    clear_sockets(90, 1);
    let cfg = fleet.write_config(&["alpha"], 90, &["a"]);
    fleet.seed_lock("alpha", &["a"]);

    // Stand in for a concurrent apply's master: the socket is already there and
    // the shim answers `ssh -O check` successfully, so forjar's
    // `start_control_master` reports "already running" and this run owns none.
    let sock = PathBuf::from(format!("/tmp/forjar-ssh/root@{}", addr(90, 0)));
    fs::create_dir_all("/tmp/forjar-ssh").expect("create control dir");
    fs::write(&sock, b"").expect("seed a foreign ControlMaster socket");

    let run = fleet.run(&cfg, &["--yes"], None);
    let survived = sock.exists();
    let _ = fs::remove_file(&sock);

    assert!(
        survived,
        "the apply tore down a ControlMaster socket it never opened — that is \
         a concurrent apply's live connection. ssh spawns:\n{}\nstderr:\n{}",
        run.ssh, run.stderr
    );
}
