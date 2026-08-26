//! `forjar plan` must not present a lock diff as the state of the world.
//!
//! forjar#342. `plan` compares the config to the LOCK. `drift` compares the
//! lock to the HOST. Both are correct for the question they answer, and they
//! give different answers about the same machine:
//!
//!   intel  plan  (lock-relative):  0 to add, 52 to change, 83 unchanged
//!   intel  drift (host-relative):  Drift detected: 28 resource(s)
//!
//! In a sandbox the gap is starker — mutate a managed file on the target and
//! `drift` reports it while `plan` prints `no changes / 1 unchanged`.
//!
//! `plan` is the command people run to decide what `apply` will do, so its
//! silence reads as "nothing else is wrong". The fix is not to make plan
//! consult the host — that would make it slow and network-dependent, and its
//! lock-relative answer is genuinely useful. The fix is that plan must STATE
//! THE QUANTIFIER IT RANGES OVER, so a blind spot is disclosed rather than
//! merely absent from the output.
//!
//! Same family as #305, #337 and #339: a surface reporting a result it did not
//! measure, invisible because the output looks complete.

use std::fs;
use std::process::Command;

const FORJAR: &str = env!("CARGO_BIN_EXE_forjar");

struct Sandbox {
    dir: std::path::PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("forjar-342-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("sandbox");
        let sb = Self { dir };
        sb.write_config();
        sb
    }

    fn managed(&self) -> std::path::PathBuf {
        self.dir.join("managed.txt")
    }

    fn write_config(&self) {
        let cfg = format!(
            "version: \"1.0\"\nname: blind-spot\nmachines:\n  sandbox:\n    hostname: sandbox\n\
             \x20   addr: 127.0.0.1\nresources:\n  a-file:\n    type: file\n\
             \x20   machine: sandbox\n    path: {}\n    content: \"declared\"\n",
            self.managed().display()
        );
        fs::write(self.dir.join("forjar.yaml"), cfg).expect("config");
    }

    fn run(&self, args: &[&str]) -> String {
        let out = Command::new(FORJAR)
            .args(args)
            .arg("-f")
            .arg(self.dir.join("forjar.yaml"))
            .current_dir(&self.dir)
            .output()
            .expect("run forjar");
        let joined = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        // strip ANSI so assertions are about words, not colour codes
        let mut clean = String::with_capacity(joined.len());
        let mut chars = joined.chars();
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
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// THE DEFECT. After a converged apply, `plan` holds observed state it did not
/// consult. It must say so and name the command that can.
#[test]
fn plan_discloses_that_it_did_not_look_at_the_host() {
    let sb = Sandbox::new("discloses");
    sb.run(&["apply", "--yes"]);

    let plan = sb.run(&["plan"]);

    assert!(
        plan.contains("forjar drift"),
        "plan must name the command that CAN see the host — otherwise its \
         silence reads as 'nothing else is wrong'.\n{plan}"
    );
    assert!(
        plan.to_lowercase().contains("lock"),
        "plan must say its answer is lock-relative.\n{plan}"
    );
}

/// AND IT MUST STILL SAY IT WHEN THE LOCK LOOKS CLEAN. This is the whole
/// point: the dangerous case is precisely the one where plan reports nothing
/// to do. A disclosure that only appears alongside pending changes would be
/// absent exactly when it is needed.
#[test]
fn the_disclosure_survives_a_fully_converged_plan() {
    let sb = Sandbox::new("converged");
    sb.run(&["apply", "--yes"]);

    let plan = sb.run(&["plan"]);
    assert!(
        plan.contains("0 to add, 0 to change"),
        "precondition: this plan should be clean.\n{plan}"
    );
    assert!(
        plan.contains("forjar drift"),
        "a CLEAN plan is when the blind spot matters most.\n{plan}"
    );
}

/// The disclosure must be honest about its own trigger: with no lock at all,
/// there is no observed state to be blind to, and claiming otherwise would be
/// noise that trains people to ignore the line.
#[test]
fn no_lock_means_no_disclosure() {
    let sb = Sandbox::new("nolock");

    let plan = sb.run(&["plan"]);
    assert!(
        !plan.contains("forjar drift"),
        "with no lock there is no observed state plan failed to consult; \
         an unconditional banner is noise.\n{plan}"
    );
}
