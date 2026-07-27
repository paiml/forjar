//! Tests: FJ-2710 (PMAT-197) world-derived staleness probe.
//!
//! These pin the behaviour that makes forjar's incremental build claim TRUE.
//! Before this, a task whose sources changed planned as `NoOp` and forjar
//! reported `Apply complete: 0 converged, N unchanged` over a stale artifact.

use super::probe::*;
use crate::core::types::{Resource, ResourceType};

fn task_with(inputs: &[&str], outputs: &[&str], working_dir: Option<&str>) -> Resource {
    Resource {
        resource_type: ResourceType::Task,
        task_inputs: inputs.iter().map(|s| s.to_string()).collect(),
        output_artifacts: outputs.iter().map(|s| s.to_string()).collect(),
        working_dir: working_dir.map(|s| s.to_string()),
        ..Default::default()
    }
}

#[test]
fn resource_declaring_no_io_is_not_probed() {
    // Cheap skip for the overwhelming majority of infra resources.
    assert!(probe_resource(&task_with(&[], &[], None)).is_none());
}

#[test]
fn base_dir_is_working_dir_not_cwd() {
    // A build file declares paths relative to its project root. Hashing
    // against anything else (the old code used state_dir.parent()) makes every
    // relative path hash as absent, which silently disables the whole feature.
    let r = task_with(&["src/a.c"], &["build/a.o"], Some("/proj"));
    assert_eq!(probe_base_dir(&r), std::path::PathBuf::from("/proj"));
    assert_eq!(
        probe_base_dir(&task_with(&["a"], &[], None)),
        std::path::PathBuf::from(".")
    );
}

#[test]
fn absolute_artifact_paths_are_not_rebased() {
    let base = std::path::Path::new("/proj");
    assert_eq!(
        resolve_under(base, "/etc/passwd"),
        std::path::PathBuf::from("/etc/passwd")
    );
    assert_eq!(
        resolve_under(base, "build/x.o"),
        std::path::PathBuf::from("/proj/build/x.o")
    );
}

#[test]
fn missing_output_forces_rebuild() {
    // `rm build/demo` must rebuild. This previously reported `unchanged`, and
    // `forjar check` actively reported a PASS on the deleted artifact.
    let probe = IoDigest {
        input_hash: Some("same".into()),
        output_hash: None,
        outputs_missing: true,
    };
    assert_eq!(
        staleness_reason(&probe, Some("same"), Some("whatever")),
        Some("output artifact missing".to_string())
    );
}

#[test]
fn changed_inputs_force_rebuild() {
    let probe = IoDigest {
        input_hash: Some("new".into()),
        output_hash: Some("out".into()),
        outputs_missing: false,
    };
    assert_eq!(
        staleness_reason(&probe, Some("old"), Some("out")),
        Some("inputs changed".to_string())
    );
}

#[test]
fn unchanged_io_is_not_stale() {
    // The other half of the claim: a no-op re-apply must be a genuine no-op,
    // otherwise this is an always-rebuild loop wearing a build system's hat.
    let probe = IoDigest {
        input_hash: Some("same".into()),
        output_hash: Some("out".into()),
        outputs_missing: false,
    };
    assert_eq!(staleness_reason(&probe, Some("same"), Some("out")), None);
}

#[test]
fn absent_recorded_hash_rebuilds_once_to_establish_baseline() {
    // A resource converged under an older forjar, or before it declared
    // inputs, has no recorded hash. Assuming it is current would be a guess;
    // re-running once is cheap and provably correct.
    let probe = IoDigest {
        input_hash: Some("h".into()),
        output_hash: None,
        outputs_missing: false,
    };
    assert_eq!(
        staleness_reason(&probe, None, None),
        Some("no recorded input hash".to_string())
    );
}

#[test]
fn modified_output_is_detected() {
    // Someone edited the artifact out of band; the declared build is no longer
    // what is on disk.
    let probe = IoDigest {
        input_hash: Some("same".into()),
        output_hash: Some("tampered".into()),
        outputs_missing: false,
    };
    assert_eq!(
        staleness_reason(&probe, Some("same"), Some("original")),
        Some("output artifact modified".to_string())
    );
}

