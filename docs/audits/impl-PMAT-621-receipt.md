# Implementation receipt — PMAT-621 — `store import docker` leaks one created container per call

verdict: PASS — the generated import script captures the id of the container it creates and removes it after the export, keeping the export's exit code; one new test pins the script and fails on main's provider

## Identity

- ticket PMAT-621 (forjar#621); kind: code
- PR #933, branch `ct3/621-docker-import-rm`, change commit cd4032f0
- files: `src/core/store/provider.rs` (+6 −1), `src/core/store/tests_provider.rs` (+22), `docs/roadmaps/roadmap.yaml` (row `GH-621` renamed to `PMAT-621`)

## Root cause (five whys)

1. 2658 stopped containers had piled up on one build host.
2. Every `store import docker` call created a container and never removed it.
3. The script was `docker create REF && docker export`: the id printed by `create` was dropped, so `export` had no id and nothing could `rm` it.
4. The script is built as one string and no test asserted on what it does with the container it creates.
5. The tests ran the real command, so each test run leaked too, and nothing counted containers.

## Contract and falsifier

Contract: the docker import script binds the created container id, exports from that id, runs `docker rm` on that id whether or not the export succeeds, and exits with the export's status.

Falsifier: `core::store::tests_provider::docker_import_removes_created_container`.

## Evidence (kfocus, `nice -n 19`, 2026-11-11, `cargo test --lib tests_provider`)

RED, with `src/core/store/provider.rs` taken from main:

    test core::store::tests_provider::docker_import_removes_created_container ... FAILED
    test result: FAILED. 51 passed; 1 failed; 0 ignored; 0 measured; 13716 filtered out

GREEN at cd4032f0 (forced rebuild from this tree):

    test core::store::tests_provider::docker_import_removes_created_container ... ok
    test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 13716 filtered out

## Not covered

- The test asserts on the generated script text. No run against a real docker daemon was made for this receipt, and the containers already leaked are not cleaned up by this change.

IMPL-PMAT-621-RECEIPT-END
