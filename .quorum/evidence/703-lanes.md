# Lanes — forjar#703

Lanes run through quorum-review.sh (--shape default) with the ticket row and
the full diff against main 9a37a2ff. None used the author's model
(claude-opus-5-5). Each lane was read-only. The two Claude seats ran in one
round; the agy seat (gemini-3.8-flash-high) ran as its own lane through
agy-lane.sh --mode plan under a per-round exception (see `703-agy.md`). Three
distinct model ids over two families, so the review round is not degraded.

Planted control at 4a57e71e (c98fb5c6 plus one commit that checks main's
Cargo.lock back out, so quinn-proto is 0.11.17 again while the guard stays):
NOT AGREED, 3/3 FAIL, never pushed.

- claude-sonnet-5-5: FAIL. Cargo.lock at HEAD pins the vulnerable 0.11.17,
  the diff has no lock hunk, and the new guard fails on that lock.
- claude-haiku-4-5: FAIL. No Cargo.lock change, so criterion 1 is unmet.
- gemini-3.8-flash-high (agy): FAIL. Cargo.lock still pins 0.11.17 (named
  the revert commit), and the new guard asserts the opposite of the lock.

Round at head c98fb5c6, the head being pushed: 3/3 PASS.

- claude-sonnet-5-5: PASS. Exactly two lock lines move, no exemption is
  added, the roadmap row is honest about unsafe-libyaml, and the guard fails
  on a vulnerable, absent, pre-release or unparseable version. It ran no
  build or test; the build-host measurements are in `703-judges.md`.
- claude-haiku-4-5: PASS. Version and checksum bump, a regression guard with
  edge cases, no change to deny.toml or .cargo/audit.toml.
- gemini-3.8-flash-high (agy): PASS, 0 findings.

The Claude round's third configured seat was left empty (one lane per
configured seat), so its own artifact records NOT AGREED with lane 3
NO-VERDICT; the agy lane above, run on its own, is the third lane counted
here.

Earlier local rounds r1-r3 ran on an unpushed branch with a different
receipt; they are superseded by this round and judge nothing here. One of
their findings applied to this diff's test file and is R1 in `703-judges.md`.

FR-4 slot for this PR: GRANT 3/3, planted control DENY 3/3,
`degraded: same-family` (no agy seat; no opus lane, the author's id).
