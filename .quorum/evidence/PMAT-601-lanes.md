# Lanes — impl receipts for the v1.32.0 window (PMAT-601)

THIS TABLE IS THE ONLY PLACE ROUNDS ARE COUNTED AND HEADS ARE NAMED.

2 agy + 1 claude, read-only. The Claude lane verified every merge sha, runner name
and quorum count itself against `gh` and the `.quorum/*.json` files.

| round | head | agy gemini-3.1-pro-high | agy gemini-3.1-pro-low | claude sonnet |
|---|---|---|---|---|
| 1 | fcda9163 | FAIL conv-dc3fae6e | PASS conv-f87f3c3d | FAIL |

## Findings, adjudicated

- **Claude, real:** the PMAT-601 receipt cited its own quorum receipt before that
  file existed. The citation is to the artefact this quorum produces, committed in
  the same PR; the receipt now says so instead of reading as an ordinary citation.
- **Claude, real, and outside the receipts:** `.quorum/PMAT-592-release-1.32.0.json`
  (merged in #593) said "Four confirmed claims, nine refuted" in its prose while its
  own structured fields say 3 and 10. Written before C8 moved to refuted as R10.
  Corrected here; the file is excluded from the diff hash, so the binding holds.
- **agy-high, real:** "four findings: two missing labels, three rows" — 2 + 3 = 5.
  The cut's own commit message made the same miscount; the receipt is now exact.
- **Both agy lanes, real in kind:** "gate A reports 6 of 6" appeared nowhere they
  could read. True, and uncheckable. The measured gate output is now quoted in the
  PMAT-601 receipt.
- **agy-high, not real:** C5 refuted because the roadmap diff edits PMAT-598, a
  `kind:code` row. The triage rail is a set of PATHS, and `roadmap.yaml` is on it;
  a row's label is not the rail. The claim is now worded as paths.
