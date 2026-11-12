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
    rt.diff = vec![('A', "/usr/bin/jq"), ('A', "/etc/greeting"), ('D', "/etc/old")];
    rt.nodes = std::rc::Rc::new(vec![
        FakeNode::File("usr/bin/jq", b"ELF-jq".to_vec(), 0o755),
        FakeNode::File("etc/greeting", b"hi\n".to_vec(), 0o644),
    ]);
    rt
}

fn build_with(rt: &FakeRuntime, state: &Path, cfg: &Path) -> Result<(), String> {
    let rt = rt.clone();
    let provider = move || Ok(Box::new(rt.clone()) as Box<dyn BuildRuntime>);
    cmd_build_rt(&provider, state, cfg, "app", false, false, false, false, false)
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
    assert!(second.log().is_empty(), "cache hit must not touch the runtime: {:?}", second.log());
    assert_eq!(manifest_digest(&state.join("app")), before);
}

#[test]
fn changing_packages_command_or_entrypoint_misses_the_cache() {
    let (work, state) = fresh(true);
    let variants = [
        STEPS.to_string(),
        "    packages: [jq, curl]\n    command: \"echo hi > /etc/greeting\"\n".to_string(),
        "    packages: [jq]\n    command: \"echo bye > /etc/greeting\"\n".to_string(),
        format!("{STEPS}    entrypoint: [\"/usr/bin/jq\"]\n"),
        format!("{STEPS}    provider: apt\n    entrypoint: [\"/usr/bin/jq\"]\n"),
    ];
    for (i, v) in variants.iter().enumerate() {
        let cfg = write_cfg(work.path(), &with_base(v));
        let rt = runtime();
        build_with(&rt, &state, &cfg).unwrap();
        // The last variant only restates the default provider: same key, a hit.
        let expect = usize::from(i != 4);
        assert_eq!(rt.count("start"), expect, "variant {i}");
    }
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
    assert!(err.contains("no `image:` base") && err.contains("entrypoint"), "{err}");
    assert!(!state.join("app").exists(), "no output dir for a refused build");
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
    assert!(err.contains("first difference") && err.contains("docker save"), "{err}");
    assert!(!state.join("app/index.json").exists());
    assert_eq!(rt.count("start"), 0);
}

#[test]
fn no_steps_needs_no_runtime() {
    let (work, state) = fresh(true);
    let cfg = write_cfg(work.path(), &with_base("    path: /app/bin\n"));
    let provider = || -> Result<Box<dyn BuildRuntime>, String> { Err("must not be asked".into()) };
    cmd_build_rt(&provider, &state, &cfg, "app", false, false, false, false, false).unwrap();
    assert_eq!(read_manifest(&state.join("app")).layers.len(), 2);
}

#[test]
fn files_layers_stay_above_the_executed_layer() {
    let (work, state) = fresh(true);
    let cfg = write_cfg(work.path(), &with_base(&format!("{STEPS}    path: /app/bin\n")));
    build_with(&runtime(), &state, &cfg).unwrap();
    assert_eq!(read_manifest(&state.join("app")).layers.len(), 3, "base, exec, files");
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
    assert!(load(&with_base("    command: \"echo x\"\n")).entrypoint.is_none());
    assert_eq!(
        load(&with_base("    entrypoint: [\"/bin/app\", \"--serve\"]\n")).entrypoint,
        Some(vec!["/bin/app".into(), "--serve".into()])
    );
}
