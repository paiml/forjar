//! forjar#501: an I/O hash is recorded and read only for a machine this host
//! answers for.
//!
//! Two sites, one shape — the forjar#485 defect in the write path and in the
//! cache reader beside it:
//!
//! * `record_io_hashes` had no machine argument. `record_success` called it for
//!   every (resource, machine) a successful apply converged, so a REMOTE
//!   machine's lock recorded `input_hash` / `output_hash` of the CONTROLLER's
//!   tree: a correct hash of the wrong files.
//! * `check_task_input_cache` read that hash back for a `cache: true` task on
//!   ANY machine and skipped the run when the controller's current hash matched
//!   it — deciding "inputs unchanged" on a remote machine from files that were
//!   never there.
//! * Beside it, the reader hashed relative to the state directory's parent
//!   while the writer hashed relative to `working_dir`; `hash_inputs` folds the
//!   expanded path into the hash, so the two could never agree. Measured on
//!   643363b3: a `cache: true` task re-ran on every apply. The dead cache is
//!   what hid the remote half.
//!
//! Every assertion spawns the real binary. `far` starts as this host (loopback)
//! so the lock records real hashes, then moves to a TEST-NET address this host
//! cannot answer for. The machine name is not part of the resource hash, so
//! the lock still matches; only the cache can decide the row.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

const FORJAR: &str = env!("CARGO_BIN_EXE_forjar");

/// RFC 5737 TEST-NET-3: never routable, never this host.
const REMOTE: &str = "203.0.113.7";

struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    /// The project lives in `proj/`, the state directory beside it — so the
    /// state directory's parent is NOT the working directory, which is the
    /// base mismatch the reader had.
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("forjar-501-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("proj")).expect("sandbox");
        fs::write(dir.join("proj/src.txt"), "v1\n").expect("input");
        let sb = Self { dir };
        sb.write_config("127.0.0.1", "one");
        sb
    }

    fn cfg(&self) -> PathBuf {
        self.dir.join("forjar.yaml")
    }

    /// `tag` changes the command (so the config hash moves and the row plans
    /// `Update`) without touching the declared inputs.
    fn write_config(&self, far_addr: &str, tag: &str) {
        let cfg = format!(
            "version: \"1.0\"\nname: cache501\nmachines:\n  far:\n    hostname: far\n\
             \x20   addr: {far_addr}\nresources:\n  build:\n    type: task\n\
             \x20   machine: [far]\n\
             \x20   command: \"cat src.txt > out.txt; echo {tag} >> runs.log\"\n\
             \x20   working_dir: {}\n    task_inputs: [src.txt]\n\
             \x20   output_artifacts: [out.txt]\n    cache: true\n",
            self.dir.join("proj").display()
        );
        fs::write(self.cfg(), cfg).expect("config");
    }

    /// How many times the task's command actually ran.
    fn runs(&self) -> usize {
        fs::read_to_string(self.dir.join("proj/runs.log"))
            .map(|s| s.lines().count())
            .unwrap_or(0)
    }

    fn lock(&self) -> String {
        fs::read_to_string(self.dir.join("state/far/state.lock.yaml")).unwrap_or_default()
    }

    /// Converge on loopback: the command runs once and the lock records a
    /// real input hash of this host's tree.
    fn converge_locally(&self) {
        let (code, out) = self.apply();
        assert_eq!(
            code, 0,
            "precondition: the local converge must succeed.\n{out}"
        );
        assert_eq!(self.runs(), 1, "precondition: the command ran once.\n{out}");
        assert!(
            self.lock().contains("input_hash"),
            "precondition: the lock recorded an input hash.\n{}",
            self.lock()
        );
    }

    fn apply(&self) -> (i32, String) {
        let out = Command::new(FORJAR)
            .args(["apply", "--yes", "-f"])
            .arg(self.cfg())
            .current_dir(&self.dir)
            .output()
            .expect("run forjar");
        let text = strip_ansi(&format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ));
        (out.status.code().unwrap_or(-1), text)
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn strip_ansi(s: &str) -> String {
    let mut clean = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for c2 in chars.by_ref() {
                if c2 == 'm' {
                    break;
                }
            }
        } else {
            clean.push(c);
        }
    }
    clean
}

/// THE BASE. The command changed, the declared inputs did not, the machine is
/// this host. A `cache: true` task must be reported unchanged and must not run
/// — that is what the cache is for. Before forjar#501 the reader hashed the
/// state directory's parent, where `src.txt` is not, so the cache never hit
/// and the task ran on every apply.
#[test]
fn a_cache_hit_is_decided_from_the_base_the_writer_used() {
    let sb = Sandbox::new("base");
    sb.converge_locally();
    sb.write_config("127.0.0.1", "two");

    let (code, out) = sb.apply();
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("1 unchanged"),
        "the declared inputs did not move; the cache must answer from the same \
         base the writer used.\n{out}"
    );
    assert_eq!(
        sb.runs(),
        1,
        "the task ran again over unchanged inputs: the cache never hit.\n{out}"
    );
}

/// THE MACHINE. The same task, now on a machine this host cannot answer for,
/// with its config changed. The lock's `input_hash` is a hash of THIS host's
/// tree; the reader must not decide "inputs unchanged" on `far` from it. The
/// only honest answer is to run the task there — which fails at ssh, and says
/// so — never to report the row unchanged. With the machine guard removed
/// from the reader this test fails: the row is skipped from the controller's
/// hash without one connection attempt.
#[test]
fn a_remote_cache_true_task_is_never_skipped_from_the_controllers_hash() {
    let sb = Sandbox::new("remote");
    sb.converge_locally();
    sb.write_config(REMOTE, "two");

    let (code, out) = sb.apply();
    assert!(
        !out.contains("1 unchanged"),
        "a task on a machine this host cannot read was reported unchanged from \
         a hash of this host's files.\n{out}"
    );
    assert_ne!(
        code, 0,
        "the run must reach the machine and fail there, not succeed here.\n{out}"
    );
    assert!(
        out.contains("ssh") && out.contains("1 failed"),
        "the failure must be the connection to far, named as such.\n{out}"
    );
    assert!(
        sb.lock().contains("status: failed"),
        "the lock row must record the failed run, not keep a converged row it \
         never re-verified.\n{}",
        sb.lock()
    );
    assert_eq!(
        sb.runs(),
        1,
        "nothing ran on this host for a remote row.\n{out}"
    );
}
