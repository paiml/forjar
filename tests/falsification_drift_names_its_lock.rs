//! forjar#585: a drift verdict names the lock it read.
//!
//! Three callers passing three `--state-dir`s got 2, 3 and 9 DRIFTED from "the
//! same" command with no way to tell they had read three files. Two lock trees
//! holding different entries for one machine must give outputs that differ in
//! the named path, in both human and `--json` form.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const FORJAR: &str = env!("CARGO_BIN_EXE_forjar");

fn run(args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(FORJAR)
        .args(args)
        .arg("--no-color")
        .output()
        .expect("forjar must run");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn converge(dir: &Path, state: &Path, file: &str) -> PathBuf {
    let target = dir.join(file);
    let cfg = dir.join(format!("{file}.yaml"));
    fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: s\nmachines:\n  box:\n    hostname: box\n    addr: 127.0.0.1\nresources:\n  r:\n    type: file\n    machine: box\n    path: {}\n    content: \"x\\n\"\n",
            target.display()
        ),
    )
    .unwrap();
    let (code, out, err) = run(&[
        "apply",
        "-f",
        cfg.to_str().unwrap(),
        "--state-dir",
        state.to_str().unwrap(),
        "--yes",
    ]);
    assert_eq!(code, Some(0), "{out}\n{err}");
    cfg
}

#[test]
fn two_lock_trees_for_one_machine_give_outputs_that_differ_in_the_named_path() {
    let dir = tempfile::tempdir().unwrap();
    let (s1, s2) = (dir.path().join("state-one"), dir.path().join("state-two"));
    let cfg1 = converge(dir.path(), &s1, "one.txt");
    let cfg2 = converge(dir.path(), &s2, "two.txt");

    let mut outs = Vec::new();
    for (cfg, state) in [(&cfg1, &s1), (&cfg2, &s2)] {
        let (code, out, err) = run(&[
            "drift",
            "-f",
            cfg.to_str().unwrap(),
            "--state-dir",
            state.to_str().unwrap(),
        ]);
        assert_eq!(code, Some(0), "{out}\n{err}");
        let want = fs::canonicalize(state.join("box").join("state.lock.yaml")).unwrap();
        assert!(
            out.contains(want.to_str().unwrap()),
            "the verdict must name the lock it read ({}):\n{out}",
            want.display()
        );
        outs.push(out);
    }
    assert_ne!(outs[0], outs[1], "the two outputs must differ in the path");

    // --json carries it too.
    let (code, out, err) = run(&[
        "drift",
        "-f",
        cfg1.to_str().unwrap(),
        "--state-dir",
        s1.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(code, Some(0), "{out}\n{err}");
    let want = fs::canonicalize(s1.join("box").join("state.lock.yaml")).unwrap();
    assert!(
        out.contains(&format!("\"lock\": \"{}\"", want.display()))
            || out.contains(&format!("\"lock\":\"{}\"", want.display())),
        "json census must carry the lock path:\n{out}"
    );
}
