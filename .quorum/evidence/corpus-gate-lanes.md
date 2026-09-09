# Quorum evidence — PMAT-217 — lane rulings

Three sandboxed review lanes on the diff at `ae43a884`, each in its own `git clone --shared` copy with an empty `CARGO_HOME`. Verdicts 3 of 3 FAIL. Every finding was re-run before anything changed.

## The lanes contradicted each other, and that was the useful part

Asked where the old ratchet figures were left behind, all three answered "measured" and named different places:

| lane | claim |
|---|---|
| 1 | `crates/forjar-contracts/Cargo.toml` and `VENDORED.md` still say 38 |
| 2 | a repository-wide search confirms the constants are left behind nowhere |
| 3 | `docs/roadmaps/roadmap.yaml` lines 2016 and 2712 still say 38 and 43 |

At most one could be right. Re-grepping settled it: lane 2 was wrong, and lanes 1 and 3 each found real sites the other had missed. One of lane 3's two is a `status: completed` row whose title records what was true when it was filed, and it stays as filed, because rewriting a finished ticket to match a later number falsifies the record rather than fixing it.

A reduced consensus would have shown one number and hidden the disagreement. The disagreement is what found the sites.

## Two findings all three lanes shared, and both were right

**The attribute parser was dead code.** The first version of this suite had a rule that read `#[cfg_attr]` attributes; when that rule was replaced by the ratchet invariant, the parser stayed and its output was bound to `_`. Removed. The helper is `test_bodies` now and its own doc records why it collects only bodies.

**The sibling rule was evadable one call deep.** It matched string shapes inside `#[test]` bodies, so moving the check into a helper hid it — and `cross_project_tests.rs` was passing the rule by exactly that accident. An exemption that holds by accident is not an exemption; it is a gap that happens to be empty, and the next file to use a helper would have inherited it silently. The scan is file-wide now and `cross_project_tests.rs` is exempt BY NAME with its reason, with a case that fails if it ever stops reaching for the sibling.

Measured after the change: the same defect written through a helper, in a non-exempt file, is caught. It was not before.

## One single-lane finding, also right

Lane 2 charged that the module docstring claimed semantics the code does not have: "narrow on purpose: it is about querying aprender's KERNELS", when the rule reads text and knows nothing about kernels. Corrected. The docstring now says it is a text ratchet and names the evasions it cannot catch, which is the only honest thing a string-matching rule can say about itself.

## What the lanes did not do

None ran cargo. The delegate forbade it, having measured 213G of build output against 220G free and citing the earlier incident where a lane filled the disk. So every test count in the receipt is the orchestrator's own run, and the lanes' figures come from grep. Two of them independently counted 39 annotations by grep, which agrees with the measured 39 failures under `--features aprender-corpus`.
