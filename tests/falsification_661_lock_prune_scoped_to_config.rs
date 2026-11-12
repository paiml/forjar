//! forjar#661: `lock-prune -f <config>` prunes only what `<config>` answers for.
//!
//! WHAT WAS WRONG. `lock-prune` walked every machine directory of `--state-dir`
//! and treated every lock key absent from the ONE `-f` config as stale. With two
//! stacks sharing a state dir it deleted the other stack's entries, including
//! on a machine the config never declared; the next `plan -f stack-a` then
//! wanted to re-create two files that were already on disk.
//!
//! The BINARY is driven (apply, apply, prune), so the stamps the guard reads are
//! the ones a real apply wrote. Reverting the fix turns the first test red.

use std::path::Path;
use std::process::Command;

fn forjar() -> &'static str {
    env!("CARGO_BIN_EXE_forjar")
}

fn stack(name: &str, machines: &[&str], resources: &[(&str, &str)], out: &Path) -> String {
    let mut y = format!("version: \"1.0\"\nname: {name}\nmachines:\n");
    for m in machines {
        y += &format!("  {m}:\n    hostname: {m}\n    addr: 127.0.0.1\n");
    }
    y += "resources:\n";
    for (id, machine) in resources {
        y += &format!(
            "  {id}:\n    type: file\n    machine: {machine}\n    path: {}/{id}.txt\n    content: \"{id}\\n\"\n",
            out.display()
        );
    }
    y
}

fn fj(args: &[&str], dir: &Path) -> (bool, String) {
    let out = Command::new(forjar())
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run forjar");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), text)
}

fn lock(dir: &Path, machine: &str) -> String {
    std::fs::read_to_string(dir.join("st").join(machine).join("state.lock.yaml"))
        .expect("machine lock")
}

#[test]
fn prune_from_one_stack_leaves_the_other_stacks_locks_alone() {
    let t = tempfile::tempdir().expect("tempdir");
    let d = t.path();
    let out = d.join("out");
    std::fs::create_dir_all(&out).expect("out");
    std::fs::write(
        d.join("a.yaml"),
        stack(
            "stack-a",
            &["shared", "only-a"],
            &[("a-on-shared", "shared"), ("a-on-only-a", "only-a")],
            &out,
        ),
    )
    .expect("a");
    std::fs::write(
        d.join("b.yaml"),
        stack("stack-b", &["shared"], &[("b-on-shared", "shared")], &out),
    )
    .expect("b");
    for f in ["a.yaml", "b.yaml"] {
        let (ok, text) = fj(&["apply", "--yes", "-f", f, "--state-dir", "st"], d);
        assert!(ok, "apply {f}: {text}");
    }
    let only_a_before = lock(d, "only-a");
    let (ok, text) = fj(
        &["lock-prune", "-f", "b.yaml", "--state-dir", "st", "--yes"],
        d,
    );
    assert!(
        !ok,
        "pruning a shared machine from stack-b must refuse: {text}"
    );
    assert!(
        text.contains("stack-a"),
        "the refusal names the other stack: {text}"
    );
    assert_eq!(
        lock(d, "only-a"),
        only_a_before,
        "only-a is not stack-b's machine"
    );
    assert!(
        lock(d, "shared").contains("a-on-shared"),
        "stack-a's entry survives"
    );
    let (ok, text) = fj(&["plan", "-f", "a.yaml", "--state-dir", "st"], d);
    assert!(
        ok && text.contains("0 to add"),
        "stack-a still plans clean: {text}"
    );
}

#[test]
fn control_single_stack_still_prunes_a_removed_resource() {
    let t = tempfile::tempdir().expect("tempdir");
    let d = t.path();
    let out = d.join("out");
    std::fs::create_dir_all(&out).expect("out");
    std::fs::write(
        d.join("a.yaml"),
        stack("solo", &["m"], &[("keep", "m"), ("drop", "m")], &out),
    )
    .expect("a");
    let (ok, text) = fj(&["apply", "--yes", "-f", "a.yaml", "--state-dir", "st"], d);
    assert!(ok, "apply: {text}");
    std::fs::write(
        d.join("a.yaml"),
        stack("solo", &["m"], &[("keep", "m")], &out),
    )
    .expect("a2");
    let (ok, text) = fj(
        &["lock-prune", "-f", "a.yaml", "--state-dir", "st", "--yes"],
        d,
    );
    assert!(ok, "single-stack prune must succeed: {text}");
    let after = lock(d, "m");
    assert!(after.contains("keep") && !after.contains("drop"), "{after}");
}
