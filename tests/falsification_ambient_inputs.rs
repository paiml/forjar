//! GH-244(c): an ambient host input must be DECLARABLE, and declaring it must
//! make the task stale when it changes.
//!
//! THE FLAW THIS CLOSES.
//!
//! `probe_resource` derived a task's `input_hash` from one source only:
//! `hash_inputs(&resource.task_inputs, &base)`, which glob-expands PATHS. There
//! was no environment component, no tool-version component, no ambient
//! component — and `Resource` had no field that could carry one, so the input
//! was not merely undetected, it was undeclarable.
//!
//! Measured on forjar 1.13.2. A task reading an undeclared `ambient/fonts.txt`
//! was applied, the file was changed, and every read verb reported clean:
//!
//!     plan   -> Plan: 0 to add, 0 to change, 0 to destroy, 1 unchanged.
//!     check  -> Check: 1 pass, 0 fail, 0 skip
//!     drift  -> No drift detected.
//!     apply  -> Apply complete: 0 converged, 1 unchanged.
//!
//! while `--force` proved the artifact stale by changing its bytes. The
//! motivating case is a rasterizer calling `fontdb.load_system_fonts()`: an
//! ambient input to every render, no honest glob for it, and it changes when
//! somebody runs `apt install fonts-*`.
//!
//! WHAT THIS TEST MUST NOT BECOME. The easy way to "detect an ambient change"
//! is to composite it in the probe and forget the lock — which reports "inputs
//! changed" on EVERY plan forever and breaks `f(f(x)) = f(x)`. So the
//! settle-to-quiet case here is as load-bearing as the detection case, and both
//! run through the same `record_io_hashes` the executor writes with.
//!
//! SCOPE, stated plainly. This is still a declaration. It covers what you name.
//! `forjar verify --check-declared-inputs` covers unnamed reads of files inside
//! the project tree. Nothing on offer proves the declaration complete.

use forjar::core::task::probe::record_io_hashes;
use forjar::core::task::{hash_declared_inputs, hash_inputs, probe_resource, staleness_reason};
use forjar::core::types::{MachineTarget, Resource, ResourceType};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn fixture(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fj244-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join("ambient")).unwrap();
    std::fs::create_dir_all(dir.join("out")).unwrap();
    std::fs::write(dir.join("src/slide.svg"), "<svg>hello</svg>\n").unwrap();
    std::fs::write(dir.join("ambient/fonts.txt"), "FONT_SET=v1\n").unwrap();
    dir
}

fn rasterize(dir: &Path, ambient: &[&str]) -> Resource {
    Resource {
        resource_type: ResourceType::Task,
        machine: MachineTarget::Single("local".to_string()),
        command: Some("cat src/slide.svg ambient/fonts.txt > out/slide.png".to_string()),
        working_dir: Some(dir.display().to_string()),
        task_inputs: vec!["src/slide.svg".to_string()],
        ambient_inputs: ambient.iter().map(|s| (*s).to_string()).collect(),
        output_artifacts: vec!["out/slide.png".to_string()],
        cache: true,
        ..Default::default()
    }
}

/// Run the recipe the way apply would, then record what the lock would hold.
fn apply_and_record(r: &Resource, dir: &Path) -> HashMap<String, serde_yaml_ng::Value> {
    let status = std::process::Command::new("bash")
        .arg("-c")
        .arg(r.command.as_deref().unwrap())
        .current_dir(dir)
        .status()
        .expect("run recipe");
    assert!(status.success(), "fixture recipe failed");
    let mut details = HashMap::new();
    record_io_hashes(r, &mut details);
    details
}

fn verdict(r: &Resource, details: &HashMap<String, serde_yaml_ng::Value>) -> Option<String> {
    staleness_reason(
        &probe_resource(r).expect("resource declares I/O"),
        details.get("input_hash").and_then(|v| v.as_str()),
        details.get("output_hash").and_then(|v| v.as_str()),
    )
}

