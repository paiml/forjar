//! forjar#488 / forjar#485: `drift -f <config>` answered about other machines.
//!
//! `drift` walked every stack directory under `--state-dir` regardless of the
//! config it was pointed at. The operator measured 31 stacks checked from a
//! config declaring one machine, with the first row reported belonging to a
//! different box:
//!
//! ```text
//! $ forjar drift -f machines/yoga/forjar.yaml | grep -c '^Checking '
//! 31
//! $ forjar drift -f machines/yoga/forjar.yaml | grep '^Checking ' | head -1
//! Checking gx10 (67 resources)...
//! ```
//!
//! That is worse than noise, and forjar#485 is why. `check_machine_drift` looks
//! the stack up with `config.machines.get(name)`; a stack the loaded config does
//! not declare misses, falls to the lock-only arm, and is compared against
//! THIS box's filesystem. So a resource that lives on another machine is judged
//! by the path of the same name on the workstation. The operator read
//!
//! ```text
//! DRIFTED: bashrc (/home/noah/.bashrc content changed)
//! ```
//!
//! as yoga's, and spent a while establishing that `machines/yoga/forjar.yaml`
//! has no `bashrc` at all. It is gx10's, surfaced by a command pointed at yoga,
//! and the two hashes differ because they are hashes of two different files on
//! two different computers. `forjar apply -r bashrc` converged the remote one
//! and the next `drift` re-read the local one, for ever — which is what made
//! forjar#485 look like a resource that "converges forever and is never
//! converged", and what kept the fleet drift lane from ever being green.
//!
//! DRIVEN THROUGH THE REAL BINARY, because what is under test is what the
//! operator reads.

use std::fs;
use std::process::Command;

const FORJAR: &str = env!("CARGO_BIN_EXE_forjar");

struct Fleet {
    dir: std::path::PathBuf,
}

impl Fleet {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("forjar-488-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("files")).expect("sandbox");
        Self { dir }
    }

    fn state(&self) -> std::path::PathBuf {
        self.dir.join("state")
    }

    fn file_for(&self, stack: &str) -> std::path::PathBuf {
        self.dir.join("files").join(format!("{stack}.txt"))
    }

    /// One stack, one machine, one file resource. The machine KEY is the stack
    /// name, which is also the state directory name — the join `drift` makes.
    fn write_stack(&self, stack: &str) -> std::path::PathBuf {
        let cfg = format!(
            "version: \"1.0\"\nname: {stack}\nmachines:\n  {stack}:\n\
             \x20   hostname: {stack}\n    addr: 127.0.0.1\nresources:\n  probe-{stack}:\n\
             \x20   type: file\n    machine: {stack}\n\
             \x20   path: \"{path}\"\n\
             \x20   content: |\n      content declared for {stack}\n\
             \x20   mode: \"0644\"\n",
            stack = stack,
            path = self.file_for(stack).display()
        );
        let p = self.dir.join(format!("{stack}.yaml"));
        fs::write(&p, cfg).expect("config");
        p
    }

    fn run(&self, args: &[&str]) -> (String, bool) {
        let out = Command::new(FORJAR)
            .args(args)
            .arg("--state-dir")
            .arg(self.state())
            .arg("--no-color")
            .current_dir(&self.dir)
            .output()
            .expect("forjar runs");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        (text, out.status.success())
    }

    fn apply(&self, cfg: &std::path::Path) {
        let (plan, _) = self.run(&["plan", "-f", &cfg.display().to_string()]);
        assert!(
            plan.contains("to add") || plan.contains("unchanged"),
            "the plan must describe the stack before it is applied: {plan}"
        );
        let (text, ok) = self.run(&["apply", "-f", &cfg.display().to_string(), "--yes"]);
        assert!(
            ok,
            "the fixture must converge before drift is measured: {text}"
        );
    }

    /// A two-stack fleet: `home` is the one the operator asks about, `elsewhere`
    /// is the other box. `elsewhere`'s file is then changed, which on a real
    /// fleet is not a change at all — it is simply a DIFFERENT computer's copy
    /// of the path, which is exactly what the workstation reads.
    fn two_stacks(name: &str) -> (Self, std::path::PathBuf) {
        let fleet = Fleet::new(name);
        let home = fleet.write_stack("home");
        let elsewhere = fleet.write_stack("elsewhere");
        fleet.apply(&home);
        fleet.apply(&elsewhere);
        fs::write(
            fleet.file_for("elsewhere"),
            "what THIS computer has at that path\n",
        )
        .expect("the other box's copy");
        (fleet, home)
    }
}

#[test]
fn drift_does_not_report_a_stack_the_config_never_declares() {
    let (fleet, home) = Fleet::two_stacks("scope");
    let (text, _) = fleet.run(&["drift", "-f", &home.display().to_string(), "-v"]);

    assert!(
        !text.contains("probe-elsewhere"),
        "forjar#488: `drift -f home.yaml` reported a resource belonging to a stack \
         that config never declares. On a fleet that resource lives on another \
         computer and this verdict is a hash of the wrong machine's file \
         (forjar#485).\n{text}"
    );
    assert!(
        !text.contains("Checking elsewhere"),
        "forjar#488: `drift -f home.yaml` walked a stack the config never \
         declares.\n{text}"
    );
    assert!(
        text.contains("Checking home"),
        "the stack the config DOES declare must still be checked.\n{text}"
    );
    assert!(
        text.contains("No drift detected"),
        "the only stack in scope is converged, so the verdict is clean.\n{text}"
    );
}

#[test]
fn the_summary_counts_only_the_stacks_in_scope() {
    let (fleet, home) = Fleet::two_stacks("summary");
    let (text, ok) = fleet.run(&["drift", "-f", &home.display().to_string(), "--tripwire"]);
    assert!(
        ok,
        "forjar#488: --tripwire exited non-zero for drift on a stack the config \
         never declares. A per-machine tripwire cannot be written with -f while \
         this is true.\n{text}"
    );
    assert!(
        !text.contains("Drift detected"),
        "the aggregate verdict must be about the config it was given.\n{text}"
    );
}

#[test]
fn all_stacks_restores_the_aggregate_and_every_row_names_its_machine() {
    let (fleet, home) = Fleet::two_stacks("allstacks");
    let (text, _) = fleet.run(&[
        "drift",
        "-f",
        &home.display().to_string(),
        "--all-stacks",
        "-v",
    ]);
    assert!(
        text.contains("probe-elsewhere"),
        "--all-stacks is how the wide question is asked, and it must still \
         answer it.\n{text}"
    );
    for line in text.lines().filter(|l| l.contains("DRIFTED:")) {
        assert!(
            line.contains("elsewhere") || line.contains("home"),
            "forjar#488: a row in an aggregated run does not name its machine, so \
             the output cannot be attributed after the fact — which is how gx10's \
             bashrc was read as yoga's.\n{line}"
        );
    }
}

#[test]
fn a_declared_machine_with_no_state_yet_is_not_an_error() {
    let fleet = Fleet::new("nostate");
    let home = fleet.write_stack("home");
    let elsewhere = fleet.write_stack("elsewhere");
    fleet.apply(&elsewhere);
    // `home` has never been applied, so it has no state directory at all.
    let (text, ok) = fleet.run(&["drift", "-f", &home.display().to_string()]);
    assert!(
        ok,
        "a machine that has never been applied is not a drift failure — scoping \
         must not turn `drift` before the first apply into an error.\n{text}"
    );
    assert!(
        !text.contains("probe-elsewhere"),
        "and it must still not answer about the other stack.\n{text}"
    );
}
