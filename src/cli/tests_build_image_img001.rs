//! IMG-001 (paiml/infra#2074, forjar#756): an image resource's own `image`,
//! `packages` and `command` must reach the built image; `--json` must print the
//! manifest digest.

use super::build_image::*;
use super::build_image_exec::{build_json_doc, image_build_scripts, plan_needs_execution};
use crate::core::types::{ForjarConfig, LayerStrategy};

fn spec() -> ForjarConfig {
    serde_yaml_ng::from_str(
        r#"
version: "1.0"
name: img
machines:
  m: {hostname: m, addr: 127.0.0.1}
resources:
  build-image:
    type: image
    machine: m
    image: "ubuntu@sha256:0a9887931b0e38af2e3487ed39e29436b0535c4f3f52e57745baaf79d1898956"
    tag: "almacen:5000/paiml/aprender-build:spike"
    packages: [build-essential, git]
    command: "echo built > /opt/built.txt"
    path: /opt/img001/prejob-gate.sh
"#,
    )
    .unwrap()
}

#[test]
fn img001_packages_and_command_become_build_layers_not_an_entrypoint() {
    let config = spec();
    let res = &config.resources["build-image"];
    let plan = test_build_plan_from_resource("build-image", res, &config).unwrap();
    assert!(plan
        .layers
        .iter()
        .any(|l| matches!(l, LayerStrategy::Packages { names } if names == &["build-essential", "git"])));
    assert!(plan
        .layers
        .iter()
        .any(|l| matches!(l, LayerStrategy::Build { command, .. } if command.contains("/opt/built.txt"))));
    assert!(plan.entrypoint.is_none(), "a build command is not the entrypoint");
}

#[test]
fn img001_scripts_install_each_package_then_run_the_command() {
    let config = spec();
    let res = &config.resources["build-image"];
    let plan = test_build_plan_from_resource("build-image", res, &config).unwrap();
    let scripts = image_build_scripts(&plan, res).unwrap();
    assert_eq!(scripts.len(), 2, "one packages script, one command script");
    assert!(scripts[0].contains("build-essential") && scripts[0].contains("git"));
    assert!(scripts[1].contains("echo built > /opt/built.txt"));
}

#[test]
fn img001_a_resource_with_packages_needs_a_container() {
    let config = spec();
    let res = &config.resources["build-image"];
    let plan = test_build_plan_from_resource("build-image", res, &config).unwrap();
    assert!(plan_needs_execution(&plan));
    let legacy = super::build_image::test_build_plan_from_resource(
        "x",
        &serde_yaml_ng::from_str::<ForjarConfig>(
            "version: '1.0'\nname: t\nmachines: {m: {hostname: m, addr: 127.0.0.1}}\nresources:\n  x: {type: image, machine: m, path: /a}\n",
        )
        .unwrap()
        .resources["x"],
        &spec(),
    )
    .unwrap();
    assert!(!plan_needs_execution(&legacy));
}

#[test]
fn img001_json_prints_the_manifest_digest_fresh_and_cached() {
    let out = tempfile::tempdir().unwrap();
    let plan = test_build_plan_from_resource(
        "build-image",
        &spec().resources["build-image"],
        &spec(),
    )
    .unwrap();
    let entries = vec![vec![crate::core::store::layer_builder::LayerEntry::file(
        "a", b"a", 0o644,
    )]];
    let mut plan1 = plan.clone();
    plan1.layers.truncate(1);
    let img = crate::core::store::image_assembler::assemble_image(
        &plan1,
        &entries,
        out.path(),
        &crate::core::types::OciLayerConfig::default(),
        None,
    )
    .unwrap();
    let fresh = build_json_doc(&plan1.tag, out.path(), Some(&img), false);
    assert_eq!(fresh["manifest_digest"], img.manifest_digest.as_str());
    let cached = build_json_doc(&plan1.tag, out.path(), None, true);
    assert_eq!(
        cached["manifest_digest"], img.manifest_digest.as_str(),
        "a cache hit reads the digest back from the layout on disk"
    );
}
