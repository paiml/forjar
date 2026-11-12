# Implementation receipt — PMAT-989 phases 1-2 — `forjar build` layers its declared base and executes packages/command into a layer

verdict: PARTIAL — phases 1 and 2 are implemented and measured (F1 and F2 pass with real Docker on kfocus, twice); open: the `--sandbox` path, no CI job runs the container falsifier, executed-layer files are owned 0:0, and `--load` names are unresolvable on a containerd-image-store daemon.

## Identity

- ticket PMAT-989 (forjar#989, IMG-001 3/N); kind: code; branch `feat/989-image-base-and-exec`, base `main` @ 03375fcc
- phase 1 commits b9dbc7cf, 7e06e49f, 7d1360d4; phase 2 commits 58725d76 (implementation + fake-runtime tests), b3b0a0ff (container-test falsifier), e6360d0c (F2 by image id), and the docs commit that carries this receipt
- phase 2 files: `src/core/store/{build_runtime,exec_capture,exec_layer,fake_build_runtime,tests_exec_layer}.rs`, `src/core/store/mod.rs`, `src/core/types/resource.rs` (+`entrypoint`), `src/core/parser/{known_fields,resource_types,tests_known_fields_completeness}.rs`, `src/core/observe/classify_e01.rs`, `src/cli/{build_image,build_image_exec,build_image_cache,build_image_entries,build_image_base,build_image_report,mod,mod_test_decl_c,tests_build_image,tests_build_image_base,tests_build_image_exec}.rs`, `tests/falsification_image_exec.rs`, spec Phase 10, book 39 and 20, CHANGELOG, roadmap row

## Contract (phase 2)

An image resource with `packages` and/or `command` and a declared `image:` base runs them in one container started from that base: packages with the `forjar apply` package script (bash), then `command` (sh). The container's changes are captured (diff + export), normalised (single list in `exec_capture.rs`), sorted, epoch-mtime, 0:0, and appended as one layer after the base layers. `packages`/`command` without `image:` is an error. The base's diff_ids are checked against the runtime's copy; a mismatch is an error with the `docker pull` / `docker save <ref> | tar -x -C <dir>` / skopeo commands. `entrypoint: [..]` is the new entrypoint field; `command` is no longer the entrypoint. The cache key covers the steps and shells. `--json` carries `exec`. `--sandbox` is unchanged.

## Evidence

TDD note: phase 2 has NO RED commit. The implementation was written before its tests were committed; the tests were not observed failing against a prior tree. Phase 1's RED evidence (5 failed, then 33 passed) stands in the git history. This is a process gap, stated here.

Host kfocus (x86_64, Docker 29.1.3, containerd image store), through the capped wrapper, tree equal to 58725d76 for lib/clippy:

- `cargo fmt --all -- --check`: rc=0
- `cargo clippy --all-targets -- -D warnings`: rc=0; with `--features container-test`: rc=0 (also on the final test file)
- `cargo test --lib`: 13821 passed, 0 failed, 4 ignored (phase 1: 13792; +29). An earlier lib run failed 2 tests: `resource_fields_has_no_stale_entries` (fixed: `entrypoint` added to ALIASES) and `cli::tests_build_image::cmd_build_far_produces_valid_archive` ("read magic: failed to fill whole buffer"), a pre-existing race (two tests build resource "img" with `--far` into the same cwd-relative `state/images/img.far`); it passed on rerun. Both verdicts recorded; not touched.
- `falsification_image_assembler`: 30 passed; `falsification_container_builds`: 16 passed.

Falsifiers, `cargo test --features container-test --test falsification_image_exec -- --nocapture --test-threads=1`, base `debian@sha256:734a72b3d6142a71993b4bc580ca01d8ca687f77a2c422d96a961d3376b179bc`, `packages: [jq]`, a command writing `/etc/forjar-exec-marker`. Final tree (e6360d0c), two rounds in one run, both 2 passed / 0 failed:

- F1 round 1: run1 = run2 = `sha256:21e6349abad53505af4df5a44cb6ce312ea1256a25a63f31aeed322c059e629a`; round 2: identical value. The same digest was also seen in 3 earlier rounds. Executed layer: 29 files captured, 0 whiteouts; manifest has 2 layers, base first.
- F2 (loaded image run by id): `jq --version` rc=0 `jq-1.6`; marker file present: true, in both rounds. On gx10 (aarch64, overlay2) F2 run by tag also passed (jq-1.6, rc 0, marker true).
- D6 additions to the normalised list beyond the initial list: none were needed.

Not measured: `cargo test --workspace`, other integration targets, a clean-room run, the LAB reds of forjar#858 (not run), the falsifier in any CI job, a non-Debian base, a real `docker load` + run of an image with a non-root-owned file.

## Open

- `forjar build --sandbox` ignores base/packages/command; `LayerStrategy::Packages` marker arm is unconstructed by `forjar build`.
- No CI job runs `tests/falsification_image_exec.rs` (workflows not edited by this ticket).
- Executed-layer files are 0:0 (`LayerEntry` has no uid/gid).
- No registry pull in forjar; the layout is supplied by the user.
- containerd image store: `--load` lists `<tag>` but `docker run <tag>` / `docker image inspect <tag>` do not resolve it (assembler index annotation `io.containerd.image.name`, IMG-001 2/N, `image_assembler.rs`); F2 runs by id. Not fixed here.
- Package versions are unpinned: digest equality was measured within one day's repository snapshot, not across time.
- Scope touched beyond the briefed set: `src/core/observe/classify_e01.rs` (entrypoint classified Unobservable, forced by the `every_hashed_field_is_classified` gate; one-line revert), `src/core/parser/tests_known_fields_completeness.rs` (ALIASES), `src/core/parser/resource_types.rs` (`path` not required for an image with packages/command), `src/cli/tests_build_image.rs` fixture (`command:` became `entrypoint:`), and docs/book/src/20-platform-features.md.
- Phase 9 spec line untouched.

IMPL-PMAT-989-RECEIPT-END
