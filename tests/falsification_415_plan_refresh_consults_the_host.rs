//! forjar#415: `plan` is lock-relative and cannot consult a host.
//!
//! A file edited behind forjar's back is invisible to `plan`, which compares
//! the config to the lock and says so (forjar#342). `plan --refresh` runs the
//! drift detectors first, so the edit must show up as a change WITHOUT an
//! intervening `apply` or `drift` — and without writing the lock, because a
//! plan is a read.
//!
//! Every assertion is on `plan --json` counts and the lock bytes on disk, never
//! on prose alone: a refresh that printed "Refreshed 1 machine(s)" and planned
//! nothing would pass a text check.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

struct Sandbox {
    dir: tempfile::TempDir,
}

impl Sandbox {
    fn new() -> Self {
        let s = Self {
            dir: tempfile::tempdir().expect("tempdir"),
        };
        fs::write(
            s.config(),
            format!(
                "version: \"1.0\"\nname: refresh\nmachines: {{ local: {{ hostname: localhost, addr: 127.0.0.1 }} }}\nresources:\n  managed: {{ type: file, machine: local, path: {}, content: \"DECLARED\\n\", mode: \"0644\" }}\n",
                s.target().display()
            ),
        )
        .unwrap();
        s
    }
    fn path(&self, rel: &str) -> PathBuf {
        self.dir.path().join(rel)
    }
    fn config(&self) -> PathBuf {
        self.path("forjar.yaml")
    }
    fn target(&self) -> PathBuf {
        self.path("target.txt")
    }
    fn state(&self) -> PathBuf {
        self.path("state")
    }
    fn run(&self, verb: &str, extra: &[&str]) -> (i32, String, String) {
        let out = Command::new(env!("CARGO_BIN_EXE_forjar"))
            .arg(verb)
            .args(["-f", self.config().to_str().unwrap()])
            .args(["--state-dir", self.state().to_str().unwrap()])
            .args(extra)
            .output()
            .expect("forjar failed to start");
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }
    fn plan_json(&self, extra: &[&str]) -> serde_json::Value {
        let mut args = vec!["--json"];
        args.extend_from_slice(extra);
        let (code, stdout, stderr) = self.run("plan", &args);
        assert_eq!(code, 0, "plan {extra:?} failed: {stdout}{stderr}");
        let start = stdout.find('{').expect("plan --json printed no object");
        serde_json::from_str(&stdout[start..]).unwrap_or_else(|e| panic!("{e}: {stdout}"))
    }
    /// Every byte under the state dir, keyed by path.
    fn state_bytes(&self) -> Vec<(PathBuf, Vec<u8>)> {
        fn walk(dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
            for e in fs::read_dir(dir).unwrap() {
                let p = e.unwrap().path();
                if p.is_dir() {
                    walk(&p, out);
                } else {
                    out.push((p.clone(), fs::read(&p).unwrap()));
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.state(), &mut out);
        out.sort();
        out
    }
    fn converged() -> Self {
        let s = Self::new();
        let (code, stdout, stderr) = s.run("apply", &["--yes"]);
        assert_eq!(code, 0, "apply failed: {stdout}{stderr}");
        assert_eq!(fs::read_to_string(s.target()).unwrap(), "DECLARED\n");
        s
    }
}

#[test]
fn falsify_415_refresh_plans_a_file_edited_behind_forjars_back() {
    let s = Sandbox::converged();
    fs::write(s.target(), "EDITED ON THE HOST\n").unwrap();
    let before = s.state_bytes();

    let plain = s.plan_json(&[]);
    assert_eq!(plain["to_update"], 0, "control: plain plan reads the lock");
    assert_eq!(plain["lock_relative"], true);

    let refreshed = s.plan_json(&["--refresh"]);
    assert_eq!(
        refreshed["to_update"], 1,
        "--refresh did not plan the drifted file: {refreshed}"
    );
    assert_eq!(refreshed["lock_relative"], false, "{refreshed}");
    assert_eq!(refreshed["refresh"]["drifted"], 1, "{refreshed}");
    assert_eq!(refreshed["unconsulted_observations"], 0, "{refreshed}");
    assert!(
        refreshed["changes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["resource_id"] == "managed"),
        "{refreshed}"
    );

    assert_eq!(s.state_bytes(), before, "plan --refresh wrote state");
    assert_eq!(
        fs::read_to_string(s.target()).unwrap(),
        "EDITED ON THE HOST\n",
        "plan --refresh changed the host"
    );
}

#[test]
fn falsify_415_refresh_of_a_converged_host_plans_nothing() {
    let s = Sandbox::converged();
    let refreshed = s.plan_json(&["--refresh"]);
    assert_eq!(refreshed["to_update"], 0, "{refreshed}");
    assert_eq!(refreshed["to_create"], 0, "{refreshed}");
    assert_eq!(refreshed["refresh"]["drifted"], 0, "{refreshed}");
    assert_eq!(refreshed["lock_relative"], false, "{refreshed}");
}

#[test]
fn falsify_415_refresh_text_does_not_claim_it_stayed_off_the_host() {
    let s = Sandbox::converged();
    fs::write(s.target(), "EDITED ON THE HOST\n").unwrap();
    let (code, stdout, stderr) = s.run("plan", &["--refresh"]);
    assert_eq!(code, 0, "{stdout}{stderr}");
    let stdout = plain(&stdout);
    assert!(stdout.contains("1 to change"), "{stdout}");
    assert!(
        !stdout.contains("did not\ncontact any machine"),
        "a refreshed plan printed the lock-relative disclosure: {stdout}"
    );
    assert!(
        stdout.contains("1 resource(s) measured as drifted"),
        "{stdout}"
    );
}

#[test]
fn falsify_415_refresh_refuses_to_seal_a_plan_file() {
    let s = Sandbox::converged();
    let out = s.path("plan.json");
    let (code, stdout, stderr) = s.run("plan", &["--refresh", "--out", out.to_str().unwrap()]);
    assert_ne!(code, 0, "{stdout}");
    assert!(stderr.contains("plan --refresh --out"), "{stderr}");
    assert!(!out.exists(), "a refreshed plan file was written");
}

#[test]
fn falsify_415_refresh_counts_drift_the_lock_already_recorded() {
    // An earlier apply's pre-apply drift check can leave `Drifted` in the
    // lock. A refresh that counted only the statuses it flipped would report
    // `drifted: 0` for a host it had just measured as changed. Seeded through
    // the one writer, `save_lock`, so the lock's seal stays valid.
    use forjar::core::state::{load_lock, save_lock};
    use forjar::core::types::ResourceStatus;
    let s = Sandbox::converged();
    fs::write(s.target(), "EDITED ON THE HOST\n").unwrap();
    let mut lock = load_lock(&s.state(), "local").unwrap().expect("lock");
    lock.resources.get_mut("managed").expect("entry").status = ResourceStatus::Drifted;
    save_lock(&s.state(), &lock).unwrap();
    let refreshed = s.plan_json(&["--refresh"]);
    assert_eq!(refreshed["to_update"], 1, "{refreshed}");
    assert_eq!(refreshed["refresh"]["drifted"], 1, "{refreshed}");
}

#[test]
fn falsify_415_refresh_discloses_what_it_could_not_measure() {
    // A host that never answers is UNMEASURED (forjar#549), not drift: the
    // entry is planned from the lock alone, counted as unconsulted, and the
    // refreshed disclosure says so and names `forjar drift`. The machine is
    // at TEST-NET-3, the address forjar#407 and #549 already use.
    use forjar::core::state::{load_lock, save_lock};
    let dir = tempfile::tempdir().expect("tempdir");
    let bait = dir.path().join("on-controller.txt");
    fs::write(&bait, "controller copy").unwrap();
    let hash = format!("blake3:{}", blake3::hash(b"controller copy").to_hex());
    let cfg = dir.path().join("forjar.yaml");
    fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: e05\nmachines:\n  web:\n    hostname: web\n\
             \x20   addr: 203.0.113.9\n    user: root\nresources:\n  conf:\n    type: file\n\
             \x20   machine: web\n    path: {}\n    content: \"controller copy\"\n",
            bait.display()
        ),
    )
    .unwrap();
    let state = dir.path().join("state");
    fs::create_dir_all(state.join("web")).unwrap();
    fs::write(
        state.join("web/state.lock.yaml"),
        format!(
            "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: now\n\
             generator: test\nblake3_version: \"1\"\nresources:\n  conf:\n    type: file\n\
             \x20   status: converged\n    hash: \"h\"\n    details:\n      path: \"{}\"\n\
             \x20     content_hash: \"{hash}\"\n",
            bait.display()
        ),
    )
    .unwrap();
    // Re-save through the one writer so the lock carries a valid seal.
    let lock = load_lock(&state, "web").unwrap().expect("lock");
    save_lock(&state, &lock).unwrap();
    let before = fs::read(state.join("web/state.lock.yaml")).unwrap();

    let plan = |extra: &[&str]| -> serde_json::Value {
        let out = Command::new(env!("CARGO_BIN_EXE_forjar"))
            .args(["plan", "--json", "-f", cfg.to_str().unwrap()])
            .args(["--state-dir", state.to_str().unwrap()])
            .args(extra)
            .output()
            .expect("forjar failed to start");
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        assert_eq!(out.status.code(), Some(0), "{stdout}");
        serde_json::from_str(&stdout[stdout.find('{').unwrap()..]).unwrap()
    };
    let plain = plan(&[]);
    let v = plan(&["--refresh"]);
    assert_eq!(v["lock_relative"], false, "{v}");
    assert_eq!(v["refresh"]["drifted"], 0, "unmeasured read as drift: {v}");
    assert_eq!(v["unconsulted_observations"], 1, "{v}");
    // Planned from the lock alone: exactly what the lock-relative plan plans.
    assert_eq!(v["changes"], plain["changes"], "{v}\n{plain}");
    let disclosure = v["disclosure"].as_str().unwrap_or_default();
    // The entry carries no observed state (no `live_hash`), only a locked
    // content hash: the disclosure must not claim otherwise.
    assert!(
        disclosure.contains("1 locked resource(s)\nwere not measured (the target did not answer"),
        "{v}"
    );
    assert!(!disclosure.contains("observed state"), "{v}");
    assert!(disclosure.contains("`forjar drift`"), "{v}");
    assert_eq!(
        fs::read(state.join("web/state.lock.yaml")).unwrap(),
        before,
        "plan --refresh wrote the lock"
    );
}

/// Strip ANSI so assertions do not depend on colour.
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
