# pmat lane — PMAT-159 / v1.25.2 — the mechanical check

Tool: `analyze_vacuous_tests`, run as `pmat analyze vacuous-tests -p . -f json` from
the worktree root at the commit under review. pmat 3.37.0. The scan is REPO-WIDE by
necessity — a single-file `-p` is refused by the tool — and the result below is the
whole-repository output filtered to the four test files this branch touches.

## Raw output (the counters, verbatim from the JSON)

    "files_parsed": 2101
    "tests_examined": 19315
    "vacuous": 360 entries
    "conditional_skips": 4 entries
    "skipped": { "unreadable": [], "unparseable": [], "unmeasured_tests": 0 }

`unmeasured_tests: 0` matters more than the 360: it says nothing in the tree was
skipped unread, so the zero below is a measurement and not an absence.

## Filtered to the touched paths

    vacuous_tests_in_touched_paths = 0

Neither the `vacuous` list nor the `conditional_skips` list contains any entry whose
`file` is `tests/falsification_version_matches_manifest.rs`,
`tests/falsification_quorum_anchors_release_shaped.rs`,
`tests/falsification_quorum_gate_reads_the_pushed_ref.rs` or
`tests/integration_smoke.rs`. The filter was applied over both lists, because a test
that silently skips backs no claim either.

## WHY THREE TESTS ARE NAMED IN `pmat.accepted` DESPITE THAT ZERO

The zero is what the tool measured, not a verdict that every touched test can fail.
`analyze vacuous-tests` reads the test BODY: it asks whether the assertions in the
function can ever be false. The three tests below have assertions that can be false —
so the tool is right to pass them — and are still non-discriminating under the command
CI runs, because the FAILING STATE IS REPAIRED BY CARGO BEFORE THE BODY EXECUTES. That
is a property of the invocation, not of the source, and it is outside what a body-level
detector can see. The quorum measured it by hand and the file says so itself at
`tests/falsification_version_matches_manifest.rs:38`; recording it here keeps the
receipt from reporting a clean tool run as if it were a clean suite.

- `the_compiled_version_is_the_manifest_version`
  (`tests/falsification_version_matches_manifest.rs:132`) — compares the manifest text
  on disk to `CARGO_PKG_VERSION`. Under a plain `cargo test` cargo REBUILDS the test
  binary from that same manifest first, so the two agree by construction. It
  discriminates only against an out-of-band binary — one built before the bump.

- `the_built_binary_reports_the_manifest_version`
  (`tests/falsification_version_matches_manifest.rs:149`) — same repair, one step out:
  cargo rebuilds the `forjar` binary from the bumped manifest before the assertion runs,
  so `forjar --version` cannot disagree with it. It discriminates against an ALREADY
  BUILT or installed binary, which is the case this fleet keeps meeting, and not under
  the command that rebuilds.

- `the_lockfile_records_the_manifest_version`
  (`tests/falsification_version_matches_manifest.rs:177`) — pins the #131 invariant, and
  the enforcer is cargo under `--locked`, not this body: without the flag cargo silently
  REPAIRS a stale `Cargo.lock` and the assertion then reads a lockfile that was fixed a
  second ago; with it, cargo refuses at resolution and the test never starts. The test
  is the message-bearing witness for an invariant `--locked` enforces, and it goes red
  on its own only when the already-built test binary is run directly.

The fourth leg, `the_changelog_has_an_entry_for_this_version`
(`tests/falsification_version_matches_manifest.rs:194`), IS discriminating under a plain
`cargo test`: nothing in the toolchain writes `CHANGELOG.md`, so nothing can repair it
underneath the assertion. It is the leg that went red when `Cargo.toml:3` was bumped
alone.

## Limit of this lane

A denylist of body shapes cannot prove a test is discriminating; it can only report the
shapes it recognises. The three accepted entries above were found by the quorum reading
the invocation, not by the tool, which is the honest description of how much this lane
proves.
