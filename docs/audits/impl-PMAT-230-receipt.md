# Implementation receipt — PMAT-230 — every `gh release download` overwrites what the previous release left, and the two other fixed /tmp paths are cleared

verdict: PASS — the v1.28.0 release was blocked at `dist-artifacts` by ``/tmp/SHA256SUMS already exists (use `--clobber` to overwrite file or `--skip-existing` to skip file)``, a file left by a previous release on a runner that is not ephemeral; all three `gh release download` call sites now pass `--clobber`, the two other fixed `/tmp` paths release.yml writes into are cleared before they are written, and rules 9 and 10 of the workflow shape test hold both properties — red together against main's workflows (8 passed, 2 failed) and green here (10 passed), with twelve mutation cases measured one at a time. Three review rounds, nine lanes, **none of which passed**: every refutation that reproduced was fixed and the fix measured, and the ones deliberately not acted on are named. Not claimed: that the v1.28.0 release is published — that is the next step, and gate R reports it PENDING until it is.

orch_model: opus [A]   orch_class: code   orch_decision: admit   orch_basis: release
fable_binding: false   quota_age_h: absent   quota_mark: ?   k_measured_at_set: refused(R-5)

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1  class=impl  route=agy-goal  w=1.00  basis=absent  note=fable-binding  effort=1[U]  bucket_collision=true  (executed by self: workers and lanes may not edit .github/workflows)
  ph2.quorum   class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]  (delegate, three lanes on 469cc9a9)
  ph2.quorum2  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]  (delegate, three lanes on b02d26b4)
  ph2.quorum3  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]  (delegate, three lanes on 5d1920f6)
  ph3  class=orchestration  route=self  w=100.00  basis=absent

verification:
  cmd="cargo test --test falsification_release_workflow_shape, with main's workflows checked out (RED: 8 passed, 2 failed) then at HEAD (GREEN: 10 passed)"  claimed_exit=101(lanes)  rerun_exit=101/0  log_path=docs/audits/logs/PMAT-230-gate-tests.log  sha256=89d336da7db663d2
  cmd="twelve mutation cases: seven evasions of rule 9, two innocent commands that must stay green, three mutations of rule 10's guards"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-230-rule9-mutations.log  sha256=67b7a028d8ce7803

## What broke, and where it had already been written down

The v1.28.0 release (tag `v1.28.0`, cut 2026-09-10T16:07:14Z, crates.io and docs.rs both serving) stopped at `dist-artifacts`: `gh release download … --dir /tmp` exited 1 because `/tmp/SHA256SUMS` was already there from an earlier release on the same clean-room runner. `publish-release` was skipped and the GitHub release stayed a draft prerelease with thirteen assets and no installer.

The lesson was already in the file, one job over. `checksums` clears its staging directory first, under a comment that reads *THE STAGING DIR IS A FIXED PATH ON A RUNNER THAT IS NOT EPHEMERAL* and recalls v1.18.0 shipping a `SHA256SUMS` of ten lines, four of them belonging to 1.17.0. `dist-artifacts` and `homebrew` never got the guard.

`--skip-existing` is refused as firmly as absence, and it is the worse of the two: it exits 0 leaving the previous release's checksums in place, the step's own `test -s /tmp/SHA256SUMS` passes, and `forjar dist --checksums-file` embeds them into `install.sh`. That is the v1.18.0 failure again with a friendlier exit code.

## The change

| where | what |
|---|---|
| release.yml `dist-artifacts` | `--clobber` on the checksums fetch; `rm -rf /tmp/dist-output` before `dist --output-dir /tmp/dist-output`, whose whole contents are uploaded as the release's artifact |
| release.yml `homebrew` | `--clobber` on its checksums fetch; `rm -rf /tmp/tap` before `git clone`, which exits 128 over an existing directory |
| binary-release.yml `checksums` | `--clobber`, with a comment saying plainly that it is **not** load-bearing there (hosted runner, `mkdir assets` first) and why it carries the flag anyway |
| rule 9 | every `gh release download` in every workflow passes `--clobber`, none passes `--skip-existing`, none is guarded with `||`, and fewer than three call sites is itself a failure |
| rule 10 | `dist-artifacts` and `homebrew` clear the fixed `/tmp` directory they write into, and the clear precedes the write |

## Falsification

