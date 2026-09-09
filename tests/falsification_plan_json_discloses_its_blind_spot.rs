//! forjar#342, residual: the machine-readable plan surfaces still presented a
//! lock diff as the state of the world.
//!
//! Step 1 of the RFC landed in 1.20.0 for the TTY rendering only, because the
//! disclosure was implemented as a side-effecting printer:
//! `print_scope_disclosure` formatted the sentence and immediately `println!`d
//! it, returning `()`. With no value to serialise, `print_plan_json` could not
//! carry it and `PlanOutput` had nothing to attach.
//!
//! That inverts the issue's own threat model. #342's motivating incident is
//! machine-driven — a nightly lane parsing forjar output, and the "52 changes"
//! figure quoted from the blind command. The consumers that CANNOT notice a
//! missing disclosure (a CI parser, an MCP agent reading `to_update: 0`) were
//! the ones still receiving the undisclosed diff; the human at a terminal, who
//! at least has `forjar drift` in muscle memory, was the only one told.
//!
//! `src/verb/registry.rs` routes the CLI `verb call`, MCP stdio and HTTP
//! transports through one `PlanOutput`, so test 2 covers all three.
//!
//! Every assertion spawns the real binary, per the house rule in
//! `tests/e2e_verb_surface_t.rs`.

use std::fs;
use std::process::Command;

const FORJAR: &str = env!("CARGO_BIN_EXE_forjar");

struct Sandbox {
    dir: std::path::PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("forjar-342-json-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("sandbox");
        let sb = Self { dir };
        sb.write_config();
        sb
    }

    fn cfg(&self) -> std::path::PathBuf {
        self.dir.join("forjar.yaml")
    }

    fn write_config(&self) {
        let cfg = format!(
            "version: \"1.0\"\nname: blind-spot\nmachines:\n  sandbox:\n    hostname: sandbox\n\
             \x20   addr: 127.0.0.1\nresources:\n  a-file:\n    type: file\n\
             \x20   machine: sandbox\n    path: {}\n    content: \"declared\"\n",
            self.dir.join("managed.txt").display()
        );
        fs::write(self.cfg(), cfg).expect("config");
    }