#[test]
fn remote_resources_are_never_probed() {
    // HONESTY GATE: probing runs on the controller. Hashing this host's files
    // on behalf of a remote target would compare the wrong tree and produce a
    // confidently wrong build decision. Skipping preserves the old
    // config-hash behaviour instead of inventing an answer.
    let mut resources = indexmap::IndexMap::new();
    let mut r = task_with(&["src/a.c"], &["build/a.o"], Some("/proj"));
    r.machine = crate::core::types::MachineTarget::Single("remote".to_string());
    resources.insert("t".to_string(), r);

    let probed = probe_all(&resources, |_| false);
    assert!(
        probed.is_empty(),
        "remote resources must not be probed against the controller filesystem"
    );
}

#[test]
fn real_filesystem_roundtrip_detects_content_change() {
    // End-to-end over a real temp dir: the hash must move when CONTENT moves.
    let dir = std::env::temp_dir().join(format!("forjar-probe-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join("build")).unwrap();
    std::fs::write(dir.join("src/a.c"), "int a(void){return 1;}").unwrap();
    std::fs::write(dir.join("build/a.o"), "OBJ-V1").unwrap();

    let r = task_with(&["src/a.c"], &["build/a.o"], dir.to_str());

    let first = probe_resource(&r).expect("declares I/O");
    assert!(!first.outputs_missing);
    assert!(first.input_hash.is_some() && first.output_hash.is_some());

    // Same content → same digest (idempotent).
    assert_eq!(probe_resource(&r).unwrap(), first);

    // Changed content → different input digest.
    std::fs::write(dir.join("src/a.c"), "int a(void){return 999;}").unwrap();
    let after = probe_resource(&r).unwrap();
    assert_ne!(
        after.input_hash, first.input_hash,
        "content change must move the input hash"
    );
    assert_eq!(
        staleness_reason(&after, first.input_hash.as_deref(), None),
        Some("inputs changed".to_string())
    );

    // Deleted output → missing, and that beats every other signal.
    std::fs::remove_file(dir.join("build/a.o")).unwrap();
    assert!(probe_resource(&r).unwrap().outputs_missing);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn directory_artifact_is_identified_by_existence_not_contents() {
    // v1.11.0 REGRESSION: hashing a directory's CONTENTS created an
    // idempotency pump. The canonical translation of make's `| build`
    // order-only prerequisite declares `output_artifacts: ["build"]`, and the
    // next rule writes build/a.o INTO it — so apply #2 saw "output artifact
    // modified" and re-ran the whole graph, violating f(f(x)) = f(x).
    let dir = std::env::temp_dir().join(format!("forjar-dirart-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("build")).unwrap();

    let r = task_with(&[], &["build"], dir.to_str());
    let before = probe_resource(&r).expect("declares an output");
    assert!(!before.outputs_missing, "the directory exists");

    // A later rule drops a product into the directory.
    std::fs::write(dir.join("build/a.o"), "OBJECT BYTES").unwrap();
    let after = probe_resource(&r).unwrap();

    assert_eq!(
        after, before,
        "a directory artifact must not change identity when other rules write into it"
    );
    assert_eq!(
        staleness_reason(&after, None, before.output_hash.as_deref()),
        None,
        "writing into the directory must not make its creator stale"
    );

    // Deleting it still counts as missing — existence is the signal.
    std::fs::remove_dir_all(dir.join("build")).unwrap();
    assert!(probe_resource(&r).unwrap().outputs_missing);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mixed_file_and_directory_artifacts_still_track_the_file() {
    // Descoping directories must not silently descope files declared next to
    // them — that would turn the fix into a hole.
    let dir = std::env::temp_dir().join(format!("forjar-mixart-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("build")).unwrap();
    std::fs::write(dir.join("build/bin"), "V1").unwrap();

    let r = task_with(&[], &["build", "build/bin"], dir.to_str());
    let first = probe_resource(&r).unwrap();
    assert!(
        first.output_hash.is_some(),
        "the file artifact is still hashed"
    );

    std::fs::write(dir.join("build/bin"), "V2").unwrap();
    let second = probe_resource(&r).unwrap();
    assert_ne!(
        second.output_hash, first.output_hash,
        "a modified FILE artifact must still be detected"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
