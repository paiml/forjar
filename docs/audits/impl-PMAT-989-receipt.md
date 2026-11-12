# Implementation receipt — PMAT-989 phase 1 — `forjar build` layers the base image its config declares

verdict: PARTIAL — phase 1 is done and measured (declared base layered, absent base is an error, base keys the cache); phase 2 (executing `packages` / `command` into a layer) is not started.

## Identity

- ticket PMAT-989 (forjar#989, IMG-001 3/N); kind: code; phase 1 of 2
- branch `feat/989-image-base-and-exec`, base `main` @ 03375fcc
- files: `src/cli/build_image.rs`, `src/cli/build_image_base.rs` (new), `src/cli/build_image_report.rs` (new, code moved out of `build_image.rs`), `src/cli/mod.rs`, `src/cli/mod_test_decl_c.rs`, `src/cli/tests_build_image_base.rs` (new), `docs/specifications/platform/05-container-builds.md`, `CHANGELOG.md`, `docs/roadmaps/roadmap.yaml` (row PMAT-989 label/status/notes), this receipt

## Root cause (five whys)

1. `forjar build` on a config with `image: ubuntu:22.04` and no base layout on disk wrote a scratch image and exited 0.
2. `cmd_build` called `assemble_image(..)`, which is `assemble_image_with_base(.., None)`; the base was never passed.
3. `build_plan_from_resource` read `state/images/<base>` only to print `format_base_info`, then dropped the result; a missing or unreadable layout was silently skipped (`if base_dir.exists()`, `if let Ok(..)`).
4. `assemble_image_with_base(.., Some(..))` was written and tested (IMG-001 1/N, 2/N) against hand-built fixtures, but no test drove `cmd_build` with a declared base, so nothing connected the tested assembler to the CLI.
5. The build-cache key (`compute_layer_input_hash`) covered only the file entries, and the only observable of a missing base was a printed line that was absent, so no gate or test compared the declared config with the built manifest.

## Contract and falsifiers

Contract: an image resource with `image: <ref>` builds on the layout at `<state>/images/<ref with ':' and '/' as '_'>` (one function, `base_layout_dir`, used by the lookup and the error text). The manifest lists the base layers first, then the built layers; the base blobs are in the output layout. An absent, unreadable or incomplete base is an `Err` naming the reference, the directory and `skopeo copy docker://<ref> oci:<dir>`, raised before the output directory is created. Without `image:` the build is unchanged. The base's layer digests, diff_ids, arch, os, runtime config and history are part of the cache key. Same config + same base into two empty output dirs gives the same manifest digest.

Falsifiers (`src/cli/tests_build_image_base.rs`, all through the real `cmd_build_in` path with a private state dir):

- `t1_base_layers_come_first_and_their_blobs_are_in_the_output`
- `t2_declared_base_without_a_layout_fails_closed_and_writes_nothing`
- `t3_base_with_a_missing_blob_fails_naming_the_digest`
- `t4_same_config_and_base_give_the_same_manifest_digest_in_two_output_dirs`
- `t5_cache_keys_on_the_base_identity` (a deleted `index.json` is the witness of a cache hit)
- `t6_no_image_field_builds_a_scratch_image_as_before`
- `t7_report_json_carries_the_base_reference_and_layer_count`

## Evidence (host t2yoga, rust 1.93.0 toolchain via `~/src/fc-pre/fc-pre.sh`, 2026-11-12)

RED, commit b9dbc7cf (tests + behaviour-preserving seam), `cargo test --lib tests_build_image`: 28 passed, 5 failed.

    t2 ... a declared base that is absent must be an error, not a scratch image: ()
    t3 ... a base with a missing blob must be refused: ()
    t1 ... assertion `left == right` failed: base layer + one built layer   left: 1  right: 2
    t5 ... changing the base must invalidate the build cache
    t7 ... assertion `left == right` failed   left: Null  right: "ubuntu:22.04"
    t4, t6 passed on unchanged behaviour (guards)

GREEN, commit 7e06e49f, same command: 33 passed, 0 failed.

On the tree at 7e06e49f (before this receipt and the roadmap note were added), same host:

- `cargo fmt --all -- --check`: rc=0
- `cargo clippy --all-targets -- -D warnings`: rc=0
- `cargo test --lib`: 13792 passed, 0 failed, 4 ignored
- `cargo test --test falsification_image_assembler --test falsification_container_builds`: 30 passed and 16 passed, 0 failed

Not measured: the other integration targets, `cargo test --workspace`, any real `skopeo copy` of a registry image into the layout and a `docker load` of the result, a clean-room run. The known LAB reds of forjar#858 were not run.

## Open

- Phase 2: `packages` / `command` are not executed into a layer; `LayerStrategy::Packages` still writes only a marker file. Nothing here changes that.
- `forjar build --sandbox` (`cmd_build_sandbox`) was not changed and does not layer the declared base.
- There is still no registry pull; the layout must be put on disk by the user.

IMPL-PMAT-989-RECEIPT-END
