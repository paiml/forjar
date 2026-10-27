# Erratum for PMAT-721-lanes.md (round 7, ci/arch-neutral-lint)

Added after the fact; this file records the defect without rewriting the round-7 files.

- `.quorum/evidence/PMAT-721-lanes.md` lines 14 and 17, and
  `.quorum/ci-arch-neutral-lint.json` line 69, cite claims C9-C11.
- `claims.md` and `judges.md` never define C9-C11. The defined set is C1-C8.
- Those citations are therefore dangling references, not evidence of
  additional claims having been judged.
- This PR leaves the round-7 files (`PMAT-721-lanes.md`,
  `.quorum/ci-arch-neutral-lint.json`) at their ec1d5741 bytes and records the
  C9-C11 defect here, in this erratum.

Recorded under PMAT-721 (review finding F5).
