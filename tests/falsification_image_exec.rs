//! PMAT-989 (IMG-001 3/N) falsifiers against a REAL container runtime.
//!
//! F1: the same spec built twice gives the same manifest digest.
//! F2: a declared package (and the declared command's output) is in the built image.
//!
//! Needs docker (or a PATH `docker`) and a network for `apt-get`; it FAILS, it does
//! not skip, when the runtime is missing. Opt in with `--features container-test`.
//! Determinism is measured within one run window: `jq` is not version-pinned, so
//! the package index must not change between the two builds.
#![cfg(feature = "container-test")]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// debian:bookworm-slim, by digest (linux/amd64 manifest list), resolved 2026-11-12.
const BASE: &str = "debian@sha256:734a72b3d6142a71993b4bc580ca01d8ca687f77a2c422d96a961d3376b179bc";
/// The tag the built image gets (`name:version` of the resource below).
const TAG: &str = "forjar-exec-989:1";
const MARKER: &str = "built by forjar\n";

fn docker(args: &[&str]) -> Output {
    Command::new("docker").args(args).output().expect(
        "container-test needs `docker` on PATH; none found (this test fails, it does not skip)",
    )
}

fn ok(args: &[&str]) -> String {
    let o = docker(args);
    assert!(
        o.status.success(),
        "docker {args:?}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8_lossy(&o.stdout).into_owned()
}

/// Ids of the images loaded under `TAG`'s repository (what this test created).
fn loaded_ids() -> Vec<String> {
    let repo = TAG.split(':').next().unwrap();
    let mut ids: Vec<String> = ok(&["image", "ls", "-q", "--no-trunc", repo])
        .lines()
        .map(str::to_string)
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

/// Removes the loaded image even when an assertion fails. By id: on a daemon
/// with the containerd image store the loaded name is not normalised, so
/// `docker rmi <tag>` and `docker run <tag>` do not find it (see F2).
struct Untag;
impl Drop for Untag {
    fn drop(&mut self) {
        for id in loaded_ids() {
            let _ = docker(&["rmi", "-f", &id]);
        }
    }
}

/// A working dir with the base OCI layout in `state/images/<mapped ref>` and a config.
fn workdir() -> (tempfile::TempDir, PathBuf) {
    if !docker(&["image", "inspect", BASE]).status.success() {
        ok(&["pull", BASE]);
    }
    let work = tempfile::tempdir().unwrap();
    let layout = work
        .path()
        .join("state/images")
        .join(BASE.replace([':', '/'], "_"));
    std::fs::create_dir_all(&layout).unwrap();
    // The command the error text tells a user to run, pipe and all.
    let st = Command::new("bash")
        .arg("-c")
        .arg("set -o pipefail; docker save \"$0\" | tar -x -C \"$1\"")
        .arg(BASE)
        .arg(&layout)
        .status()
        .unwrap();
    assert!(st.success(), "docker save | tar -x failed: {st}");
    let cfg = work.path().join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: exec-989\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\n\
             resources:\n  app:\n    type: image\n    machine: m\n    name: forjar-exec-989\n    \
             version: \"1\"\n    image: \"{BASE}\"\n    packages: [jq]\n    \
             command: \"printf 'built by forjar\\\\n' > /etc/forjar-exec-marker\"\n"
        ),
    )
    .unwrap();
    (work, cfg)
}

fn forjar_build(work: &Path, cfg: &Path, extra: &[&str]) -> String {
    let o = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .current_dir(work)
        .args(["build", "-f"])
        .arg(cfg)
        .args(["--resource", "app"])
        .args(extra)
        .output()
        .unwrap();
    let out = String::from_utf8_lossy(&o.stdout).into_owned();
    assert!(
        o.status.success(),
        "forjar build failed:\n{out}\n{}",
        String::from_utf8_lossy(&o.stderr)
    );
    out
}

fn read_json(p: &Path) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}

/// `(manifest digest, the manifest document)` of a built layout.
fn built_manifest(work: &Path) -> (String, serde_json::Value) {
    let out = work.join("state/images/app");
    let index = read_json(&out.join("index.json"));
    let digest = index["manifests"][0]["digest"]
        .as_str()
        .unwrap()
        .to_string();
    let hex = digest.strip_prefix("sha256:").unwrap();
    let doc = read_json(&out.join("blobs/sha256").join(hex));
    (digest, doc)
}

#[test]
fn f1_the_same_spec_built_twice_has_the_same_manifest_digest() {
    let (a, cfg_a) = workdir();
    let (b, cfg_b) = workdir();
    forjar_build(a.path(), &cfg_a, &[]);
    forjar_build(b.path(), &cfg_b, &[]);
    let (da, ma) = built_manifest(a.path());
    let (db, _) = built_manifest(b.path());
    println!("F1 run1 manifest digest: {da}");
    println!("F1 run2 manifest digest: {db}");
    assert_eq!(
        da, db,
        "same spec, same base, same steps: the digest must not move"
    );

    // The declared base is first, the executed layer second.
    let saved = read_json(
        &a.path()
            .join("state/images")
            .join(BASE.replace([':', '/'], "_"))
            .join("manifest.json"),
    );
    let base_layer = saved[0]["Layers"][0].as_str().unwrap();
    let layers = ma["layers"].as_array().unwrap();
    assert_eq!(layers.len(), 2, "base layer + one executed layer");
    assert_eq!(
        layers[0]["digest"].as_str().unwrap(),
        format!("sha256:{}", base_layer.rsplit('/').next().unwrap())
    );
}

#[test]
fn f2_the_declared_package_and_command_output_are_in_the_built_image() {
    let (work, cfg) = workdir();
    let _untag = Untag;
    let out = forjar_build(work.path(), &cfg, &["--load"]);
    assert!(out.contains("loaded into docker"), "{out}");
    // Run by image id, not by tag: a containerd-image-store daemon lists the loaded
    // image as `forjar-exec-989:1` yet resolves neither `docker run` nor `docker image
    // inspect` of that name (measured on Docker 29.1.3 / kfocus). The content is the
    // thing under test, so name it by id.
    let ids = loaded_ids();
    assert_eq!(ids.len(), 1, "exactly one image was loaded: {ids:?}");
    let id = ids[0].as_str();
    let jq = docker(&["run", "--rm", "--pull=never", id, "jq", "--version"]);
    println!(
        "F2 jq --version: rc={:?} {}",
        jq.status.code(),
        String::from_utf8_lossy(&jq.stdout).trim()
    );
    assert_eq!(
        jq.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&jq.stderr)
    );
    let marker = ok(&[
        "run",
        "--rm",
        "--pull=never",
        id,
        "cat",
        "/etc/forjar-exec-marker",
    ]);
    println!("F2 marker file present: {}", marker == MARKER);
    assert_eq!(marker, MARKER);
}