    fn run(&self, args: &[&str]) -> String {
        let out = Command::new(FORJAR)
            .args(args)
            .current_dir(&self.dir)
            .output()
            .expect("run forjar");
        assert!(
            out.status.success(),
            "forjar {args:?} exited {:?}\nstderr: {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    fn apply(&self) {
        self.run(&["apply", "--yes", "-f", self.cfg().to_str().unwrap()]);
    }

    fn plan_json(&self) -> serde_json::Value {
        parse(&self.run(&["plan", "--json", "-f", self.cfg().to_str().unwrap()]))
    }

    /// forjar#497: a task whose declared build I/O lives on `addr`. Converged
    /// on loopback, the lock holds real input/output hashes; moved to a
    /// TEST-NET address this host cannot answer for, the probe can no longer be
    /// taken and the machine name is not part of the resource hash, so the plan
    /// still says `unchanged` and the census is the only thing that says why.
    fn write_task_config(&self, addr: &str) {
        let cfg = format!(
            "version: \"1.0\"\nname: blind-spot\nmachines:\n  box:\n    hostname: box\n\
             \x20   addr: {addr}\nresources:\n  build:\n    type: task\n    machine: box\n\
             \x20   command: \"cat src.txt > out.txt\"\n    working_dir: {}\n\
             \x20   task_inputs: [src.txt]\n    output_artifacts: [out.txt]\n",
            self.dir.display()
        );
        fs::write(self.cfg(), cfg).expect("config");
        fs::write(self.dir.join("src.txt"), "v1\n").expect("input");
    }

    /// The same plan through the unified verb surface — which is also MCP stdio
    /// and HTTP, since all three serialise one `PlanOutput`.
    fn verb_plan(&self) -> serde_json::Value {
        let input = serde_json::json!({
            "path": self.cfg().to_string_lossy(),
            "state_dir": self.dir.join("state").to_string_lossy(),
        })
        .to_string();
        parse(&self.run(&["verb", "call", "plan", "--json", &input]))
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn parse(stdout: &str) -> serde_json::Value {
    let start = stdout
        .find('{')
        .unwrap_or_else(|| panic!("no JSON object in output:\n{stdout}"));
    serde_json::from_str(&stdout[start..])
        .unwrap_or_else(|e| panic!("unparseable JSON ({e}):\n{stdout}"))
}

/// THE DEFECT, on the surface a CI lane actually reads.
#[test]
fn json_plan_discloses_that_it_did_not_look_at_the_host() {
    let sb = Sandbox::new("cli");
    sb.apply();

    let v = sb.plan_json();

    assert_eq!(
        v["lock_relative"], true,
        "plan --json must state that it compares against the lock:\n{v:#}"
    );
    assert!(
        v["unconsulted_observations"].as_u64().unwrap_or(0) > 0,
        "a converged stack holds observed state this plan did not consult:\n{v:#}"
    );
    assert!(
        v["disclosure"]
            .as_str()
            .unwrap_or_default()
            .contains("forjar drift"),
        "the disclosure must name the command that CAN answer:\n{v:#}"
    );
}

/// The same three properties through the verb surface — and therefore through
/// MCP stdio and HTTP, which share the one `PlanOutput`.
#[test]
fn the_verb_surface_discloses_the_same_blind_spot() {
    let sb = Sandbox::new("verb");
    sb.apply();

    let v = sb.verb_plan();

    assert_eq!(v["lock_relative"], true, "{v:#}");
    assert!(
        v["unconsulted_observations"].as_u64().unwrap_or(0) > 0,
        "{v:#}"
    );
    assert!(
        v["disclosure"]
            .as_str()
            .unwrap_or_default()
            .contains("forjar drift"),
        "{v:#}"
    );
}

/// PINS BOTH DIRECTIONS OF THE BICONDITIONAL. With no lock there is nothing to
/// be blind to, so the PROSE disclosure is withheld — but the COUNT stays a
/// total function, so a parser can tell "nothing observed" from "old binary".
///
/// Deliberately not a pure absence assertion: an absence-only test passes
/// vacuously against the defect and proves nothing.
#[test]
fn a_json_plan_with_no_lock_reports_zero_rather_than_omitting_the_field() {
    let sb = Sandbox::new("nolock");

    let v = sb.plan_json();

    assert_eq!(
        v["unconsulted_observations"], 0,
        "the count must be PRESENT and zero, not absent:\n{v:#}"
    );
    assert!(
        v.get("disclosure").is_none(),
        "with no observed state an unconditional banner is noise:\n{v:#}"
    );
    assert_eq!(v["lock_relative"], true, "{v:#}");
}

/// CLI/MCP parity for the new field. The sibling defect class is already on
/// record: `forjar_plan`'s counts once disagreed with the array it shipped
/// beside.
#[test]
fn cli_json_and_the_verb_surface_agree_on_the_blind_spot() {
    let sb = Sandbox::new("parity");
    sb.apply();

    let cli = sb.plan_json();
    let verb = sb.verb_plan();

    // Both PRESENT and non-zero first: two Nulls are equal, and an equality-only
    // assertion would pass vacuously against the very defect this pins.
    let n = cli["unconsulted_observations"]
        .as_u64()
        .unwrap_or_else(|| panic!("plan --json carries no count:\n{cli:#}"));
    assert!(n > 0, "precondition: a converged stack holds observations");
    assert_eq!(
        verb["unconsulted_observations"]
            .as_u64()
            .unwrap_or_else(|| panic!("the verb surface carries no count:\n{verb:#}")),
        n,
        "the two surfaces must range over the same locks\ncli: {cli:#}\nverb: {verb:#}"
    );
    assert_eq!(cli["disclosure"], verb["disclosure"]);
    assert!(cli["disclosure"].is_string(), "{cli:#}");
}

/// forjar#497, on the surface with no human to notice. The verb/MCP/HTTP
/// `PlanOutput` must carry the unprobed census as a TOTAL list and fold it into
/// the one `disclosure` an agent reads — otherwise an agent asked "is this
/// stack converged?" is handed `to_update: 0` over a resource nothing measured.
#[test]
fn the_verb_surface_carries_the_census() {
    let sb = Sandbox::new("census");
    sb.write_task_config("127.0.0.1");
    sb.apply();
    assert!(
        sb.dir.join("out.txt").exists(),
        "precondition: the task must have run locally"
    );
    sb.write_task_config("203.0.113.7");

    let v = sb.verb_plan();

    let census = v["unprobed"]
        .as_array()
        .unwrap_or_else(|| panic!("the verb surface must carry `unprobed`:\n{v:#}"));
    assert_eq!(census.len(), 1, "one converged task, one entry:\n{v:#}");
    assert_eq!(census[0]["resource_id"], "build", "{v:#}");
    assert_eq!(census[0]["machine"], "box", "{v:#}");
    assert!(
        census[0]["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("box"),
        "the reason names the machine this host does not answer for:\n{v:#}"
    );
    let disclosure = v["disclosure"].as_str().unwrap_or_default();
    assert!(
        disclosure.contains("build@box") && disclosure.contains("forjar drift"),
        "the census must be folded into the one field an agent reads:\n{v:#}"
    );
}