/// FALSIFY-AMB-001: THE CASE. A declared ambient input changes; the task is
/// stale.
#[test]
fn an_ambient_change_makes_the_task_stale() {
    let dir = fixture("stale");
    let r = rasterize(&dir, &["cat ambient/fonts.txt"]);
    let details = apply_and_record(&r, &dir);
    assert_eq!(verdict(&r, &details), None, "settled right after apply");

    std::fs::write(dir.join("ambient/fonts.txt"), "FONT_SET=v2-NEW-FONT\n").unwrap();

    assert_eq!(
        verdict(&r, &details),
        Some("inputs changed".to_string()),
        "an ambient change is invisible — plan/check/drift/apply all report \
         clean over a stale artifact"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// FALSIFY-AMB-002: THE LOAD-BEARING ONE. Nothing changed; nothing is stale.
///
/// This catches the likeliest bad implementation — a probe that composites the
/// ambient value while `record_io_hashes` still records the file-only hash. It
/// would report "inputs changed" on every plan forever, which is worse than the
/// bug being fixed.
#[test]
fn an_unchanged_ambient_input_settles() {
    let dir = fixture("settle");
    let r = rasterize(&dir, &["cat ambient/fonts.txt"]);
    let details = apply_and_record(&r, &dir);
    for _ in 0..3 {
        assert_eq!(
            verdict(&r, &details),
            None,
            "an idempotency pump: the probe and the lock composite differently"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// FALSIFY-AMB-003: a fingerprint command that BREAKS must not collapse the
/// input hash back to the file-only value — that is "report clean over a stale
/// artifact", i.e. this exact bug, reintroduced the moment the fingerprint
/// stops working.
#[test]
fn a_failing_ambient_command_is_not_silently_dropped() {
    let dir = fixture("failing");
    let broken = rasterize(&dir, &["exit 7"]);
    let none = rasterize(&dir, &[]);

    let with_failure = probe_resource(&broken).unwrap().input_hash;
    assert!(
        with_failure.is_some(),
        "a broken fingerprint erased the hash"
    );
    assert_ne!(
        with_failure,
        probe_resource(&none).unwrap().input_hash,
        "a broken fingerprint collapsed to the no-ambient hash"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// FALSIFY-AMB-004: declaring no ambient inputs leaves the recorded hash
/// byte-identical, so upgrading forjar does not invalidate every existing lock
/// and rebuild the world once.
#[test]
fn no_ambient_inputs_leaves_the_recorded_hash_byte_identical() {
    let dir = fixture("compat");
    let r = rasterize(&dir, &[]);
    assert_eq!(
        hash_declared_inputs(&r, &dir),
        hash_inputs(&r.task_inputs, &dir).unwrap(),
        "the composite shape changed for resources that declare no ambient input"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// FALSIFY-AMB-005: a resource whose ONLY declared input is ambient is still
/// probed. Fails if the `hash_inputs` call is swapped but the guards stay keyed
/// on `task_inputs`.
#[test]
fn an_ambient_only_resource_is_still_probed() {
    let dir = fixture("ambient-only");
    let mut r = rasterize(&dir, &["echo v1"]);
    r.task_inputs.clear();

    let probe = probe_resource(&r).expect("an ambient-only resource must be probed");
    assert!(
        probe.input_hash.is_some(),
        "the fingerprint was computed and then never consulted"
    );

    let mut details = HashMap::new();
    record_io_hashes(&r, &mut details);
    assert!(
        details.contains_key("input_hash"),
        "the lock recorded nothing to compare the next probe against"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// FALSIFY-AMB-006: the declaration must survive template resolution. Its
/// sibling `task_inputs` shipped unresolved once (FJ-2721) and a config that
/// templated its inputs got a stale artifact under a green summary.
#[test]
fn ambient_inputs_are_template_resolved() {
    let mut params = HashMap::new();
    params.insert(
        "fp".to_string(),
        serde_yaml_ng::Value::String("fc-list".to_string()),
    );
    let r = Resource {
        resource_type: ResourceType::Task,
        ambient_inputs: vec!["{{params.fp}} | sha256sum".to_string()],
        ..Default::default()
    };
    let resolved =
        forjar::core::resolver::resolve_resource_templates(&r, &params, &indexmap::IndexMap::new())
            .expect("resolves");
    assert_eq!(resolved.ambient_inputs, vec!["fc-list | sha256sum"]);
}
