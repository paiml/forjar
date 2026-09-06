//! Harness for `falsification_state_stamp_per_name`: the paiml/infra shape —
//! N stacks, each with its own `name:`, its own machine and its own file
//! resource, all applied through ONE `--state-dir`.
//!
//! Kept out of the test file so the tests stay assertions and narrative; the
//! fixture is the part that would otherwise be copied into the next suite that
//! needs a multi-stack state dir.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use forjar::core::state;

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

/// One stack, in its own directory, with its OWN machine name and its own file
/// resource — the paiml/infra shape. `machines/<m>/forjar.yaml` differs from
/// its neighbours in exactly these three ways.
pub fn write_stack(root: &Path, dir: &str, name: &str, machine: &str, content: &str) -> PathBuf {
    let d = root.join(dir);
    std::fs::create_dir_all(&d).unwrap();
    let cfg = d.join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            r#"version: "1.0"
name: {name}
policy:
  snapshot_generations: 10
machines:
  {machine}:
    hostname: localhost
    addr: 127.0.0.1
    transport: local
resources:
  {name}_file:
    type: file
    machine: {machine}
    path: {}
    content: "{content}\n"
"#,
            d.join("marker.txt").display()
        ),
    )
    .unwrap();
    cfg
}

/// `write_stack` plus a SECOND file resource, `<name>_extra` → `extra.txt`.
///
/// The asymmetry is the point: apply this over a dir whose earlier generations
/// were written without it and the target of an `undo` no longer holds
/// `<name>_extra`, so the undo reaches `undo_prune::destroy_absent_from_target`
/// with something real to destroy — the only shape in which "refused" and
/// "changed nothing" are different claims.
pub fn write_stack_plus_extra(
    root: &Path,
    dir: &str,
    name: &str,
    machine: &str,
    content: &str,
) -> PathBuf {
    let cfg = write_stack(root, dir, name, machine, content);
    let extra = extra_marker(root, dir);
    let mut body = std::fs::read_to_string(&cfg).unwrap();
    body.push_str(&format!(
        "  {name}_extra:\n    type: file\n    machine: {machine}\n    path: {}\n    content: \"extra\\n\"\n",
        extra.display()
    ));
    std::fs::write(&cfg, body).unwrap();
    cfg
}

/// Where `write_stack_plus_extra` puts the extra resource's file — the
/// destroyed candidate, on the HOST rather than in the state dir.
pub fn extra_marker(root: &Path, dir: &str) -> PathBuf {
    root.join(dir).join("extra.txt")
}

/// PMAT-176: ONE stack that declares TWO machines, each with its own file
/// resource — the shape a scoped `apply -m <machine>` narrows.
///
/// `write_stack` cannot express it: a stack with one machine has nothing to
/// leave out of a scoped apply, so the stamp it writes is the same set either
/// way and the defect (the other machine dropping out of `stacks[name]`) is
/// invisible.
pub fn write_two_machine_stack(
    root: &Path,
    dir: &str,
    name: &str,
    machines: (&str, &str),
    content: &str,
) -> PathBuf {
    let (first, second) = machines;
    let d = root.join(dir);
    std::fs::create_dir_all(&d).unwrap();
    let cfg = d.join("forjar.yaml");
    let mut body = format!(
        r#"version: "1.0"
name: {name}
policy:
  snapshot_generations: 10
machines:
"#
    );
    for machine in [first, second] {
        body.push_str(&format!(
            "  {machine}:\n    hostname: localhost\n    addr: 127.0.0.1\n    transport: local\n"
        ));
    }
    body.push_str("resources:\n");
    for machine in [first, second] {
        body.push_str(&format!(
            "  {name}_{machine}_file:\n    type: file\n    machine: {machine}\n    path: {}\n    content: \"{content}\\n\"\n",
            machine_marker(root, dir, machine).display(),
        ));
    }
    std::fs::write(&cfg, body).unwrap();
    cfg
}

/// Where `write_two_machine_stack` puts one machine's file.
pub fn machine_marker(root: &Path, dir: &str, machine: &str) -> PathBuf {
    root.join(dir).join(format!("marker-{machine}.txt"))
}

