# PMAT-557 — the lanes, and what each returned

One round of three sandboxed agy quorum lanes (`agy-lane.sh --mode plan`),
review-only. Author model: Claude Opus 5; lanes Gemini, measured from their
logs. gemini-3.8-flash-high had returned 503 all day and was left out, so two
lanes share a model id (recorded by lane-reduce in `partial_reasons`).

| lane | model (measured) | verdict | findings |
|---|---|---|---|
| 1 (lane-1.json) | gemini-3.1-pro-high | FAIL | 8 |
| 2 (lane-2.json) | gemini-3.7-flash-high | PASS | 6 |
| 3 (lane-3.json) | gemini-3.1-pro-high | PASS | 6 |

`lane-reduce.sh --width 3 --not-before 1789495652` → `agreed=false` (2 PASS,
1 FAIL). Conversation ids shortened: conv-b1c1f2ee, conv-ef7beb3c,
conv-670f4dad.

All three lanes: `gh` returned 401 in the sandbox, so `tagged.sh` and
`release-goal.sh window` were UNMEASURED, and each lane checked the row's
PRs and tickets against `git log v1.29.0..v1.30.0` and the cut against the
tag's creatordate instead. All three found the cut equal to the creatordate in
UTC and the ten PRs and eleven tickets complete, with nothing merged after the
tag counted. Lanes 1 and 2 agreed the receipt marker was appended honestly and
that the three completed rows map to merged PRs; lanes 2 and 3 agreed the
minted rows carry their issue number as id tail and that no existing row was
reflowed.

Lane 1's FAIL was one sentence: the message of commit `b3b04c1f` "claims the
cookbook field was replaced, but that commit retained 0be3e1ec". Lane 1 also
wrote a `diff.txt` into its own clone despite a no-writes brief; `agy-lane.sh`
kept the clone and said so; the shared checkout was untouched and the clone
was removed after the finding was recorded.

The orchestrator re-ran what the sandbox could not: `tagged.sh` on the branch
(GATE T PASS, 8 releases, 53 tickets), `release-goal.sh window v1.30.0`, and
the CB-21xx counts.
