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

Round 3, head a41fd4e2 (forward merge of main c6c8591c): AGREED, 3/3 PASS
(claude-sonnet-5, gemini-3.1-pro-high, claude-haiku-4-5).

Round 4, head 51dd5c64: AGREED, 3/3 PASS, same three models. That head also
carried the T3 bare-#N rule and the `amend` subcommand, merged in from another
branch.

5748bbc9 takes both off again with a forward revert. The ticket's acceptance
criteria make T3 a separate ticket and do not name the amend. After it, the
scripts and tests are back to how they were before that merge, apart from what
main brought in.

Round 5, head 5748bbc9: NOT AGREED.

- claude-sonnet-5: FAIL. The receipt's last round was bound to 51dd5c64, not
  to this head, and this file narrated only rounds 1 and 2. The second point
  is answered by this section. The first is the binding order below.
- gemini-3.1-pro-high (agy): NO-VERDICT. The run ended with no output.
- claude-haiku-4-5: PASS.

How a round binds to a head: the round runs on a commit, and its verdict is
then written into `.quorum/fix-607-book-releases.json`. That file is excluded
from the bound diff hash (`diff_sha256`), so writing the verdict does not change
what was reviewed. A receipt can never already hold the round that is
reviewing it. The round on the head that carries this paragraph is the one the
receipt records last.