/// PMAT-177: named snapshots planted directly in the state dir.
///
/// The snapshot name carries a SECOND-resolution timestamp, so applies driven
/// back to back inside one second collide on it and the dir never reaches the
/// retention count by applying alone. Planting the directories is both faster
/// and deterministic; `gc_old_snapshots` reads the dir, not the applies that
/// filled it. The names sort before `pre-apply-*`, so they are exactly what a
/// gc that runs would remove first.
pub fn plant_snapshots(state: &Path, count: usize) -> Vec<String> {
    let dir = state.join("snapshots");
    std::fs::create_dir_all(&dir).unwrap();
    let mut planted = Vec::new();
    for i in 0..count {
        let name = format!("planted-{i:02}");
        std::fs::create_dir_all(dir.join(&name)).unwrap();
        std::fs::write(dir.join(&name).join("marker"), "planted\n").unwrap();
        planted.push(name);
    }
    planted
}

/// The named snapshots this state dir holds, sorted.
pub fn snapshot_names(state: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(state.join("snapshots")) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    names
}

pub fn run(args: &[&str]) -> (i32, String) {
    let out = forjar().args(args).output().unwrap();
    let mut merged = String::from_utf8_lossy(&out.stdout).into_owned();
    merged.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code().unwrap_or(-1), merged)
}

pub fn apply(cfg: &Path, state: &Path) -> (i32, String) {
    apply_with(cfg, state, &[])
}

/// `apply` carrying extra flags — `--rollback-on-failure` (PMAT-174), `-m`
/// (PMAT-176). The base arguments are the ones `apply` uses, so a row that adds
/// a flag differs from the ordinary apply in exactly that flag.
pub fn apply_with(cfg: &Path, state: &Path, extra: &[&str]) -> (i32, String) {
    let cfg = cfg.display().to_string();
    let state = state.display().to_string();
    let mut args = vec!["apply", "-f", &cfg, "--state-dir", &state, "--yes"];
    args.extend_from_slice(extra);
    run(&args)
}

pub fn undo(cfg: &Path, state: &Path) -> (i32, String) {
    run(&[
        "undo",
        "-f",
        &cfg.display().to_string(),
        "--state-dir",
        &state.display().to_string(),
        "--yes",
    ])
}

/// `undo` naming its target explicitly. The flag is `--generations N` ("how
/// many back"); `undo` has no `--generation`, that spelling belongs to
/// `rollback`.
pub fn undo_generations(cfg: &Path, state: &Path, back: &str) -> (i32, String) {
    run(&[
        "undo",
        "-f",
        &cfg.display().to_string(),
        "--state-dir",
        &state.display().to_string(),
        "--generations",
        back,
        "--yes",
    ])
}

pub fn undo_resume(cfg: &Path, state: &Path) -> (i32, String) {
    run(&[
        "undo",
        "-f",
        &cfg.display().to_string(),
        "--state-dir",
        &state.display().to_string(),
        "--resume",
        "--yes",
    ])
}

/// The other door onto the same primitive: `rollback --generation N` reaches
/// `generation::rollback_to_generation` with no config in the picture at all,
/// which is why the guard cannot live in `undo`.
pub fn rollback(state: &Path, generation: &str) -> (i32, String) {
    run(&[
        "rollback",
        "--state-dir",
        &state.display().to_string(),
        "--generation",
        generation,
        "--yes",
    ])
}

/// Every path under `dir` with a digest of what it holds — file bytes, symlink
/// target, or the fact that it is a directory.
///
/// "The other stacks were untouched" has to be asserted over the WHOLE state
/// dir: the restore is whole-dir, so what it reverts is every machine lock and
/// `forjar.lock.yaml` itself, none of which the host-side marker files show.
pub fn fingerprint(dir: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    fingerprint_into(dir, dir, &mut out);
    out
}

fn fingerprint_into(root: &Path, at: &Path, out: &mut BTreeMap<String, String>) {
    let Ok(entries) = std::fs::read_dir(at) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let key = path.strip_prefix(root).unwrap().display().to_string();
        let meta = std::fs::symlink_metadata(&path).unwrap();
        if meta.is_symlink() {
            let target = std::fs::read_link(&path).unwrap();
            out.insert(key, format!("-> {}", target.display()));
        } else if meta.is_dir() {
            out.insert(key, "dir".to_string());
            fingerprint_into(root, &path, out);
        } else {
            out.insert(key, digest(&std::fs::read(&path).unwrap()));
        }
    }
}

