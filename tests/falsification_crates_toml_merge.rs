//! forjar's `.crates.toml` merge must not corrupt multi-line entries.
//!
//! forjar#345. `_fj_register` merged the staging `.crates.toml` into
//! `$CARGO_HOME/.crates.toml` line by line, and cargo writes MULTI-LINE arrays
//! for any crate installing more than one binary:
//!
//!   `head -1` on the SOURCE        -> a multi-line entry contributed only its
//!                                     `"kani-verifier ..." = [` line; the body
//!                                     and closing `]` were dropped
//!   `grep -v "^\"$key "` on the    -> removed only the KEY line of the entry
//!   DESTINATION                       being replaced, orphaning its body
//!
//! Both fired on paiml's intel. cargo rejects the WHOLE file for one bad entry:
//!
//!   error: failed to parse crate metadata at `~/.cargo/.crates.toml`
//!   Caused by: invalid TOML found for metadata
//!
//! Every `stack-tool-*` resource then failed `missing:<tool>` while every
//! binary was present and runnable, and on a host whose $HOME is shared by
//! sixteen CI runners `cargo install --list` returned nothing for all of them.
//!
//! THIS TEST RUNS THE GENERATED SHELL. Asserting on script text is how `forjar
//! check` passed everything for five months while dozens of
//! `script.contains(...)` tests were green — the script was never executed.

use std::fs;
use std::process::Command;

/// Extract every `_fj_*` shell function from a generated install script.
///
/// Deliberately name-agnostic: these tests must measure the merge's BEHAVIOUR,
/// not the presence of any particular helper. A script that defines only
/// `_fj_register` contributes only `_fj_register` and still runs.
///
/// The `\n\` continuations in the generator strip leading whitespace, so every
/// emitted line is unindented and a bare `}` at column 0 unambiguously closes a
/// function body.
fn shell_prelude(script: &str) -> String {
    let mut out = String::new();
    let mut rest = script;
    while let Some(open) = rest.find("() {") {
        let line_start = rest[..open].rfind('\n').map_or(0, |i| i + 1);
        let name = &rest[line_start..open];
        let is_helper =
            name.starts_with("_fj_") && name.bytes().all(|b| b.is_ascii_lowercase() || b == b'_');
        if !is_helper {
            rest = &rest[open + 4..];
            continue;
        }
        let end = line_start
            + rest[line_start..]
                .find("\n}")
                .expect("unterminated shell function")
            + 2;
        out.push_str(&rest[line_start..end]);
        out.push('\n');
        rest = &rest[end..];
    }
    assert!(
        out.contains("_fj_register() {"),
        "generated script must define _fj_register"
    );
    out
}

fn generated_script() -> String {
    let r = forjar::core::types::Resource {
        resource_type: forjar::core::types::ResourceType::Package,
        provider: Some("cargo".to_string()),
        packages: vec!["kani-verifier".to_string()],
        ..Default::default()
    };
    forjar::resources::package::apply_script(&r)
}

