//! forjar#423: forjar's contract code — the `#[contract]` macro, the
//! `build_helper` that verifies `contracts/binding.yaml`, the `traits` the
//! tests use — must come from crates INSIDE this repository, not from
//! crates.io and not from a sibling checkout.
//!
//! WHAT WAS OBSERVABLY WRONG. `Cargo.toml` pulled `aprender-contracts` and
//! `aprender-contracts-macros` from the registry, and the shared CI still
//! cloned an ARCHIVED sibling repository "for pv codegen" that nothing here
//! read. When GitHub began answering the fleet's IP with 401 for anonymous
//! git, every PR went red on a fetch of code forjar did not use (#422).
//!
//! WHY THESE ASSERTIONS. They read the manifests and the lockfile as bytes —
//! the same artefacts `cargo` reads — rather than trusting a build that
//! happened to have the registry cache warm. With main's `Cargo.toml`
//! restored, the first two go RED (registry deps, registry lockfile entries);
//! the third is a link-time fact: this test compiles only if
//! `provable_contracts` resolves through the workspace.

use std::path::Path;

fn read(rel: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

/// Every contract dependency is a `path` dependency into `crates/`, and no
/// `aprender-contracts*` registry dependency remains.
#[test]
fn the_contract_crates_are_path_dependencies() {
    let manifest = read("Cargo.toml");
    assert!(
        !manifest.contains("aprender-contracts"),
        "Cargo.toml still names a registry contract crate:\n{}",
        manifest
            .lines()
            .filter(|l| l.contains("aprender-contracts"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    for dep in ["forjar-contracts-macros", "forjar-contracts"] {
        let lines: Vec<&str> = manifest
            .lines()
            .filter(|l| l.trim_start().starts_with(&format!("{dep} = ")))
            .collect();
        assert!(!lines.is_empty(), "{dep} is not a dependency at all");
        for l in lines {
            assert!(
                l.contains(&format!("path = \"crates/{dep}\"")),
                "{dep} must be a path dependency into crates/: {l}"
            );
        }
    }
}

/// The lockfile records the in-tree crates with no registry source.
#[test]
fn the_lockfile_carries_no_registry_copy_of_the_contract_crates() {
    let lock = read("Cargo.lock");
    assert!(
        !lock.contains("name = \"aprender-contracts"),
        "Cargo.lock still resolves a registry contract crate"
    );
    for name in ["forjar-contracts-macros", "forjar-contracts"] {
        let idx = lock
            .find(&format!("name = \"{name}\"\n"))
            .unwrap_or_else(|| panic!("{name} is not in Cargo.lock"));
        let block = &lock[idx..lock[idx..].find("\n\n").map_or(lock.len(), |e| idx + e)];
        assert!(
            !block.contains("source = \"registry+"),
            "{name} resolves from a registry, not the workspace:\n{block}"
        );
    }
}

/// The library names forjar imports are unchanged, and they link from the
/// workspace: this test exists only if `provable_contracts` resolves.
#[test]
fn the_vendored_crates_keep_the_library_names_forjar_imports() {
    let core = read("crates/forjar-contracts/Cargo.toml");
    let macros = read("crates/forjar-contracts-macros/Cargo.toml");
    assert!(
        core.contains("name = \"provable_contracts\""),
        "core [lib] name changed"
    );
    assert!(
        macros.contains("name = \"provable_contracts_macros\""),
        "macros [lib] name changed"
    );
    // Link-time proof: the workspace crate is what `provable_contracts` is.
    let _ = provable_contracts::build_helper::BindingPolicy::AllImplemented;
}
