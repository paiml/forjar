//! PMAT-989 (IMG-001 3/N, phase 2): `packages:` and `command:` are executed
//! into a layer on top of the declared base. The container runtime is a fake;
//! the real-runtime falsifier is `tests/falsification_image_exec.rs`.

use super::build_image::{cmd_build_rt, test_build_plan_from_resource};
use super::tests_build_image_base::{
    hex, manifest_digest, read_manifest, write_base, BASE_DIR_NAME, BASE_REF,
};
use crate::core::store::build_runtime::BuildRuntime;
use crate::core::store::fake_build_runtime::{FakeNode, FakeRuntime};
use std::path::{Path, PathBuf};

const SEED: u8 = 0x10;

/// The diff_id `write_base(.., SEED, ..)` records in its config.
fn base_diff_id() -> String {
    format!("sha256:{}", hex(SEED + 3))
}

fn write_cfg(dir: &Path, body: &str) -> PathBuf {
    let path = dir.join("forjar.yaml");
    std::fs::write(
        &path,
        format!(
            "version: \"1.0\"\nname: test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\n\
             resources:\n  app:\n    type: image\n    machine: m\n    name: myapp\n    \
             version: \"1.0\"\n{body}"
        ),
    )
    .unwrap();
    path
}

fn with_base(extra: &str) -> String {
    format!("    image: \"{BASE_REF}\"\n{extra}")
}

fn runtime() -> FakeRuntime {
    let mut rt = FakeRuntime::new(vec![base_diff_id()]);
    rt.diff = vec![
        ('A', "/usr/bin/jq"),
        ('A', "/etc/greeting"),
        ('D', "/etc/old"),
    ];
    rt.nodes = std::rc::Rc::new(vec![
        FakeNode::File("usr/bin/jq", b"ELF-jq".to_vec(), 0o755),
        FakeNode::File("etc/greeting", b"hi\n".to_vec(), 0o644),
    ]);
    rt
}

fn build_with(rt: &FakeRuntime, state: &Path, cfg: &Path) -> Result<(), String> {
    let rt = rt.clone();
    let provider = move || Ok(Box::new(rt.clone()) as Box<dyn BuildRuntime>);
    cmd_build_rt(
        &provider, state, cfg, "app", false, false, false, false, false,
    )
}

fn fresh(seed_state: bool) -> (tempfile::TempDir, PathBuf) {
    let work = tempfile::tempdir().unwrap();
    let state = work.path().join("state");
    if seed_state {
        write_base(&state.join(BASE_DIR_NAME), SEED, b"base-layer-bytes");
    }
    (work, state)
}

const STEPS: &str = "    packages: [jq]\n    command: \"echo hi > /etc/greeting\"\n";

#[test]
fn the_executed_layer_sits_on_the_base_and_holds_what_the_steps_changed() {
    let (work, state) = fresh(true);
    let cfg = write_cfg(work.path(), &with_base(STEPS));
    let rt = runtime();
    build_with(&rt, &state, &cfg).unwrap();

    let m = read_manifest(&state.join("app"));
    assert_eq!(m.layers.len(), 2, "base + the executed layer");
    assert_eq!(m.layers[0].digest, format!("sha256:{}", hex(SEED + 1)));
    assert_eq!(rt.count("start"), 1);
    assert_eq!(rt.count("remove"), 1);
    assert_eq!(rt.count("exec"), 3, "packages, command, normalise");
}

#[test]
fn the_same_spec_built_twice_has_the_same_manifest_digest() {
    let digests: Vec<String> = (0..2)
        .map(|_| {
            let (work, state) = fresh(true);
            let cfg = write_cfg(work.path(), &with_base(STEPS));
            build_with(&runtime(), &state, &cfg).unwrap();
            manifest_digest(&state.join("app"))
        })
        .collect();
    assert_eq!(digests[0], digests[1]);
}

#[test]
fn an_unchanged_spec_is_a_cache_hit_that_starts_no_container() {
    let (work, state) = fresh(true);
    let cfg = write_cfg(work.path(), &with_base(STEPS));
    build_with(&runtime(), &state, &cfg).unwrap();
    let before = manifest_digest(&state.join("app"));

    let second = runtime();
    build_with(&second, &state, &cfg).unwrap();
    assert!(
        second.log().is_empty(),
        "cache hit must not touch the runtime: {:?}",
        second.log()
    );
    assert_eq!(manifest_digest(&state.join("app")), before);
}

