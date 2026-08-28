//! forjar#336: the apply summary must distinguish a config change from a drift
//! repair.
//!
//! Both printed as `converged`. Those are different events: the first is a
//! deploy the operator asked for, the second means something outside forjar
//! modified a managed resource — the difference between a deploy and an
//! intrusion, or a deploy and a unit that keeps resetting itself.
//!
//! The finding already existed. `check_pre_apply_drift` computed a
//! `Vec<DriftFinding>` per machine, spent each one on an `eprintln!` and a
//! `ResourceStatus::Drifted` write, and returned `Result<(), String>`. By the
//! time `count_results` and `print_apply_summary` ran, the only surviving facts
//! were three integers that cannot express WHY a resource converged.
//!
//! WHY THESE ASSERT ON JSON FIRST. The `drift:` lines go to STDERR and the JSON
//! report to stdout, so `forjar apply --json` gave a machine consumer ZERO
//! drift signal — a strictly worse form of the defect than the text mode the
//! issue describes. And this fleet reads the summary line instead of the host.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forjar() -> &'static str {
    env!("CARGO_BIN_EXE_forjar")
}

const DECLARED: &str = "DECLARED";

struct Sandbox {
    dir: tempfile::TempDir,
}

impl Sandbox {
    fn new() -> Self {
        Self {
            dir: tempfile::tempdir().expect("tempdir"),
        }
    }
    fn path(&self, rel: &str) -> PathBuf {
        self.dir.path().join(rel)
    }
    fn state(&self) -> PathBuf {
        self.path("state")
    }

    /// One config, `n` managed files with inline content.
    fn write_config(&self, ids: &[&str], content: &str) -> PathBuf {
        let cfg = self.path("forjar.yaml");
        let mut body = String::new();
        for id in ids {
            body.push_str(&format!(
                "  {id}: {{ type: file, machine: local, path: {}, content: \"{content}\\n\", mode: \"0644\" }}\n",
                self.path(&format!("{id}.txt")).display()
            ));
        }
        fs::write(
            &cfg,
            format!(
                "version: \"1.0\"\nname: drift-summary\n\
                 machines: {{ local: {{ hostname: localhost, addr: 127.0.0.1 }} }}\n\
                 resources:\n{body}"
            ),
        )
        .unwrap();
        cfg
    }

