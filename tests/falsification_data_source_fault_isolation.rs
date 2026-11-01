//! PMAT-828 (forjar#828): one failing `data:` source must skip only the
//! resources that reference it, not fail the whole plan or apply.
//!
//! The assertions are on the files the run wrote, the exit status and the
//! output naming the failed source: the unrelated resource's file must exist,
//! the dependent's must not, and the run must still exit non-zero so the
//! failure is never silent. Only `apply` is driven here; `plan --out` (refuses to seal) is covered
//! by the in-crate test `plan_out_refuses_to_seal_when_a_data_source_failed`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forjar() -> &'static str {
    env!("CARGO_BIN_EXE_forjar")
}

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
    fn config(&self) -> PathBuf {
        let cfg = self.path("forjar.yaml");
        let body = format!(
            "version: \"1.0\"\nname: iso828\n\
machines: {{ local: {{ hostname: localhost, addr: 127.0.0.1 }} }}\n\
data:\n\x20 broken: {{ type: command, value: \"exit 127\" }}\n\
resources:\n\
\x20 unrelated: {{ type: file, machine: local, path: {}, content: \"fine\\n\" }}\n\
\x20 dependent: {{ type: file, machine: local, path: {}, content: \"v={{{{data.broken}}}}\\n\" }}\n\
\x20 downstream: {{ type: file, machine: local, path: {}, content: \"x\\n\", depends_on: [dependent] }}\n",
            self.path("unrelated.txt").display(),
            self.path("dependent.txt").display(),
            self.path("downstream.txt").display(),
        );
        fs::write(&cfg, body).unwrap();
        cfg
    }
    fn run(&self, verb: &str, cfg: &Path) -> (bool, String) {
        let state = self.path("state");
        let mut args = vec![verb, "-f", cfg.to_str().unwrap(), "--state-dir"];
        args.push(state.to_str().unwrap());
        if verb == "apply" {
            args.push("--yes");
        }
        let out = Command::new(forjar())
            .args(&args)
            .output()
            .expect("forjar failed to start");
        let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
        s.push_str(&String::from_utf8_lossy(&out.stderr));
        (out.status.success(), s)
    }
}

/// The failing source skips its dependents and nothing else; the exit is still
/// non-zero and the output names the source.
#[test]
fn apply_writes_unrelated_resources_when_one_data_source_fails() {
    let sb = Sandbox::new();
    let cfg = sb.config();
    let (ok, out) = sb.run("apply", &cfg);
    assert!(!ok, "a failed data source must still fail the run:\n{out}");
    assert!(
        out.contains("broken"),
        "output must name the source:\n{out}"
    );
    assert_eq!(
        fs::read_to_string(sb.path("unrelated.txt")).unwrap_or_default(),
        "fine\n",
        "unrelated resource must be applied:\n{out}"
    );
    assert!(
        !sb.path("dependent.txt").exists(),
        "dependent of the failed source must be skipped:\n{out}"
    );
    assert!(
        !sb.path("downstream.txt").exists(),
        "depends_on the skipped resource must be skipped:\n{out}"
    );
}

/// Negative control: with a healthy source the dependent is applied and the
/// run succeeds, so the fix is not "always skip templated resources".
#[test]
fn apply_writes_the_dependent_when_the_data_source_succeeds() {
    let sb = Sandbox::new();
    let cfg = sb.config();
    let fixed = fs::read_to_string(&cfg)
        .unwrap()
        .replace("exit 127", "echo ok");
    fs::write(&cfg, fixed).unwrap();
    let (ok, out) = sb.run("apply", &cfg);
    assert!(ok, "healthy sources must apply cleanly:\n{out}");
    assert_eq!(
        fs::read_to_string(sb.path("dependent.txt")).unwrap(),
        "v=ok\n",
        "{out}"
    );
}

/// A skipped resource is skipped, not forgotten: a later outage of its data
/// source must leave the file it converged earlier and its lock entry alone.
#[test]
fn a_later_data_source_outage_does_not_forget_a_converged_dependent() {
    let sb = Sandbox::new();
    let cfg = sb.config();
    let healthy = fs::read_to_string(&cfg).unwrap();
    fs::write(&cfg, healthy.replace("exit 127", "echo ok")).unwrap();
    let (ok, out) = sb.run("apply", &cfg);
    assert!(ok, "first apply:\n{out}");
    fs::write(&cfg, healthy).unwrap();
    let (ok, out) = sb.run("apply", &cfg);
    assert!(!ok, "outage must fail the run:\n{out}");
    assert_eq!(
        fs::read_to_string(sb.path("dependent.txt")).unwrap(),
        "v=ok\n"
    );
    let lock = fs::read_to_string(sb.path("state/local/state.lock.yaml")).unwrap_or_default();
    assert!(
        lock.contains("dependent"),
        "lock entry forgotten:\n{lock}\n{out}"
    );
}