/// THE DEFECT, executed. A multi-line entry must survive a merge intact, and
/// the unrelated multi-line entry beside it must not be damaged.
#[test]
fn merging_a_multi_line_entry_leaves_valid_toml() {
    let dir = std::env::temp_dir().join("forjar-345-merge");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("sandbox");

    let dest = dir.join("crates.toml");
    fs::write(
        &dest,
        "[v1]\n\
         \"bat 0.26.1 (registry+https://github.com/rust-lang/crates.io-index)\" = [\"bat\"]\n\
         \"cross 0.2.5 (registry+https://github.com/rust-lang/crates.io-index)\" = [\n    \"cross\",\n    \"cross-util\",\n]\n\
         \"kani-verifier 0.66.0 (registry+https://github.com/rust-lang/crates.io-index)\" = [\n    \"cargo-kani\",\n    \"kani\",\n]\n\
         \"ripgrep 15.1.0 (registry+https://github.com/rust-lang/crates.io-index)\" = [\"rg\"]\n",
    )
    .expect("dest");

    let src = dir.join("staging.toml");
    fs::write(
        &src,
        "[v1]\n\"kani-verifier 0.67.0 (registry+https://github.com/rust-lang/crates.io-index)\" = [\n    \"cargo-kani\",\n    \"kani\",\n]\n",
    )
    .expect("src");

    let runner = dir.join("run.sh");
    fs::write(
        &runner,
        format!(
            "#!/bin/sh\n_CRATES_TOML=\"{}\"\n{}\n_fj_register \"{}\"\n",
            dest.display(),
            shell_prelude(&generated_script()),
            src.display()
        ),
    )
    .expect("runner");

    let out = Command::new("sh").arg(&runner).output().expect("run merge");
    assert!(
        out.status.success(),
        "merge failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let merged = fs::read_to_string(&dest).expect("read merged");

    assert!(
        merged.contains("kani-verifier 0.67.0"),
        "new entry absent:\n{merged}"
    );
    assert!(
        !merged.contains("kani-verifier 0.66.0"),
        "old entry survived:\n{merged}"
    );
    assert!(merged.contains("cargo-kani"), "binaries dropped:\n{merged}");
    assert!(
        merged.contains("\"cross\",") && merged.contains("\"cross-util\","),
        "an UNRELATED multi-line entry was damaged — this is the orphan bug:\n{merged}"
    );

    // cargo rejects the entire file for one bad entry, so validity is the
    // property that matters, not the presence of any single line.
    let parsed: toml::Table = merged
        .parse()
        .unwrap_or_else(|e| panic!("merge produced INVALID TOML ({e}):\n{merged}"));
    let v1 = parsed
        .get("v1")
        .and_then(|v| v.as_table())
        .expect("v1 table");
    assert_eq!(
        v1.len(),
        4,
        "expected 4 entries, got {}:\n{merged}",
        v1.len()
    );

    let _ = fs::remove_dir_all(&dir);
}

/// A single-line entry must still work — the common case, and the one the old
/// line-oriented merge handled correctly.
#[test]
fn merging_a_single_line_entry_still_works() {
    let dir = std::env::temp_dir().join("forjar-345-single");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("sandbox");

    let dest = dir.join("crates.toml");
    fs::write(&dest, "[v1]\n\"bat 0.26.0 (registry+https://github.com/rust-lang/crates.io-index)\" = [\"bat\"]\n").expect("dest");
    let src = dir.join("staging.toml");
    fs::write(&src, "[v1]\n\"bat 0.26.1 (registry+https://github.com/rust-lang/crates.io-index)\" = [\"bat\"]\n").expect("src");

    let runner = dir.join("run.sh");
    fs::write(
        &runner,
        format!(
            "#!/bin/sh\n_CRATES_TOML=\"{}\"\n{}\n_fj_register \"{}\"\n",
            dest.display(),
            shell_prelude(&generated_script()),
            src.display()
        ),
    )
    .expect("runner");
    let out = Command::new("sh").arg(&runner).output().expect("run merge");
    assert!(out.status.success());

    let merged = fs::read_to_string(&dest).expect("read");
    let parsed: toml::Table = merged.parse().expect("valid TOML");
    let v1 = parsed.get("v1").and_then(|v| v.as_table()).expect("v1");
    assert_eq!(
        v1.len(),
        1,
        "a reinstall must update, not duplicate:\n{merged}"
    );
    assert!(merged.contains("bat 0.26.1"), "not upgraded:\n{merged}");

    let _ = fs::remove_dir_all(&dir);
}

/// THE READ-BACK THAT WAS MISSING (forjar#345, suggested fix 3).
///
/// The merge above is entry-aware now, but a correct merge INTO wreckage is
/// still wreckage. An older forjar left `$CARGO_HOME/.crates.toml` with an
/// orphaned array body and an unclosed key, and cargo rejects the WHOLE file
/// for one bad entry.
///
/// `mv` cannot fail on content, so the commit reported CONVERGED while every
/// `stack-tool-*` resource on the host read `missing:<tool>` and every binary
/// kept running. The write was never read back.
#[test]
fn a_merge_into_an_unparseable_registry_is_refused_not_committed() {
    let dir = std::env::temp_dir().join("forjar-345-unparseable");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("sandbox");

    // The exact wreckage measured on paiml's intel: `cross` intact, then an
    // orphaned array body with no key, then a `kani-verifier` key whose array
    // is never closed.
    let dest = dir.join("crates.toml");
    fs::write(
        &dest,
        r#"[v1]
"bat 0.26.1 (registry+https://github.com/rust-lang/crates.io-index)" = ["bat"]
"cross 0.2.5 (registry+https://github.com/rust-lang/crates.io-index)" = [
    "cross",
    "cross-util",
]
    "cargo-kani",
    "kani",
]
"ripgrep 15.1.0 (registry+https://github.com/rust-lang/crates.io-index)" = ["rg"]
"kani-verifier 0.67.0 (registry+https://github.com/rust-lang/crates.io-index)" = [
"#,
    )
    .expect("dest");
    let before = fs::read(&dest).expect("snapshot");

    // A perfectly valid single-entry source for an UNRELATED crate.
    let src = dir.join("staging.toml");
    fs::write(&src, "[v1]\n\"copia 0.1.6 (registry+https://github.com/rust-lang/crates.io-index)\" = [\"copia\"]\n").expect("src");

    let runner = dir.join("run.sh");
    fs::write(
        &runner,
        format!(
            "#!/bin/sh\n_CRATES_TOML=\"{}\"\n{}\n_fj_register \"{}\"\n",
            dest.display(),
            shell_prelude(&generated_script()),
            src.display()
        ),
    )
    .expect("runner");

    let out = Command::new("sh").arg(&runner).output().expect("run merge");

    assert!(
        !out.status.success(),
        "a merge leaving metadata cargo cannot parse must FAIL the apply, not \
         report success — stdout: {} stderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    assert_eq!(
        fs::read(&dest).expect("re-read"),
        before,
        "the destination was mutated; forjar must not append to a registry \
         cargo cannot read"
    );

    let strays: Vec<String> = fs::read_dir(&dir)
        .expect("list sandbox")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("crates.toml.forjar."))
        .collect();
    assert!(
        strays.is_empty(),
        "rejected temp file left behind: {strays:?}"
    );

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(&dest.display().to_string()),
        "the operator must be told WHICH file: {stderr}"
    );

    let _ = fs::remove_dir_all(&dir);
}