Rules 9 and 10 are red together against main's workflows and green here (`docs/audits/logs/PMAT-230-gate-tests.log`). Twelve cases, each applied alone to release.yml's `dist-artifacts` call site and reverted (`docs/audits/logs/PMAT-230-rule9-mutations.log`):

| case | rule 9 |
|---|---|
| flags before the subcommand (`gh release -R … download`) | RED |
| a backslash splitting `release` / `download` | RED |
| `\|\| true` appended | RED |
| a backslash splitting `gh` / `release` | RED |
| a trailing `# --clobber` comment | RED |
| `--pattern "*--clobber*"` | RED |
| a second command lending its flag across `&&` | RED |
| an innocent `gh release upload --title download` | green |
| an unrelated `gh api --pattern release download` | green |

and for rule 10: each of the two clears dropped, and one moved after the write it guards — all three RED.

**The battery caught a defect in the rule one edit after it was written**: cutting the joined command at a bare `|` also cut it at `||`, silently disarming the assertion that refuses a suppressed failure. Case 3 went from red to green and said so. No lane found that; the instrument did, which is the argument for the battery over a fourth round of prose.

## Review record — three rounds, none of them green

| round | commit | lanes | what they refuted |
|---|---|---|---|
| 1 | 469cc9a9 | 0/3 PASS | the rule matched the literal string `gh release download`, so `gh release -R repo download` walks past it; the claim misquoted gh's error; the flag on binary-release.yml is not needed there |
| 2 | b02d26b4 | 1/3 PASS | a trailing `# --clobber` comment and `--pattern "*--clobber*"` both satisfied a substring check; a backslash between `gh` and `release` walked past the finder; `gh release upload --title download` was flagged as a download |
| 3 | 5d1920f6 | 0/3 PASS | `release` was read as any token after `gh`; two commands on one continued line lent each other flags; **and `homebrew` clones into the fixed path `/tmp/tap`, so this fix would have moved the failure two steps rather than removed it** |

Every one of those reproduced when re-run and every one is closed, each with a mutation case. Round 3's last finding is the one that mattered most: `homebrew` has been failing since v1.27.0 at the checksums download two steps above the clone, so the clone had never been reached and nothing had ever reported it.

Two round-3 findings are deliberately **not** acted on, and are named rather than dropped: a malformed `--repo` with no value, and a `gh` token inside a quoted string. Neither exists in this repository, both fail closed or fail loudly, and closing them means parsing shell inside a test whose whole point is to be cheaper than that. The rule's own comment says it is a text ratchet and names what it cannot catch.

**No round passed, and this receipt does not claim one did.** What closes the ticket is the twelve-case battery and the red/green pair, not a green lane.

## Gates measured

| gate | result |
|---|---|
| `cargo test --test falsification_release_workflow_shape` | 10 passed at HEAD; 8 passed / 2 failed against main's workflows |
| twelve mutation cases | 10 red as designed, 2 innocent commands green |
| `actionlint` over every workflow | the finding SET is identical to main's (22 both sides; only two in-script line numbers move). actionlint has never exited 0 here — it does not know the `clean-room` runner label — so the measurement is the diff of the finding sets, not the exit code |
| YAML parse | both edited workflows parse |
| `scripts/dogfood/tagged.sh` | GATE T PASS on the branch |
| `pmat analyze vacuous-tests --path tests` | 43 of 4713 tree-wide, 0 in the touched test file |
| gate F's mutation arm | nothing under `src/` changes, so the arm has nothing to mutate — a measured zero, not a skip (PMAT-216 stays open) |
| pre-commit | format, complexity, clippy, SATD green on every commit; every commit carries `Pmat-Ticket: PMAT-230` |

## Gaps, named

- The v1.28.0 GitHub release is still a draft prerelease. This branch makes the re-dispatch of `release.yml` from main able to finish; it does not itself publish anything, and gate R reports the release, crates.io and docs.rs arms separately (crates.io and docs.rs are already green for 1.28.0).
- A re-run of the OLD workflow run cannot pick this up: GitHub re-runs a run against the workflow file of its original commit, so the release must be re-dispatched (`workflow_dispatch`, `tag: v1.28.0`) from main after this merges.
- The two unclosed parser holes above, and the general property "no fixed `/tmp` path is written without being cleared" — rule 10 asserts it for the two paths that have one, by name, not as a general rule over shell.
- `homebrew` is not on `publish-release`'s `needs`, so its failure never blocked a release; it has simply been silently broken since at least v1.27.0.

IMPL-PMAT-230-RECEIPT-END
