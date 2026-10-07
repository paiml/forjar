# Lanes — forjar#607

Lanes ran through quorum-review.sh. None used the author's model
(claude-opus-5-5). Each lane was read-only, with the full diff and the ticket.
The diff touches release tooling, so the one agy lane ran. An advisory local
lane was recorded as unavailable (busy) in both rounds and counts toward
nothing.

Round 1, head 150e0fac: NOT AGREED.

- claude-sonnet-5: FAIL. The v1.33.0 row names
  `docs/audits/dogfood-1.33.0-receipt.md`, which is not in the tree, so gate T
  (rule T5) fails on the branch's own state. Confirmed as C6, and answered by
  ffff31c9, which puts it in the ticket's acceptance criteria: the receipt was
  not measured, so it is not written, and gate T stays red until it is.
- gemini-3.1-pro-high (agy): FAIL. The same missing receipt, and the crux
  audit's claim that the #611 fix appends `$CARGO_HOME/bin` to `$GITHUB_PATH`
  is not backed by the diff. Refuted as R1: the fix shipped in v1.33.0 and is
  not this branch's. ffff31c9 makes the audit cite where it is.
- claude-haiku-4-5: NO-VERDICT. The lane's envelope was an error; its text
  says it had already returned PASS with no findings, but the structured
  verdict did not arrive, so it counts as nothing.

Round 2, head ffff31c9: AGREED, 3/3 PASS.

- claude-sonnet-5: PASS. The row has the shape gate T expects; the edits are
  awk; tags.sh limits a release tag to vX.Y.Z in all five scripts; the loader
  was checked against its fixtures.
- gemini-3.1-pro-high (agy): PASS. It ran `scripts/release-goal.sh window
  v1.33.0` and found the booked PRs and tickets match. It checked the crux
  audit's claims against v1.33.0 and v1.33.0-rc.1.
- claude-haiku-4-5: PASS, no findings.