#[test]
fn changing_packages_command_or_entrypoint_misses_the_cache() {
    // Each variant differs from the one before it in exactly ONE field, so a key
    // that drops that field serves the stale image (a hit, no container).
    let (work, state) = fresh(true);
    let cmd_only = "    packages: [jq]\n    command: \"echo bye > /etc/greeting\"\n";
    let with_entry = format!("{cmd_only}    entrypoint: [\"/usr/bin/jq\"]\n");
    let with_pkg = "    packages: [jq, curl]\n    command: \"echo bye > /etc/greeting\"\n    \
                    entrypoint: [\"/usr/bin/jq\"]\n";
    let restated = format!("{with_pkg}    provider: apt\n");
    let entry_only = "    packages: [jq, curl]\n    command: \"echo bye > /etc/greeting\"\n    \
                      entrypoint: [\"/usr/bin/curl\"]\n";
    let variants: [(&str, String, usize); 6] = [
        ("baseline", STEPS.to_string(), 1),
        ("command only", cmd_only.to_string(), 1),
        ("entrypoint only", with_entry, 1),
        ("packages only", with_pkg.to_string(), 1),
        // Restating the default provider is the same key: a hit.
        ("default provider restated", restated, 0),
        ("entrypoint value only", entry_only.to_string(), 1),
    ];
    for (name, v, expect) in &variants {
        let cfg = write_cfg(work.path(), &with_base(v));
        let rt = runtime();
        build_with(&rt, &state, &cfg).unwrap();
        assert_eq!(rt.count("start"), *expect, "variant {name}");
    }
}

fn exec_key(command: Option<&str>, entrypoint: &[&str]) -> String {
    use super::build_image_exec::{hash_exec_inputs, ExecDecl};
    let decl = ExecDecl {
        provider: "apt".to_string(),
        packages: vec!["jq".to_string()],
        command: command.map(str::to_string),
    };
    let ep: Vec<String> = entrypoint.iter().map(|s| (*s).to_string()).collect();
    let mut h = blake3::Hasher::new();
    hash_exec_inputs(&mut h, Some(&decl), &ep);
    h.finalize().to_hex().to_string()
}

#[test]
fn the_exec_cache_key_covers_command_and_entrypoint_and_is_deterministic() {
    let base = exec_key(Some("echo a"), &["/bin/a"]);
    assert_eq!(base, exec_key(Some("echo a"), &["/bin/a"]), "same inputs");
    assert_ne!(base, exec_key(Some("echo b"), &["/bin/a"]), "command only");
    assert_ne!(base, exec_key(Some("echo a"), &["/bin/b"]), "entrypoint only");
    assert_ne!(base, exec_key(Some("echo a"), &[]), "entrypoint dropped");
    assert_ne!(base, exec_key(None, &["/bin/a"]), "command dropped");
}

#[test]
fn a_package_order_change_misses_the_cache() {
    let (work, state) = fresh(true);
    for (i, pk) in ["[a, b]", "[b, a]"].iter().enumerate() {
        let cfg = write_cfg(work.path(), &with_base(&format!("    packages: {pk}\n")));
        let rt = runtime();
        build_with(&rt, &state, &cfg).unwrap();
        assert_eq!(rt.count("start"), 1, "order variant {i}");
    }
}

#[test]
fn steps_without_a_declared_base_are_refused_before_anything_is_written() {
    let (work, state) = fresh(false);
    let cfg = write_cfg(work.path(), STEPS);
    let rt = runtime();
    let err = build_with(&rt, &state, &cfg).unwrap_err();
    assert!(
        err.contains("no `image:` base") && err.contains("entrypoint"),
        "{err}"
    );
    assert!(
        !state.join("app").exists(),
        "no output dir for a refused build"
    );
    assert!(rt.log().is_empty());
}

#[test]
fn a_failing_step_writes_no_image_and_removes_the_container() {
    let (work, state) = fresh(true);
    let cfg = write_cfg(work.path(), &with_base(STEPS));
    let mut rt = runtime();
    rt.fail_on = Some("echo hi");
    let err = build_with(&rt, &state, &cfg).unwrap_err();
    assert!(err.contains("'command'") && err.contains("boom"), "{err}");
    assert!(!state.join("app/index.json").exists(), "no image written");
    assert_eq!(rt.count("remove"), 1);
}

#[test]
fn a_runtime_whose_base_differs_from_the_layout_is_refused() {
    let (work, state) = fresh(true);
    let cfg = write_cfg(work.path(), &with_base(STEPS));
    let rt = FakeRuntime::new(vec!["sha256:other".into()]);
    let err = build_with(&rt, &state, &cfg).unwrap_err();
    assert!(
        err.contains("first difference") && err.contains("docker save"),
        "{err}"
    );
    assert!(!state.join("app/index.json").exists());
    assert_eq!(rt.count("start"), 0);
}

#[test]
fn no_steps_needs_no_runtime() {
    let (work, state) = fresh(true);
    let cfg = write_cfg(work.path(), &with_base("    path: /app/bin\n"));
    let provider = || -> Result<Box<dyn BuildRuntime>, String> { Err("must not be asked".into()) };
    cmd_build_rt(
        &provider, &state, &cfg, "app", false, false, false, false, false,
    )
    .unwrap();
    assert_eq!(read_manifest(&state.join("app")).layers.len(), 2);
}

#[test]
fn files_layers_stay_above_the_executed_layer() {
    let (work, state) = fresh(true);
    let cfg = write_cfg(
        work.path(),
        &with_base(&format!("{STEPS}    path: /app/bin\n")),
    );
    build_with(&runtime(), &state, &cfg).unwrap();
    assert_eq!(
        read_manifest(&state.join("app")).layers.len(),
        3,
        "base, exec, files"
    );
}

