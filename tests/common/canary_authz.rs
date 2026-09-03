//! forjar#374: the two-machine fleet fixture and the refusal assertions shared by
//! `falsification_canary_apply_is_authorized` and its `_b` binary.
#![allow(dead_code)]
use std::path::PathBuf;
use std::process::{Command, Output};

pub const FORJAR: &str = env!("CARGO_BIN_EXE_forjar");

pub struct Sandbox {
    pub dir: PathBuf,
}

impl Sandbox {
    /// `machines` is `(machine, allowed_operators)`; an empty operator list
    /// restricts nobody on that machine.
    pub fn new(name: &str, machines: &[(&str, &[&str])]) -> Self {
        // Per-process: two concurrent runs of this suite on one box would
        // otherwise share `forjar-374-<name>`, and each `Drop` deletes the
        // other's fixture out from under it mid-run.
        let dir = std::env::temp_dir().join(format!("forjar-374-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("sandbox");
        let me = Self { dir };
        me.write_config(machines);
        me
    }

    /// The default two-machine fleet: both restricted to `alice`.
    pub fn fleet(name: &str) -> Self {
        Self::new(name, &[("sandbox", &["alice"]), ("prod", &["alice"])])
    }

    pub fn cfg(&self) -> PathBuf {
        self.dir.join("forjar.yaml")
    }

    pub fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    pub fn canary_file(&self) -> PathBuf {
        self.dir.join("canary.txt")
    }

    pub fn prod_file(&self) -> PathBuf {
        self.dir.join("prod.txt")
    }

    pub fn write_config(&self, machines: &[(&str, &[&str])]) {
        let d = self.dir.display();
        let mut yaml = String::from("version: \"1.0\"\nname: canary-authz\nmachines:\n");
        for (m, ops) in machines {
            yaml.push_str(&format!("  {m}:\n    hostname: {m}\n    addr: 127.0.0.1\n"));
            if !ops.is_empty() {
                yaml.push_str("    allowed_operators:\n");
                for o in *ops {
                    yaml.push_str(&format!("      - {o}\n"));
                }
            }
        }
        yaml.push_str(&format!(
            "resources:\n  \
             canary_file:\n    type: file\n    machine: sandbox\n    \
             path: {d}/canary.txt\n    content: \"canary\"\n  \
             prod_file:\n    type: file\n    machine: prod\n    \
             path: {d}/prod.txt\n    content: \"prod\"\n"
        ));
        std::fs::write(self.cfg(), yaml).expect("config");
    }

    pub fn run(&self, args: &[&str]) -> Output {
        Command::new(FORJAR)
            .args(args)
            .current_dir(&self.dir)
            .output()
            .expect("run forjar")
    }

    /// `forjar apply -f <cfg> --state-dir <sd> <extra…>`, plus `--operator` when given.
    pub fn apply(&self, operator: Option<&str>, extra: &[&str]) -> Output {
        let cfg = self.cfg();
        let sd = self.state();
        let mut args = vec![
            "apply",
            "-f",
            cfg.to_str().unwrap(),
            "--state-dir",
            sd.to_str().unwrap(),
        ];
        args.extend_from_slice(extra);
        if let Some(op) = operator {
            args.push("--operator");
            args.push(op);
        }
        self.run(&args)
    }

    /// Like `apply`, but with `input` on stdin — for the confirmation prompts,
    /// which read one line each and treat EOF as "no".
    pub fn apply_with_stdin(&self, operator: Option<&str>, extra: &[&str], input: &str) -> Output {
        use std::io::Write;
        let cfg = self.cfg();
        let sd = self.state();
        let mut args = vec![
            "apply",
            "-f",
            cfg.to_str().unwrap(),
            "--state-dir",
            sd.to_str().unwrap(),
        ];
        args.extend_from_slice(extra);
        if let Some(op) = operator {
            args.push("--operator");
            args.push(op);
        }
        let mut child = Command::new(FORJAR)
            .args(&args)
            .current_dir(&self.dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("spawn forjar");
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(input.as_bytes())
            .expect("write stdin");
        child.wait_with_output().expect("wait forjar")
    }

    pub fn canary(&self, operator: Option<&str>, extra: &[&str]) -> Output {
        let mut args = vec!["--canary-machine", "sandbox"];
        args.extend_from_slice(extra);
        self.apply(operator, &args)
    }

    pub fn nothing_was_written(&self) -> bool {
        !self.canary_file().exists() && !self.prod_file().exists()
    }

    pub fn reset_targets(&self) {
        let _ = std::fs::remove_file(self.canary_file());
        let _ = std::fs::remove_file(self.prod_file());
        let _ = std::fs::remove_dir_all(self.state());
    }

    /// Every file under the state dir, with its modification time. `--refresh-only`
    /// calls `state::save_lock` unconditionally, so a refusal must leave this equal.
    pub fn state_fingerprint(&self) -> Vec<(PathBuf, std::time::SystemTime, u64)> {
        let mut out = Vec::new();
        let Ok(rd) = std::fs::read_dir(self.state()) else {
            return out;
        };
        for e in rd.flatten() {
            if let Ok(md) = e.metadata() {
                if md.is_file() {
                    out.push((e.path(), md.modified().expect("mtime"), md.len()));
                }
            }
        }
        out.sort();
        out
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

pub fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

pub fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

pub fn refused(o: &Output, what: &str) {
    assert!(
        !o.status.success(),
        "{what} exited {:?}; the operator gate was never reached.\nstdout: {}",
        o.status.code(),
        stdout(o)
    );
    assert!(
        stderr(o).contains("not authorized"),
        "{what} must give the SAME refusal the ordinary path gives.\nstderr: {}",
        stderr(o)
    );
}
