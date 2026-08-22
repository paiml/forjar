//! GH-246: byte-identity is not the only way two artifacts can be "the same".
//!
//! The sharp case, and the one these tests are built around: a human-corrected
//! transcript. An ASR pass writes `narration.srt`, a person edits it, and under
//! byte-equivalence that edit reads as staleness — so the next apply
//! regenerates the machine draft over the human's work. The artifact is not
//! uncached; it is content-addressed with the wrong key, which is the failure
//! that corrupts rather than merely misses.

use forjar::core::task::hash_outputs_with;
use forjar::core::types::OutputEquivalence;
use indexmap::IndexMap;
use std::path::Path;

fn write(dir: &Path, name: &str, body: &str) {
    std::fs::write(dir.join(name), body).unwrap();
}

fn rules(pairs: &[(&str, OutputEquivalence)]) -> IndexMap<String, OutputEquivalence> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), v.clone()))
        .collect()
}

#[test]
fn bytes_is_the_default_and_is_unchanged() {
    // The existing behaviour must be exactly preserved for everything that did
    // not opt in — recompiling to identical bytes correctly does not relink.
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "out.o", "OBJECT\n");
    let arts = vec!["out.o".to_string()];

    let empty = IndexMap::new();
    let before = hash_outputs_with(&arts, dir.path(), &empty).unwrap();
    assert!(before.is_some());

    // Same bytes -> same hash.
    assert_eq!(
        before,
        hash_outputs_with(&arts, dir.path(), &empty).unwrap(),
        "byte-equivalence must stay deterministic"
    );

    // Different bytes -> different hash.
    write(dir.path(), "out.o", "OBJECT v2\n");
    assert_ne!(
        before,
        hash_outputs_with(&arts, dir.path(), &empty).unwrap(),
        "a changed artifact must still be staleness under `bytes`"
    );
}

#[test]
fn an_external_artifact_survives_a_human_edit() {
    // THE CASE FROM THE ISSUE. A person corrects the ASR draft; that must not
    // read as staleness, or the producer overwrites the correction.
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "narration.srt", "ASR draft\n");
    let arts = vec!["narration.srt".to_string()];
    let rule = rules(&[("narration.srt", OutputEquivalence::External)]);

    let before = hash_outputs_with(&arts, dir.path(), &rule).unwrap();

    write(dir.path(), "narration.srt", "Human corrected transcript\n");
    let after = hash_outputs_with(&arts, dir.path(), &rule).unwrap();

    assert_eq!(
        before, after,
        "a human edit to an `external` artifact must not register as staleness"
    );
    assert!(
        !OutputEquivalence::External.producer_may_overwrite(),
        "and the producer must not be allowed to overwrite it"
    );
}

#[test]
fn a_missing_artifact_is_still_staleness_under_every_predicate() {
    // `none` and `external` say "do not key on my content" — NOT "stop tracking
    // me". A predicate that swallowed a missing output would turn an escape
    // hatch into a hole.
    let dir = tempfile::tempdir().unwrap();
    let arts = vec!["gone.srt".to_string()];

    for rule in [
        OutputEquivalence::Bytes,
        OutputEquivalence::None,
        OutputEquivalence::External,
    ] {
        write(dir.path(), "gone.srt", "here\n");
        let present =
            hash_outputs_with(&arts, dir.path(), &rules(&[("gone.srt", rule.clone())])).unwrap();
        std::fs::remove_file(dir.path().join("gone.srt")).unwrap();
        let absent =
            hash_outputs_with(&arts, dir.path(), &rules(&[("gone.srt", rule.clone())])).unwrap();

        assert_ne!(
            present,
            absent,
            "a missing artifact must remain distinguishable under `{}`",
            rule.as_str()
        );
    }
}

#[test]
fn changing_the_declaration_changes_the_hash() {
    // Flipping an artifact from `external` back to `bytes` must not look
    // identical to never having declared it — otherwise a config change that
    // re-enables content-keying would produce no re-apply, and the artifact
    // would stay keyed by the old predicate forever.
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.txt", "X\n");
    let arts = vec!["a.txt".to_string()];

    let as_none = hash_outputs_with(
        &arts,
        dir.path(),
        &rules(&[("a.txt", OutputEquivalence::None)]),
    )
    .unwrap();
    let as_external = hash_outputs_with(
        &arts,
        dir.path(),
        &rules(&[("a.txt", OutputEquivalence::External)]),
    )
    .unwrap();
    let as_bytes = hash_outputs_with(&arts, dir.path(), &IndexMap::new()).unwrap();

    assert_ne!(as_none, as_bytes, "`none` must differ from `bytes`");
    assert_ne!(as_external, as_bytes, "`external` must differ from `bytes`");
    assert_ne!(
        as_none, as_external,
        "`none` and `external` are different declarations and must not collide"
    );
}

