# Lanes — forjar#683

One counted round at head d2210701, 14m budget. Three lanes from two
families, none the author's model (claude-opus-5-5). Each lane was
read-only and had the full diff and the ticket.

- claude-sonnet-5: PASS, two non-blocking findings. The test's WORK_ROOT
  constant is one runner's layout; the run id and failure history in the
  doc comment are not checkable from the diff.
- agy gemini-3.1-pro-high, sandboxed: PASS. Confirmed the RUSTUP_HOME
  placement and the discovery of every cross job.
- claude-haiku-4-5: PASS, no findings.

Sonnet's two findings, answered rather than changed:

- The WORK_ROOT constant is one runner's layout. Both mounted paths are
  written relative to `${{ github.workspace }}`, so the verdict does not
  depend on which absolute root the constant names; it only needs a root
  that `$HOME` is not under, which is the containerized runner's shape.
- The run id in the test's doc comment is not checkable from the diff. It is
  also in the commit message and issue #683, and stays, as #611's test keeps
  its run history.


Round 2, head 91b12a6a (main 46ee342d merged in; the branch's diff was
byte-identical to round 1's): AGREED, 3/3 PASS.

- claude-sonnet-5: PASS, four non-blocking notes. (1) WORK_ROOT was a
  personal home path naming one machine: changed to `/srv/runner-work` by
  1d3e2d45, the verdict unchanged (red with the workflows at 61d5a703,
  green restored). (2) Discovery reads inline `run:` text, so a job calling
  cross through a script would evade it; none exists today, and that limit
  is recorded here. (3) The receipt's measured results cannot be re-run from
  the diff: they are recorded with the commands that produced them.
  (4) `diff_sha256` holds a 40-hex digest: the field name is the gate's,
  not this branch's.
- agy gemini-3.1-pro-high, sandboxed: PASS, no findings.
- claude-haiku-4-5: PASS, no findings.

Round 3, head 1d3e2d45 (the scrub): NOT AGREED, sonnet FAIL.

- claude-sonnet-5: FAIL. A per-job RUSTUP_HOME is empty, and
  release.yml:build-binaries installed a toolchain only on macOS, so its
  Linux legs reached `rustup target add` and `cargo metadata` with none; the
  path test could not see it; the receipt still named machines. All three
  were right: C5, C6 and the receipt's removal answer them.
- agy gemini-3.1-pro-high, sandboxed: PASS.
- claude-haiku-4-5: PASS.

Round 4, head a86201fa (C5 and C6): NOT AGREED, sonnet and gemini FAIL.

- claude-sonnet-5: FAIL. The receipt still recorded 3 tests and claims
  C1–C4, so it covered neither the install steps nor the new falsifier.
  Right: the stale receipt is removed, C5 and C6 are claimed and measured
  above, and the receipt is rebuilt from the round at the final head.
- agy gemini-3.1-pro-high, sandboxed: FAIL on the same stale receipt, and
  "the toolchain install is outside the ticket": refuted as R3.
- claude-haiku-4-5: PASS.

Round 5, head 930fe3e1 (the evidence for C5 and C6; the stale receipt
removed): AGREED, 3/3 PASS.

- claude-sonnet-5: PASS, no findings.
- agy gemini-3.1-pro-high, sandboxed: PASS. Confirmed RUSTUP_HOME under
  the work root at release.yml:234, the Linux toolchain install before the
  first cargo or rustup at release.yml:269, and the falsifier's two checks.
- claude-haiku-4-5: PASS.
