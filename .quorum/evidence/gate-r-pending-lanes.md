# PMAT-234 — the lanes

One round, three sandboxed agy quorum lanes, width 3, `writes=false`, on the
diff at `a517c9af` against `origin/main`, with `--not-before` pinned to the
dispatch instant so a stale lane set cannot be reduced into this one.

| lane | conversation | exit | duration | verdict |
|---|---|---|---|---|
| 1 | conv-483fb521 | 0 | 371s | PASS |
| 2 | conv-5d8c1125 | 0 | 332s | FAIL |
| 3 | conv-51c90ddf | 0 | 333s | FAIL |

The brief fixed the order of the attack: enumerate every `note_` call site and
assign its bucket; look for a state where the old script exited non-zero and
the new one exits 0; look for a state where `also pending` drops a note or sits
beside a false `CRUX present`; rebuild the harness and watch the published case
go red on `origin/main`; diff the moved fixture for behaviour rather than
formatting; and check the `cargo search` output shape against the real tool.

All six produced a confirmation. The finding that mattered came from outside
the list: **two lanes read the one remaining note and found it asserts the crux
document is missing when it is present**. That is the value of an adversarial
round over a self-review — the brief could not ask about a defect its author
had not noticed, and two lanes noticed it anyway.

The split verdict (1 PASS, 2 FAIL) is recorded as it fell rather than resolved
by majority. The refutation reproduces, which is what decides it; two lanes
agreeing is not a measurement and one lane's PASS is not a clearance.

## The no-write rule

The brief OPENS with it. The repository was verified clean afterwards:
`git status --porcelain` showed only the orchestrator's own untracked log, and
HEAD was still the commit the round was dispatched on.
