//! Regression guard (Refs #179): every shipped standalone example config must
//! pass `forjar validate`.
//!
//! Background: 4 `examples/*.yaml` configs went stale against the current schema
//! (unknown fields like `model_source`/`description`, privileged paths missing
//! `sudo: true`, a non-existent `pattern` policy field) and silently failed
//! `forjar validate`. This test sweeps the example set so a stale example can
//! never regress unnoticed again.
//!
//! Strategy: call the library validate path directly
//! (`parse_and_validate_opts(path, /*deny_unknown=*/ true)`) rather than shelling
//! out to the binary. This mirrors exactly what `forjar validate` does — unknown
//! fields are hard errors, then `validate_config` runs the semantic checks — while
//! staying deterministic and fully offline (validate never opens an SSH/network
//! connection). It also avoids depending on a built binary being present in CI.
//!
//! Scope: only top-level `examples/*.yaml` files that declare a `machines:` block
//! are treated as standalone, validatable configs. Files without `machines:` are
//! illustrative fragments (e.g. an `includes:`-only snippet) and are skipped, as
//! are recipe/cookbook fragments under subdirectories.
//!
//! Usage: cargo test --test examples_validate

use forjar::core::parser::parse_and_validate_opts;
use std::path::{Path, PathBuf};

/// Absolute path to the `examples/` directory in the crate root.
fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples")
}

/// True if the YAML declares a top-level `machines:` block — i.e. it is a
/// standalone config rather than an illustrative fragment.
fn declares_machines(contents: &str) -> bool {
    contents.lines().any(|line| line.trim_end() == "machines:")
}

/// Collect every top-level `examples/*.yaml` that declares a `machines:` block.
fn standalone_example_configs() -> Vec<PathBuf> {
    let dir = examples_dir();
    let mut configs: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", dir.display()))
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "yaml"))
        .filter(|path| {
            std::fs::read_to_string(path)
                .ok()
                .as_deref()
                .is_some_and(declares_machines)
        })
        .collect();
    configs.sort();
    configs
}

#[test]
fn examples_dir_has_standalone_configs() {
    // Guard against the sweep silently passing because it found nothing
    // (e.g. the directory moved or the glob broke).
    let configs = standalone_example_configs();
    assert!(
        configs.len() >= 40,
        "expected the standalone example sweep to find many configs, found {} — \
         did examples/ move or did `machines:` detection break?",
        configs.len()
    );
}

#[test]
fn every_standalone_example_validates() {
    let mut failures: Vec<String> = Vec::new();

    for path in standalone_example_configs() {
        // deny_unknown = true mirrors `forjar validate` exactly: unknown fields
        // are hard errors, then validate_config runs the semantic checks
        // (sudo inference, dependency cycles, type-specific rules, …).
        if let Err(e) = parse_and_validate_opts(&path, true) {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string());
            failures.push(format!("  {name}: {e}"));
        }
    }

    assert!(
        failures.is_empty(),
        "the following standalone example configs failed `forjar validate` \
         (fix the example or mark it as an intentionally-partial fragment by \
         removing its top-level `machines:` block):\n{}",
        failures.join("\n")
    );
}
