# Erratum for PMAT-721-lanes.md (round 7, ci/arch-neutral-lint)

Added after the fact; the frozen round-7 artifacts are NOT edited.

- `.quorum/evidence/PMAT-721-lanes.md` lines 14 and 17, and
  `.quorum/ci-arch-neutral-lint.json` line 69, cite claims C9-C11.
- `claims.md` and `judges.md` never define C9-C11. The defined set is C1-C8.
- Those citations are therefore dangling references, not evidence of
  additional claims having been judged.
- The receipt and the lane file are pinned by blob and sha256 in the receipt
  (`diff_sha256` bound to the round-7 diff). Rewriting either would falsify
  a historical record, so both are left byte-identical on purpose.

Recorded under PMAT-721 (review finding F5).
