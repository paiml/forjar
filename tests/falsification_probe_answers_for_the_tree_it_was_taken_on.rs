//! forjar#499: a build-I/O probe answers only for the tree it was taken on.
//!
//! `probe_all` inserted one `IoDigest` per RESOURCE ID, probed on the
//! controller, for any resource with at least one machine this host answers
//! for. `determine_present_action` then looked the digest up by resource id
//! for EVERY (resource, machine) row it planned. So a task declared on
//! `[box, far]` — box this host, far an SSH target — carried the local probe's
//! verdict for the far row: local inputs stale → far planned `Update` from a
//! tree nobody read; local inputs fresh → far planned `NoOp` (disclosed since
//! forjar#497, because the census asked the machine predicate before the map
//! precisely because the map could not answer per machine).
//!
//! It is the forjar#485 shape one level up: a number measured on one machine
//! reported as a fact about another, invisible because it is a correct hash
//! of the wrong tree. Multi-machine task resources with declared build I/O are
//! exactly the fleet's build boxes.
//!
//! The fix keys the probe by (machine, resource). Every assertion spawns the
//! real binary. The fixture converges the task on two loopback machines so
//! BOTH locks record real I/O hashes, then moves `far` to a TEST-NET address
//! this host cannot answer for. The machine name is not part of the resource
//! hash, so both locks still match; only the probe can move a row, and it
//! must move only the row it measured.

use std::fs;
use std::process::Command;

const FORJAR: &str = env!("CARGO_BIN_EXE_forjar");

/// RFC 5737 TEST-NET-3: never routable, never this host.
const REMOTE: &str = "203.0.113.7";

struct Sandbox {
    dir: std::path::PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("forjar-499-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("sandbox");
        fs::write(dir.join("src.txt"), "v1\n").expect("input");
        let sb = Self { dir };
        sb.write_config("127.0.0.1");
        sb
    }

    fn cfg(&self) -> std::path::PathBuf {
        self.dir.join("forjar.yaml")
    }

    /// `box` is always this host; `far` is wherever the test puts it.
    fn write_config(&self, far_addr: &str) {
        let cfg = format!(
            "version: \"1.0\"\nname: keyed\nmachines:\n  box:\n    hostname: box\n\
             \x20   addr: 127.0.0.1\n  far:\n    hostname: far\n    addr: {far_addr}\n\
             resources:\n  build:\n    type: task\n    machine: [box, far]\n\
             \x20   command: \"cat src.txt > out.txt\"\n    working_dir: {}\n\
             \x20   task_inputs: [src.txt]\n    output_artifacts: [out.txt]\n",
            self.dir.display()
        );
        fs::write(self.cfg(), cfg).expect("config");
    }

    /// Converge on both machines while both are this host, so each lock
    /// holds real input and output hashes.
    fn converge_locally(&self) {
        let out = self.run(&["apply", "--yes"]);
        assert!(
            self.dir.join("out.txt").exists(),
            "precondition: the task must have run locally.\n{out}"
        );
        let json = self.run_json(&["plan", "--json"]);
        assert_eq!(
            json["unchanged"].as_u64(),
            Some(2),
            "precondition: both rows converged and probed.\n{json:#}"
        );
    }

    /// Move `far` somewhere this host does not answer for. Both locks are
    /// untouched: the machine name is not part of the resource hash.
    fn move_far_remote(&self) {
        self.write_config(REMOTE);
    }

    fn run(&self, args: &[&str]) -> String {
        let out = Command::new(FORJAR)
            .args(args)
            .arg("-f")
            .arg(self.cfg())
            .current_dir(&self.dir)
            .output()
            .expect("run forjar");
        strip_ansi(&format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ))
    }

    fn run_json(&self, args: &[&str]) -> serde_json::Value {
        let out = Command::new(FORJAR)
            .args(args)
            .arg("-f")
            .arg(self.cfg())
            .current_dir(&self.dir)
            .output()
            .expect("run forjar");
        let stdout = String::from_utf8_lossy(&out.stdout);
        serde_json::from_str(&stdout).unwrap_or_else(|e| {
            panic!(
                "forjar {args:?} did not print JSON: {e}\nstdout: {stdout}\nstderr: {}",
                String::from_utf8_lossy(&out.stderr)
            )
        })
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

/// The action `plan --json` gives the (resource, machine) row.
fn action_of(json: &serde_json::Value, id: &str, machine: &str) -> String {
    json["changes"]
        .as_array()
        .unwrap_or_else(|| panic!("`changes` must be an array:\n{json:#}"))
        .iter()
        .find(|c| c["resource_id"] == id && c["machine"] == machine)
        .unwrap_or_else(|| panic!("no change row for {id}@{machine}:\n{json:#}"))["action"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

fn unprobed_rows(json: &serde_json::Value) -> Vec<(String, String)> {
    json["unprobed"]
        .as_array()
        .unwrap_or_else(|| panic!("`unprobed` must be an array:\n{json:#}"))
        .iter()
        .map(|u| {
            (
                u["resource_id"].as_str().unwrap_or_default().to_string(),
                u["machine"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect()
}

/// THE DEFECT. The local input moved. The local row must rebuild — that is
/// what the probe is for. The remote row's tree was never read, so the local
/// probe must not answer for it: it keeps the config-hash answer, `NoOp`,
/// and is NAMED as not measured. Before the fix both rows planned `Update`,
/// the second one from a hash of the wrong tree.
#[test]
fn a_stale_local_probe_moves_only_the_row_it_measured() {
    let sb = Sandbox::new("stale");
    sb.converge_locally();
    sb.move_far_remote();
    fs::write(sb.dir.join("src.txt"), "v2\n").expect("mutate input");

    let json = sb.run_json(&["plan", "--json"]);
    assert_eq!(
        action_of(&json, "build", "box"),
        "update",
        "the local row saw its declared input move.\n{json:#}"
    );
    assert_eq!(
        action_of(&json, "build", "far"),
        "no_op",
        "the remote tree was never read; a probe taken on this host must not \
         answer for it — that is a correct hash of the wrong tree.\n{json:#}"
    );
    assert_eq!(
        unprobed_rows(&json),
        vec![("build".to_string(), "far".to_string())],
        "the remote row is the one not measured; the local row was.\n{json:#}"
    );

    let plan = sb.run(&["plan"]);
    assert!(
        plan.contains("1 to change") && plan.contains("1 unchanged"),
        "one row rebuilds, one row is unmeasured and unchanged.\n{plan}"
    );
    assert!(
        plan.contains("build@far") && !plan.contains("build@box"),
        "the disclosure names the machine that was not measured and only that one.\n{plan}"
    );
}

/// The forjar#497 disclosure does not regress: with the local input fresh,
/// both rows stay `NoOp` and the remote row is still named.
#[test]
fn a_fresh_local_probe_still_says_nothing_about_the_remote_row() {
    let sb = Sandbox::new("fresh");
    sb.converge_locally();
    sb.move_far_remote();

    let json = sb.run_json(&["plan", "--json"]);
    assert_eq!(action_of(&json, "build", "box"), "no_op", "{json:#}");
    assert_eq!(action_of(&json, "build", "far"), "no_op", "{json:#}");
    assert_eq!(
        unprobed_rows(&json),
        vec![("build".to_string(), "far".to_string())],
        "{json:#}"
    );
}