#[test]
fn command_is_a_build_step_and_entrypoint_is_its_own_field() {
    let (work, _) = fresh(false);
    let load = |body: &str| {
        let cfg = write_cfg(work.path(), body);
        let config: crate::core::types::ForjarConfig =
            serde_yaml_ng::from_str(&std::fs::read_to_string(cfg).unwrap()).unwrap();
        let res = config.resources.get("app").unwrap().clone();
        test_build_plan_from_resource("app", &res, &config).unwrap()
    };
    assert!(load(&with_base("    command: \"echo x\"\n"))
        .entrypoint
        .is_none());
    assert_eq!(
        load(&with_base("    entrypoint: [\"/bin/app\", \"--serve\"]\n")).entrypoint,
        Some(vec!["/bin/app".into(), "--serve".into()])
    );
}

#[test]
fn sandbox_with_steps_is_refused_not_silently_dropped() {
    let (work, state) = fresh(true);
    let cfg = write_cfg(work.path(), &with_base(STEPS));
    let rt = runtime();
    let provider = {
        let rt = rt.clone();
        move || Ok(Box::new(rt.clone()) as Box<dyn BuildRuntime>)
    };
    let err = cmd_build_rt(
        &provider, &state, &cfg, "app", false, false, false, true, false,
    )
    .unwrap_err();
    assert!(
        err.contains("--sandbox") && err.contains("packages"),
        "{err}"
    );
    assert!(rt.log().is_empty());
    assert!(
        !state.join("app").exists(),
        "nothing written for a refused build"
    );
}

#[test]
fn the_packages_step_runs_the_apply_script_for_the_declared_packages() {
    let (work, state) = fresh(true);
    let cfg = write_cfg(work.path(), &with_base("    packages: [jq, curl]\n"));
    let rt = runtime();
    build_with(&rt, &state, &cfg).unwrap();
    let scripts = rt.scripts.borrow();
    let first = &scripts[0];
    assert!(
        !first.trim().is_empty(),
        "packages script must not be empty"
    );
    assert!(first.contains("jq") && first.contains("curl"), "{first}");
    assert!(first.contains("apt"), "default provider: {first}");
}

#[test]
fn json_report_includes_the_exec_object() {
    use super::build_image_exec::ExecReport;
    use super::build_image_report::build_json_doc;
    let report = ExecReport {
        packages: 2,
        command: true,
        files: Some(5),
        whiteouts: Some(1),
        stripped: 3,
    };
    let doc = build_json_doc("t:1", Path::new("o"), None, false, None, Some(&report));
    assert_eq!(doc["exec"]["packages"], 2);
    assert_eq!(doc["exec"]["command"], true);
    assert_eq!(doc["exec"]["files"], 5);
    assert_eq!(doc["exec"]["whiteouts"], 1);
    assert_eq!(doc["exec"]["setid_stripped"], 3);
    assert!(report.line().contains("stripped setuid/setgid from 3"));
}

#[test]
fn a_non_root_setuid_file_is_built_without_its_setid_bits() {
    use crate::core::store::build_runtime::{DiffEntry, DiffKind};
    use crate::core::store::exec_capture::capture;
    let mut rt = runtime();
    rt.nodes = std::rc::Rc::new(vec![FakeNode::File("usr/bin/su2", b"x".to_vec(), 0o4755)]);
    let dir = tempfile::tempdir().unwrap();
    let tar = dir.path().join("e.tar");
    rt.export("c", &tar).unwrap();
    let diff = [DiffEntry {
        kind: DiffKind::Added,
        path: "/usr/bin/su2".into(),
    }];
    let layer = capture(&diff, &tar).unwrap();
    assert_eq!(layer.stripped, vec!["/usr/bin/su2".to_string()]);
    let e = layer
        .entries
        .iter()
        .find(|e| e.path == "usr/bin/su2")
        .unwrap();
    assert_eq!(e.mode & 0o6000, 0);
    assert_eq!(e.mode, 0o755);
}

#[test]
fn the_report_counts_what_capture_stripped() {
    use super::build_image_exec::{ExecDecl, ExecReport};
    use crate::core::store::build_runtime::{DiffEntry, DiffKind};
    use crate::core::store::exec_capture::capture;
    let mut rt = runtime();
    rt.nodes = std::rc::Rc::new(vec![FakeNode::File("usr/bin/su2", b"x".to_vec(), 0o4755)]);
    let dir = tempfile::tempdir().unwrap();
    let tar = dir.path().join("e.tar");
    rt.export("c", &tar).unwrap();
    let diff = [DiffEntry {
        kind: DiffKind::Added,
        path: "/usr/bin/su2".into(),
    }];
    let layer = capture(&diff, &tar).unwrap();
    let decl = ExecDecl {
        provider: "apt".into(),
        packages: vec!["jq".into()],
        command: None,
    };
    let report = ExecReport::new(&decl).captured(&layer);
    assert_eq!(report.stripped, 1);
    assert_eq!(report.json()["setid_stripped"], 1);
    assert!(report.line().contains("stripped setuid/setgid from 1"));
}
