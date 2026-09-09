//! forjar#497: the planner must not present "never probed" as "probed, nothing
//! stale".
//!
//! `determine_present_action` consults `probes.get(resource_id)` and, when a
//! probe exists, lets observed staleness override a matching config hash. When
//! no probe exists it falls through to the config-hash comparison — and a
//! MISSING probe is indistinguishable from a probe that found nothing stale.
//! The probe is only taken for machines this host answers for, so a converged
//! task on ANY non-local machine plans `NoOp` as long as its YAML is unchanged,
//! however much its declared inputs changed on disk. That has been true for
//! every SSH target since the probe existed; forjar#495 added namespaces.
//!
//! This repository's own doctrine (`scripts/dogfood/*.sh`, `DriftCensus`) is
//! that an UNMEASURED check must never print the same thing as a passed one.
//! `drift` names why each resource was skipped; the planner had no census.
//!
//! The fix the issue asks for is disclosure, not a remote probe: the action
//! stays `NoOp` (a rebuild-every-run would break f(f(x)) = f(x) at the plan
//! level), but the plan SAYS which resources it could not measure, per
//! (resource, machine), in the forjar#342 / #372 shape — a TOTAL list beside a
//! PARTIAL prose disclosure that names the instrument that can answer.
//!
//! Every assertion spawns the real binary. The fixture converges a task on a
//! loopback machine so the lock records real input/output hashes, then moves
//! the machine to a TEST-NET address this host cannot answer for. The machine
//! name is not part of the resource hash, so the lock still matches and the
//! planner's only remaining signal is the probe it can no longer take.

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
        let dir = std::env::temp_dir().join(format!("forjar-497-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("sandbox");
        fs::write(dir.join("src.txt"), "v1\n").expect("input");
        let sb = Self { dir };
        sb.write_config("127.0.0.1", "cat src.txt > out.txt");
        sb
    }

    fn cfg(&self) -> std::path::PathBuf {
        self.dir.join("forjar.yaml")
    }

    fn write_config(&self, addr: &str, command: &str) {
        let cfg = format!(
            "version: \"1.0\"\nname: unprobed\nmachines:\n  box:\n    hostname: box\n\
             \x20   addr: {addr}\nresources:\n  build:\n    type: task\n    machine: box\n\
             \x20   command: \"{command}\"\n    working_dir: {}\n    task_inputs: [src.txt]\n\
             \x20   output_artifacts: [out.txt]\n",
            self.dir.display()
        );
        fs::write(self.cfg(), cfg).expect("config");
    }

    /// Converge on the loopback machine so the lock holds real I/O hashes.
    fn converge_locally(&self) {
        let out = self.run(&["apply", "--yes"]);
        assert!(
            self.dir.join("out.txt").exists(),
            "precondition: the task must have run locally.\n{out}"
        );
    }

    /// Move the machine somewhere this host does not answer for. The lock is
    /// untouched: the machine name is not part of the resource hash.
    fn move_remote(&self) {
        self.write_config(REMOTE, "cat src.txt > out.txt");
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

/// THE DEFECT. The task's declared input changed on disk, the machine is one
/// this host cannot probe, and the plan says `0 to change` — which is the
/// unchanged behaviour and stays so. What must not stay is the silence: the
/// plan has to name the resource it did not measure, on which machine, and the
/// instrument that can.
#[test]
fn a_remote_task_whose_inputs_changed_plans_noop_and_says_so() {
    let sb = Sandbox::new("changed");
    sb.converge_locally();
    sb.move_remote();
    fs::write(sb.dir.join("src.txt"), "v2\n").expect("mutate input");

    let plan = sb.run(&["plan"]);

    assert!(
        plan.contains("0 to add, 0 to change"),
        "precondition: without a probe the planner falls through to the config \
         hash, and that is not what this ticket changes.\n{plan}"
    );
    assert!(
        plan.contains("did not measure"),
        "the plan must say it did not measure this resource's declared build I/O; \
         a missing probe printed as `unchanged` is 'unmeasured reads as clean'.\n{plan}"
    );
    assert!(
        plan.contains("build@box"),
        "the disclosure must name the resource AND the machine it was not probed on.\n{plan}"
    );
    assert!(
        plan.contains("forjar drift"),
        "a disclosed blind spot must name the instrument that can see into it.\n{plan}"
    );
}

/// The census is about MEASUREMENT, not about change. Whether the inputs moved
/// is exactly what the planner cannot know, so the disclosure must not depend
/// on it.
#[test]
fn the_disclosure_does_not_depend_on_whether_the_inputs_moved() {
    let sb = Sandbox::new("unmoved");
    sb.converge_locally();
    sb.move_remote();

    let plan = sb.run(&["plan"]);
    assert!(
        plan.contains("0 to add, 0 to change") && plan.contains("build@box"),
        "an unprobed converged resource is disclosed even when nothing changed — \
         the planner has no way to tell, and that is the point.\n{plan}"
    );
}

/// The suppression at zero. A task on a machine this host answers for IS
/// probed, so there is nothing to disclose — an unconditional banner is noise
/// that trains operators to skip the line. The machine-readable surface still
/// carries the list, empty, so a consumer can tell "nothing unprobed" from an
/// older binary that emits no such key.
#[test]
fn a_probed_task_is_not_named_and_the_json_list_is_present_but_empty() {
    let sb = Sandbox::new("probed");
    sb.converge_locally();

    let plan = sb.run(&["plan"]);
    assert!(
        plan.contains("0 to add, 0 to change"),
        "precondition: converged and probed.\n{plan}"
    );
    assert!(
        !plan.contains("did not measure"),
        "a probed resource must not be disclosed as unprobed.\n{plan}"
    );

    let json = sb.run_json(&["plan", "--json"]);
    let unprobed = json
        .get("unprobed")
        .unwrap_or_else(|| panic!("plan --json must carry `unprobed` as a TOTAL list:\n{json:#}"));
    assert_eq!(
        unprobed.as_array().map(Vec::len),
        Some(0),
        "nothing unprobed ⇒ an empty list, never an absent key.\n{json:#}"
    );
}

/// The consumers that cannot NOTICE a missing disclosure — a CI parser, an MCP
/// agent reading `to_update: 0` — are the ones that most need the list.
#[test]
fn json_carries_the_census_as_a_total_list_and_folds_it_into_the_disclosure() {
    let sb = Sandbox::new("json");
    sb.converge_locally();
    sb.move_remote();

    let json = sb.run_json(&["plan", "--json"]);
    let unprobed = json["unprobed"]
        .as_array()
        .unwrap_or_else(|| panic!("`unprobed` must be an array:\n{json:#}"));
    assert_eq!(
        unprobed.len(),
        1,
        "one converged task, one entry.\n{json:#}"
    );
    assert_eq!(unprobed[0]["resource_id"], "build", "{json:#}");
    assert_eq!(unprobed[0]["machine"], "box", "{json:#}");
    let reason = unprobed[0]["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("box"),
        "the reason names the machine this host does not answer for.\n{json:#}"
    );
    let disclosure = json["disclosure"].as_str().unwrap_or_default();
    assert!(
        disclosure.contains("build@box") && disclosure.contains("forjar drift"),
        "the prose disclosure is the one field a consumer reads; the census must \
         be folded into it, not published beside it only.\n{json:#}"
    );
}

/// A sealed plan file is reviewed later and applied by someone else. The
/// census travels with it.
#[test]
fn a_sealed_plan_file_carries_the_census() {
    let sb = Sandbox::new("planfile");
    sb.converge_locally();
    sb.move_remote();

    let out = sb.dir.join("plan.json");
    let text = sb.run(&["plan", "--out", out.to_str().expect("utf8 path")]);
    assert!(
        out.exists(),
        "precondition: the plan file was written.\n{text}"
    );
    let doc: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&out).expect("read plan file")).expect("json");
    let unprobed = doc["unprobed"]
        .as_array()
        .unwrap_or_else(|| panic!("the plan file must carry `unprobed`:\n{doc:#}"));
    assert_eq!(unprobed.len(), 1, "{doc:#}");
    assert_eq!(unprobed[0]["resource_id"], "build", "{doc:#}");
    assert_eq!(unprobed[0]["machine"], "box", "{doc:#}");
}

/// The census names SILENCE, not every unprobed resource. A task that will run
/// anyway — its config changed, so it plans `Update` — hides nothing, and
/// listing it would be the banner the forjar#342 contract forbids.
#[test]
fn a_task_that_will_run_anyway_is_not_named() {
    let sb = Sandbox::new("update");
    sb.converge_locally();
    sb.write_config(REMOTE, "cat src.txt src.txt > out.txt");

    let plan = sb.run(&["plan"]);
    assert!(
        plan.contains("1 to change"),
        "precondition: the config changed, so the task plans Update.\n{plan}"
    );
    assert!(
        !plan.contains("did not measure"),
        "a resource that is going to run is not a silent gap.\n{plan}"
    );
    let json = sb.run_json(&["plan", "--json"]);
    assert_eq!(
        json["unprobed"].as_array().map(Vec::len),
        Some(0),
        "{json:#}"
    );
}