fn digest(bytes: &[u8]) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut h);
    format!("{:016x}", h.finish())
}

/// Every `warning:` line, whatever printed it. The point of forjar#469 is that
/// a supported layout produces NONE — a warning an operator sees on every
/// correct apply is a warning they stop reading.
///
/// ONE exclusion, and it is not a stack signal: named snapshots carry a
/// second-resolution timestamp, so two applies inside the same second collide
/// on the name and the second prints "pre-apply snapshot failed … already
/// exists". A test that drives five applies in three seconds hits that; an
/// operator does not. Excluded by its text so anything else still fails here.
pub fn warnings(out: &str) -> Vec<String> {
    out.lines()
        .filter(|l| l.trim_start().starts_with("warning:"))
        .filter(|l| !l.contains("pre-apply snapshot failed"))
        .map(str::to_string)
        .collect()
}

pub fn lock_of(state: &Path) -> forjar::core::types::GlobalLock {
    state::load_global_lock(state).unwrap().unwrap()
}

/// The stack names stamped in `state`, in the order the lock records them.
///
/// PMAT-161 asserts on this map constantly — it is the count
/// `multi_stack_restore_refusal` refuses on — so it is read in one place.
pub fn stack_names(state: &Path) -> Vec<String> {
    lock_of(state).stacks.keys().cloned().collect()
}

pub fn marker(root: &Path, dir: &str) -> String {
    std::fs::read_to_string(root.join(dir).join("marker.txt")).unwrap()
}

/// alpha/bravo/charlie, distinct machines, distinct resources, ONE state dir.
pub struct Fleet {
    pub _dir: tempfile::TempDir,
    pub root: PathBuf,
    pub state: PathBuf,
    pub alpha: PathBuf,
    pub bravo: PathBuf,
}

pub fn fleet() -> (Fleet, Vec<String>) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let state = root.join("state");
    let alpha = write_stack(&root, "alpha", "alpha", "mini", "one");
    let bravo = write_stack(&root, "bravo", "bravo", "lambda-labs", "one");
    let charlie = write_stack(&root, "charlie", "charlie", "clean-room", "one");

    let mut warned = Vec::new();
    for cfg in [&alpha, &bravo, &charlie] {
        let (rc, out) = apply(cfg, &state);
        assert_eq!(rc, 0, "apply of {} failed:\n{out}", cfg.display());
        warned.extend(warnings(&out));
    }
    (
        Fleet {
            _dir: dir,
            root,
            state,
            alpha,
            bravo,
        },
        warned,
    )
}

/// PMAT-161 (S2): RENAME a stack in place — same file, same resource ids, same
/// machine, a new `name:` and new content.
///
/// The identity that matters is the config FILE: `write_stack` with a new name
/// would also rename the resource (`<name>_file`), which is an edit to the
/// stack, not a rename OF it. The content changes with it so the apply that
/// follows moves the host, without which the undo that follows is a no-op and
/// "allowed" cannot be told from "did nothing".
pub fn rename_stack(cfg: &Path, old: &str, new: &str, content: &str) {
    let body = std::fs::read_to_string(cfg).unwrap();
    let mut out = String::new();
    for line in body.lines() {
        if line == format!("name: {old}") {
            out.push_str(&format!("name: {new}\n"));
        } else if line.trim_start().starts_with("content:") {
            out.push_str(&format!("    content: \"{content}\\n\"\n"));
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    std::fs::write(cfg, out).unwrap();
}

/// PMAT-182: the numbered generations this state dir holds, sorted.
///
/// The SECOND retention path. `snapshot_names` reads `snapshots/`, which an
/// operator restores by name; this reads `generations/`, which is what `undo`
/// and `rollback --generation` target — and what `gc_generations` trims by a
/// keep count that knows nothing about which stack wrote which number.
pub fn generation_numbers(state: &Path) -> Vec<u32> {
    let Ok(entries) = std::fs::read_dir(state.join("generations")) else {
        return Vec::new();
    };
    let mut nums: Vec<u32> = entries
        .flatten()
        .filter_map(|e| e.file_name().to_string_lossy().parse::<u32>().ok())
        .collect();
    nums.sort_unstable();
    nums
}
