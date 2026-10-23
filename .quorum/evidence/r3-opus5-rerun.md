# Quorum evidence - r3 - opus5 rerun

Fresh rerun of the claude-opus-5 lane only (Claude Code CLI, --model claude-opus-5, no tools, no bypass flags) on commit 761d37f2, delta git diff origin/epic/forjar-img-001...761d37f2, lane prompt from r3-lanes.md plus the refuted-claims evidence from r3-claims.md. The lane was not told a verdict. Verbatim output follows.

---

## Lane review — forjar #776 r3 (commit 761d37f2)

I tried to break each of (a)–(d) from the text. No blocking defect found; findings below, worst first.

### Claim (a) — arch rule judged per matrix leg: **HELD**
`unpinned_in` (arch.rs:33 region) iterates `runner_legs(job)` and tests `is_fleet(&leg) && !has_arch(&leg)` on each leg independently, so a sibling leg's `X64` cannot vouch for a bare `[self-hosted, clean-room]`. The fixture `a_matrix_with_one_pinned_leg_and_one_unpinned_leg_fails` asserts exactly one finding, with a matching all-pinned control. The replaced `main.rs` body did flatten via `runner_labels`, so the hole was real and is shut. I could not construct a leg shape that evades it while containing `self-hosted` and no arch label.

Consistency cross-check that could have exposed a lie: lint.yml's two clean-room legs are arch-less in the *context* lines (pre-existing from the epic), so under per-leg judging they must already be ARCH_NEUTRAL or the real-workflow test would be red. The author reports 17/17 green and the `a_self_hosted_neutral_job_and_a_mixed_matrix_are_not_stale` control models precisely that shape. Coherent.

### Claim (b) — neutral job on a hosted runner is stale: **HELD**
`stale_in` pushes a finding when `!legs.iter().any(is_fleet)`. For `runs-on: ubuntu-latest`, `runner_legs` yields one non-fleet leg → flagged; the fixture asserts len == 1 and the mixed-matrix control stays silent. The previous `seen`-tuple logic recorded only `has_arch`, so a hosted job genuinely passed. Shut.

### Claim (c) — ci.yml / proofs.yml edits are load-bearing: **HELD, but the evidence proves coupling, not necessity** *(low)*
Reverting the two `runs-on` lines while keeping the two new ARCH_NEUTRAL entries makes `stale_in` emit "carries an arch label" → red. That is a true measurement, but it shows the workflow text and the list must agree; reverting *both* halves would also be green. So "not cosmetic" is right — the workflow edit is the product change and the guard tracks it — but the falsification does not show the YAML hunks are required by anything independent of the list the same PR adds. Doesn't change the verdict.

### Claim (d) — check names unchanged, #772 cannot recur: **first half held; "cannot recur" overreaches** *(medium, non-blocking)*
Held, for a reason the diff gets right: a job's check name comes from `name:` or the job id, never from `runs-on`. `ledger-replay` pins `name: ledger-replay`; `dogfood-surface` has no `name:` and so is named by its id. Dropping `X64` from those two cannot rename either. For lint.yml, `name: ${{ matrix.check }}` pins four distinct strings transcribed from main's check-runs, which is the correct direction (branch protection holds main's names, and the epic's auto-names had already drifted).

Where it overreaches:

1. **No guard that every matrix leg defines `check`.** A future `include:` entry, or a top-level matrix dimension sharing a key, yields a leg whose `name:` renders empty — the same silent-rename class as #772, now with an empty name rather than a changed one. opus5 raised the empty-element worry and the judges killed it *as a present-tense claim* (main's four check-runs match); the forward-looking half survives and is recorded as a follow-up. In a repo whose own rule is that every guard ships with the address of its falsifier, this guard's absence is the one thing I'd want before calling recurrence impossible.
2. **The mechanism is unguarded elsewhere.** release.yml and nightly.yml have templated `runs-on: ${{ matrix.runner }}` with (per the receipt's own refutation #3) no `name:` pin mentioned. If their `runner` values are ever edited, their check names move the same way. Not required checks today, so not a hang — but the lesson of #772 is written down for lint.yml only, not generalised.

### Other findings
- **Pinned names assert `X64` on legs that no longer pin it** *(medium, deliberate)*. `clippy (self-hosted, clean-room, X64)` can now execute on an ARM64 clean-room runner. The comment says the label is a name, not a pin, and renaming would hang the required check — the right trade — but the name is now actively misleading to a human reading a red check, and nothing records the debt in the workflow beyond that comment.
- **Two new ARCH_NEUTRAL entries are assertions no test can falsify** *(low)*. "dogfood surface checks, no artifact" and "ledger replay over its own build" are judgement calls about chip-independence; the guard only verifies the entries are *consistent*, never that they are *true*. The whole instrument's value rests on that honesty, and this PR enlarges the trusted list by two.
- **`runner_legs` label fidelity** *(low)*. The key is parsed by locating `matrix.` inside a label, then every `${{`-bearing label is dropped from `literal`. A composite label such as `${{ matrix.os }}-latest` loses its literal suffix, and a key shared by `include:` and a top-level dimension has its values concatenated (duplicate/odd legs). Neither affects a fleet leg, which must contain `self-hosted`; latent. Matches flash/argon's non-Cartesian and duplicate-value notes.
- **Cache keys have no guard** *(low)*. `runner.arch` lands in all five keys and in coverage's sole `restore-keys` (I confirmed from the diff that the other four files show a single `key:` line each, so opus5's refuted finding #2 is correctly dead). But nothing tests that a new cache key includes `runner.arch`, so this fix is a point-in-time edit, not an instrument.
- **Receipt base vs reviewed text** *(low, disclosed)*. `diff_sha256`/`base_commit` are over the merge-base with origin/main while the lanes reviewed the delta against the epic; a verifier recomputing the hash sees more than any lane saw. The `note` says so plainly and the history shows the receipt was rebound to the final diff, so this is auditability friction, not concealment.
- **Family margin thinner than "not degraded" implies** *(informational)*. Two of four lanes and one of three judges are Claude, versus Claude 5.5 authors. The written rule is about model *id*, which is met (`claude-sonnet-5` / `claude-opus-5` ≠ the 5.5 ids), and two non-Claude agy families participated, so the label is defensible — but the independence is id-level, not family-level, for half the panel.

Gate E's predicate reads satisfied on the receipt as committed: 4 lanes, 3 judges, 4 refuters per claim, 3 refuted claims, five evidence files with sizes/hashes/blobs, no waiver or override key. The planted-defect control fired (argon named the dropped clean-room label), which is the only reason I'd trust the clean PASSes.

Claims (a), (b) and (c) survive my attempts to refute them. (d) survives as a statement about today's four names and the two renamed-immune jobs; its stronger form — recurrence impossible — is not supported, and the two gaps above are its address. Nothing here blocks the merge.

VERDICT PASS
