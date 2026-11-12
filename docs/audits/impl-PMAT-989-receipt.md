# Implementation receipt — PMAT-989 phases 1-2 — `forjar build` layers its declared base and executes packages/command into a layer

verdict: PARTIAL — phases 1 and 2 are implemented and measured (F1 and F2 pass with real Docker on kfocus, twice); open: the `--sandbox` path, no CI job runs the container falsifier, executed-layer files are owned 0:0, and `--load` names are unresolvable on a containerd-image-store daemon.

## Identity

- ticket PMAT-989 (forjar#989, IMG-001 3/N); kind: code; branch `feat/989-image-base-and-exec`, base `main` @ 03375fcc
- phase 1 commits b9dbc7cf, 7e06e49f, 7d1360d4; phase 2 commits 58725d76 (implementation + fake-runtime tests), b3b0a0ff (container-test falsifier), e6360d0c (F2 by image id), bd2700ac (docs + receipt), then the follow-ups 62cbd67b (RED test for unsafe base references), the fix commit, the YAML-quoting test fix, and the commit that updates this section
- phase 2 files: `src/core/store/{build_runtime,exec_capture,exec_layer,fake_build_runtime,tests_exec_layer}.rs`, `src/core/store/mod.rs`, `src/core/types/resource.rs` (+`entrypoint`), `src/core/parser/{known_fields,resource_types,tests_known_fields_completeness}.rs`, `src/core/observe/classify_e01.rs`, `src/cli/{build_image,build_image_exec,build_image_cache,build_image_entries,build_image_base,build_image_report,mod,mod_test_decl_c,tests_build_image,tests_build_image_base,tests_build_image_exec}.rs`, `tests/falsification_image_exec.rs`, spec Phase 10, book 39 and 20, CHANGELOG, roadmap row

## Contract (phase 2)

An image resource with `packages` and/or `command` and a declared `image:` base runs them in one container started from that base: packages with the `forjar apply` package script (bash), then `command` (sh). The container's changes are captured (diff + export), normalised (single list in `exec_capture.rs`), sorted, epoch-mtime, 0:0, and appended as one layer after the base layers. `packages`/`command` without `image:` is an error. The base's diff_ids are checked against the runtime's copy; a mismatch is an error with the `docker pull` / `docker save <ref> | tar -x -C <dir>` / skopeo commands. `entrypoint: [..]` is the new entrypoint field; `command` is no longer the entrypoint. The cache key covers the steps and shells. `--json` carries `exec`. `--sandbox` is unchanged.

## Evidence

TDD note: phase 2 had NO RED-first commit. The implementation was written before its tests were committed; the tests were not observed failing against a prior tree. See the mutation evidence section. Phase 1's RED evidence (5 failed, then 33 passed) stands in the git history. This is a process gap, stated here.

Host kfocus (x86_64, Docker 29.1.3, containerd image store), through the capped wrapper, tree equal to 58725d76 for lib/clippy:

- `cargo fmt --all -- --check`: rc=0
- `cargo clippy --all-targets -- -D warnings`: rc=0; with `--features container-test`: rc=0 (also on the final test file)
- `cargo test --lib`: 13821 passed, 0 failed, 4 ignored at bd2700ac (phase 1: 13792; +29); 13822 passed after the follow-up test. An earlier lib run failed 2 tests: `resource_fields_has_no_stale_entries` (fixed: `entrypoint` added to ALIASES) and `cli::tests_build_image::cmd_build_far_produces_valid_archive` ("read magic: failed to fill whole buffer"), a pre-existing race (two tests build resource "img" with `--far` into the same cwd-relative `state/images/img.far`); it passed on rerun. Both verdicts recorded; not touched.
- `falsification_image_assembler`: 30 passed; `falsification_container_builds`: 16 passed.

Falsifiers, `cargo test --features container-test --test falsification_image_exec -- --nocapture --test-threads=1`, base `debian@sha256:734a72b3d6142a71993b4bc580ca01d8ca687f77a2c422d96a961d3376b179bc`, `packages: [jq]`, a command writing `/etc/forjar-exec-marker`. Final tree (e6360d0c), two rounds in one run, both 2 passed / 0 failed:

- F1 round 1: run1 = run2 = `sha256:21e6349abad53505af4df5a44cb6ce312ea1256a25a63f31aeed322c059e629a`; round 2: identical value. The same digest was also seen in 3 earlier rounds. Executed layer: 29 files captured, 0 whiteouts; manifest has 2 layers, base first.
- F2 (loaded image run by id): `jq --version` rc=0 `jq-1.6`; marker file present: true, in both rounds. On gx10 (aarch64, overlay2) F2 run by tag also passed (jq-1.6, rc 0, marker true).
- D6 additions to the normalised list beyond the initial list: none were needed.

Not measured: `cargo test --workspace`, other integration targets, a clean-room run, the LAB reds of forjar#858 (not run), the falsifier in any CI job, a non-Debian base, a real `docker load` + run of an image with a non-root-owned file.

## Mutation evidence for phase 2 (stated plainly: phase 2 had no RED-first commit)

Phase 2 was written code-first: the implementation (58725d76) landed before its tests were observed failing, so there is no RED-first commit for it. In its place, three one-line mutations of `src/core/store/exec_capture.rs` were applied on a COPY of the tree on host kfocus (the worktree was never mutated; nothing mutated was committed), the exec lib tests were run, and the file was restored from a backup copy before the next one. Command (after a no-mutation baseline: 23 passed, 0 failed):

    cargo test --lib -- tests_exec_layer tests_build_image_exec

| mutation | caught by | assertion site |
|---|---|---|
| (a) capture ignores `C` (changed) diff entries: `want.insert(..)` only when `d.kind != DiffKind::Changed` | `core::store::tests_exec_layer::captures_added_and_changed_files_symlinks_and_deletions` (1 failed, 22 passed) | `tests_exec_layer.rs:82` |
| (b) no whiteouts for `D` paths: `top_level_deletes(&BTreeSet::new())` | `a_deleted_directory_gets_one_whiteout_not_one_per_child` (`left: 0`, `tests_exec_layer.rs:216`) and `captures_added_and_changed_files_symlinks_and_deletions` (`tests_exec_layer.rs:96`) (2 failed, 21 passed) | as listed |
| (c) normalised-prefix list empty: `NORMALISED_DIRS = &[]` | `caches_logs_and_scratch_are_not_in_the_layer` (1 failed, 22 passed) | `tests_exec_layer.rs:106` |

No mutation left the tests green, so no test was added for them. Not mutated: `NORMALISED_FILES` and `NORMALISED_SUFFIXED`, hardlink resolution, the fail-closed branches, the diff_ids check.

## Base reference option injection (follow-up)

`res.image` reaches the container runtime as a bare argument (`<rt> image inspect <ref>`, `<rt> run .. <ref>`). `load_declared_base` now refuses, before any lookup, a reference that is empty, starts with '-', or contains whitespace or a control character; the error names the reference (Debug-quoted) and carries no fetch hint (`base_fetch_hint` is reachable only after this check, from the refuse closure and from `build_exec_layer` on an already-loaded base). RED first: commit 62cbd67b, `t8_unsafe_base_references_are_refused_before_any_lookup`, kfocus, `cargo test --lib tests_build_image_base::t8`: FAILED at `tests_build_image_base.rs:260` (`expect_err`: the build succeeded on base `""` although a layout existed). GREEN after the fix: 1 passed; full lib 13822 passed, 0 failed, 4 ignored; fmt rc=0; clippy `--all-targets -- -D warnings` rc=0 (kfocus). The first GREEN attempt failed at the test's own quoting of DEL for YAML (test bug, fixed in its own commit), not in the fix. `--sandbox` (`cmd_build_sandbox`) does not go through `load_declared_base` and is not covered by this check.

## Open

- `forjar build --sandbox` ignores base/packages/command and the base-reference check; `LayerStrategy::Packages` marker arm is unconstructed by `forjar build`.
- No CI job runs `tests/falsification_image_exec.rs` (workflows not edited by this ticket).
- Executed-layer files are 0:0 (`LayerEntry` has no uid/gid).
- No registry pull in forjar; the layout is supplied by the user.
- containerd image store: `--load` lists `<tag>` but `docker run <tag>` / `docker image inspect <tag>` do not resolve it (assembler index annotation `io.containerd.image.name`, IMG-001 2/N, `image_assembler.rs`); F2 runs by id. Not fixed here.
- Package versions are unpinned: digest equality was measured within one day's repository snapshot, not across time.
- Scope touched beyond the briefed set: `src/core/observe/classify_e01.rs` (entrypoint classified Unobservable, forced by the `every_hashed_field_is_classified` gate; one-line revert), `src/core/parser/tests_known_fields_completeness.rs` (ALIASES), `src/core/parser/resource_types.rs` (`path` not required for an image with packages/command), `src/cli/tests_build_image.rs` fixture (`command:` became `entrypoint:`), and docs/book/src/20-platform-features.md.
- Phase 9 spec line untouched.

IMPL-PMAT-989-RECEIPT-END