    fn run(&self, args: &[&str]) -> (String, String) {
        let out = Command::new(forjar())
            .args(args)
            .output()
            .expect("forjar failed to start");
        (
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }

    fn apply(&self, cfg: &Path, extra: &[&str]) -> (String, String) {
        let state = self.state();
        let mut args = vec![
            "apply",
            "-f",
            cfg.to_str().unwrap(),
            "--state-dir",
            state.to_str().unwrap(),
            "--yes",
        ];
        args.extend_from_slice(extra);
        self.run(&args)
    }

    /// Modify a managed file OUT OF BAND — the intrusion the summary must name.
    fn tamper(&self, id: &str) {
        fs::write(self.path(&format!("{id}.txt")), "TAMPERED\n").unwrap();
    }
}

/// Strip ANSI so assertions are about words, not colour codes.
fn plain(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for c2 in chars.by_ref() {
                if c2.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn parse_json(stdout: &str) -> serde_json::Value {
    let start = stdout.find('{').unwrap_or_else(|| {
        panic!("no JSON object in apply --json output:\n{stdout}");
    });
    serde_json::from_str(&stdout[start..])
        .unwrap_or_else(|e| panic!("apply --json emitted unparseable output ({e}):\n{stdout}"))
}

/// THE falsification. A convergence driven by observed drift must be named as
/// one, on the surface a machine reads.
#[test]
fn a_drift_repair_is_named_in_the_json_summary() {
    let sb = Sandbox::new();
    let cfg = sb.write_config(&["managed"], DECLARED);
    sb.apply(&cfg, &[]);
    sb.tamper("managed");

    let (stdout, _) = sb.apply(&cfg, &["--json"]);
    let v = parse_json(&stdout);

    assert_eq!(
        v["summary"]["drift_repaired_count"], 1,
        "the summary must count the repair:\n{stdout}"
    );
    assert_eq!(
        v["summary"]["drift_repaired"][0]["resource"], "managed",
        "the summary must name the repaired resource:\n{stdout}"
    );
}

/// THE CONTROL THAT KILLS THE WRONG FIX. Nothing drifted on the host; the
/// operator edited the config. That converges, and it is NOT a repair.
#[test]
fn an_ordinary_config_change_is_not_reported_as_drift() {
    let sb = Sandbox::new();
    let cfg = sb.write_config(&["managed"], DECLARED);
    sb.apply(&cfg, &[]);

    // Change the DECLARED state, not the host.
    let cfg = sb.write_config(&["managed"], "REDECLARED");
    let (stdout, _) = sb.apply(&cfg, &["--json"]);
    let v = parse_json(&stdout);

    assert_eq!(
        v["summary"]["total_converged"], 1,
        "the config change must still converge:\n{stdout}"
    );
    assert_eq!(
        v["summary"]["drift_repaired_count"], 0,
        "a config change is not a drift repair:\n{stdout}"
    );
}

/// The operator-facing half from the issue body.
#[test]
fn the_text_summary_names_the_repaired_resource() {
    let sb = Sandbox::new();
    let cfg = sb.write_config(&["managed"], DECLARED);
    sb.apply(&cfg, &[]);
    sb.tamper("managed");

    let (stdout, stderr) = sb.apply(&cfg, &[]);
    let out = plain(&format!("{stdout}{stderr}"));

    assert!(
        out.contains("repaired drift"),
        "the summary line must say a repair happened:\n{out}"
    );
    assert!(
        out.contains("drift-repaired:") && out.contains("managed"),
        "the repaired resource must be named under the summary:\n{out}"
    );
}

/// PINS THE INTERSECTION. Two files drift; the apply is scoped to one. A naive
/// `observed.len()` implementation passes every test above and fails here.
#[test]
fn an_unrepaired_drift_is_not_counted_as_repaired() {
    let sb = Sandbox::new();
    let cfg = sb.write_config(&["alpha", "beta"], DECLARED);
    sb.apply(&cfg, &[]);
    sb.tamper("alpha");
    sb.tamper("beta");

    let (stdout, _) = sb.apply(&cfg, &["--json", "-r", "alpha"]);
    let v = parse_json(&stdout);

    assert_eq!(
        v["summary"]["drift_repaired_count"], 1,
        "only the resource this run converged was repaired:\n{stdout}"
    );
    assert_eq!(
        v["summary"]["drift_repaired"][0]["resource"], "alpha",
        "the wrong resource was claimed as repaired:\n{stdout}"
    );
    // beta is still drifted on the host — claiming it would be a lie the
    // operator would then not go and fix.
    assert_eq!(
        fs::read_to_string(sb.path("beta.txt")).unwrap(),
        "TAMPERED\n"
    );
}

/// PINS THE EMPTY-CLAUSE RULE. With nothing repaired the summary must be
/// byte-identical to what it was, because other suites assert exact substrings
/// of it (`contains("0 converged")`, `contains("1 unchanged")`).
#[test]
fn a_converged_stack_prints_the_same_summary_as_before() {
    let sb = Sandbox::new();
    let cfg = sb.write_config(&["managed"], DECLARED);
    sb.apply(&cfg, &[]);

    let (stdout, stderr) = sb.apply(&cfg, &[]);
    let out = plain(&format!("{stdout}{stderr}"));

    assert!(
        out.contains("0 converged, 1 unchanged"),
        "an unremarkable apply must keep its exact wording:\n{out}"
    );
    assert!(
        !out.contains("repaired drift"),
        "nothing drifted, so nothing may claim a repair:\n{out}"
    );
}
