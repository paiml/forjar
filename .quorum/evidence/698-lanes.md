# Lanes — forjar#698 (phase 1)

Lanes run through quorum-review.sh (--shape default) with the ticket and the
full diff against main 9a37a2ff. None used the author's model
(claude-opus-5-5). Each lane was read-only. The configured agy seat
(gemini-3.8-flash-high) was withheld by the quorum policy that reserves agy
for tier-1 diffs, so seat 2 fell through to claude-sonnet-5-5. Every round is
marked `degraded: same-family`: 2 distinct model ids over 3 lanes, all
family claude, within same_family_width 2.

Planted control at 027e45d1 (6ba722fb plus one planted commit that makes the
assignment-form test assert `is_err()`, the opposite of the ticket): NOT
AGREED, 3/3 FAIL. All three lanes named the planted line,
`tests/falsification_698_i8_sc1066_quoted_equals.rs` line 24. The control
was never pushed.

Round r1 at head 6ba722fb: NOT AGREED, 3/3 FAIL.

- claude-sonnet-5-5 (seat 1): FAIL. The lock also moves windows-sys and
  getrandom edges the ticket does not ask for.
- claude-sonnet-5-5 (seat 2): FAIL. The same edges, read as a stale or
  hand-edited resolution.
- claude-haiku-4-5: FAIL. No impl receipt file; the ticket names SC2075 but
  the test targets SC1066.

Round r2 at head 6ba722fb, the head being pushed, briefed with the
measurements in `698-judges.md` (R1 to R3): AGREED, 3/3 PASS, no findings.

- claude-sonnet-5-5 (seat 1): PASS. The diff does phase 1 and nothing more.
- claude-sonnet-5-5 (seat 2): PASS. The lock bump and the falsifier match.
- claude-haiku-4-5: PASS. The falsifier discriminates on 7.4.1 vs 7.4.2.