#[test]
fn a_normaliser_command_absorbs_an_irrelevant_difference() {
    // Structural equivalence: two renders differing only in an embedded
    // timestamp are the same artifact. The normaliser strips it; forjar stays
    // out of the media-format business.
    let dir = tempfile::tempdir().unwrap();
    let arts = vec!["render.txt".to_string()];
    let rule = rules(&[(
        "render.txt",
        OutputEquivalence::Command("grep -v '^generated:' \"$1\"".to_string()),
    )]);

    write(dir.path(), "render.txt", "generated: 111\nbody\n");
    let first = hash_outputs_with(&arts, dir.path(), &rule).unwrap();

    write(dir.path(), "render.txt", "generated: 999\nbody\n");
    let second = hash_outputs_with(&arts, dir.path(), &rule).unwrap();
    assert_eq!(
        first, second,
        "a difference the normaliser strips must not read as staleness"
    );

    // ...but a real change must still register, or the escape is a hole.
    write(dir.path(), "render.txt", "generated: 999\nDIFFERENT BODY\n");
    assert_ne!(
        first,
        hash_outputs_with(&arts, dir.path(), &rule).unwrap(),
        "a real content change must still be staleness"
    );
}

#[test]
fn a_failing_normaliser_is_an_error_not_a_silent_fallback() {
    // Falling back to byte-comparison would reintroduce exactly the spurious
    // staleness the author declared the normaliser to avoid, and would do it
    // silently.
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "r.txt", "x\n");
    let arts = vec!["r.txt".to_string()];
    let rule = rules(&[("r.txt", OutputEquivalence::Command("exit 7".to_string()))]);

    let err = hash_outputs_with(&arts, dir.path(), &rule).unwrap_err();
    assert!(
        err.contains("output_equivalence"),
        "the error must name the mechanism that failed: {err}"
    );
}

// ── End to end, through the binary ──────────────────────────────────────
//
// The unit tests above prove the predicate. These prove the DECLARATION
// reaches it: a field can be on `Resource`, hash correctly, and still be
// rejected by the parser as an unknown field — which is exactly what happened
// on the first run of this work.

use std::process::Command;

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

/// Config for the ASR-transcript scenario, with or without the escape declared.
fn transcript_project(dir: &Path, external: bool) -> std::path::PathBuf {
    let work = dir.join("work");
    std::fs::create_dir_all(&work).unwrap();
    let decl = if external {
        "    output_equivalence:\n      narration.srt: external\n"
    } else {
        ""
    };
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            r#"version: "1.0"
name: oe
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  transcribe:
    type: task
    machine: local
    working_dir: "{}"
    cache: true
    task_inputs: []
    output_artifacts: ["narration.srt"]
{decl}    command: |
      printf 'ASR draft\n' > narration.srt
"#,
            work.display()
        ),
    )
    .unwrap();
    cfg
}

fn apply(cfg: &Path, state: &Path) {
    let out = forjar()
        .args([
            "apply",
            "-f",
            cfg.to_str().unwrap(),
            "--state-dir",
            state.to_str().unwrap(),
            "--no-tripwire",
            "--yes",
        ])
        .output()
        .expect("apply runs");
    assert!(
        out.status.success(),
        "apply failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Apply, let a human edit the artifact, apply again, return the final content.
fn human_edit_then_reapply(dir: &Path, external: bool) -> String {
    let state = dir.join("state");
    std::fs::create_dir_all(&state).unwrap();
    let cfg = transcript_project(dir, external);
    apply(&cfg, &state);

    let artifact = dir.join("work").join("narration.srt");
    std::fs::write(&artifact, "Human corrected transcript\n").unwrap();
    apply(&cfg, &state);
    std::fs::read_to_string(&artifact).unwrap()
}

#[test]
fn without_the_declaration_a_human_correction_is_destroyed() {
    // The bug, reproduced. Kept as a test so the fix is verified DIFFERENTIALLY
    // — "the artifact survived" proves nothing unless the default destroys it.
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        human_edit_then_reapply(dir.path(), false),
        "ASR draft\n",
        "precondition: byte-equivalence must overwrite the human edit, or the \
         next test proves nothing"
    );
}

#[test]
fn declaring_external_preserves_the_human_correction() {
    // THE FIX, end to end and through the parser.
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        human_edit_then_reapply(dir.path(), true),
        "Human corrected transcript\n",
        "`output_equivalence: external` must stop the producer overwriting a \
         human-authoritative artifact"
    );
}
