# Quorum evidence — PMAT-223 — the lanes

## Plan grill (phase 1) — one teamwork lane, sandboxed, read-only

- Mode: `/teamwork-preview` through the delegate; agy 1.1.28; 255 s; exit 0; conversation conv-7f56f8d3; fan-out measured as UNKNOWN (no agents directory, no new teamwork project, one turn), so this is read as a single model answering once, not a panel.
- Verdict: implement-with-changes on five findings. Two were refutations of the plan as written and are recorded as refuted claims: the cache-hit consequences (P1 — a hit leaves the old spec hash in the lock, so the plan says `1 to change` forever; the lane demanded it be fixed here) and the reader inventory (P2 — `cli/verify.rs::recorded_output_hash` is a third reader). The lane confirmed the machine guard on the writer, the writer's base on the reader, and the contract owner.
- The orchestrator first REFUTED P1 by hand against a stale binary, then CONFIRMED it against the right one (jidoka row 2 in docs/audits/jidoka.jsonl); the measurement on the fixed binary also found a deleted output skipped as unchanged and `--force` refused — both fixed in d4f7cc93 with the settle.
- Host tree byte-identical before and after.

## Diff review round 1 (phase 3) — three quorum lanes, --mode plan --sandbox, read-only, base pinned

- Diff: main (643363b3) ... dda3cf0e; each lane read its own `--shared` clone; no lane ran cargo (the workspace suite was running on the host), so every behavioural statement in a lane is reasoning about the diff and was re-run by the orchestrator.
- Lane 1, conv-3b2d2bec: PASS, C1–C6 and C8–C12 confirmed, C7 refuted (eight logs, not seven), 438 s.
- Lane 2, conv-516a4444: FAIL, C1–C6 and C8–C11 confirmed, C7 refuted (eight logs), C12 refuted on scope (the verify.rs and ambient.rs doc hunks), 480 s.
- Lane 3, conv-b78cd7d5: FAIL, C1–C11 confirmed (C7 confirmed as seven without counting — treated as unhunted on C7), C12 refuted on scope, 523 s.
- Not agreed (1 PASS, 2 FAIL); no lane found a functional defect. The verify.rs note was dropped (f6cb029a); the ambient.rs paragraph stayed because the diff had made its old sentence false.

## Diff review round 2 (phase 3) — three lanes, same form, base pinned

- Diff: main (643363b3) ... f6cb029a. Lane 1, conv-9c006b59: FAIL, 708 s; lane 2, conv-9870b88c: FAIL, 645 s; lane 3, conv-68351fc7: FAIL, 459 s. All three confirmed C1–C5 and C7–C11; all three refuted C6 (the claim overstated the test file's doc comment) and C12: lane 1 named the book's stage-cache paragraph and the task-framework spec's algorithm block as still describing an inputs-only cache, lanes 2 and 3 named the tests_ambient.rs comment that still said the executor passes state_dir.parent().
- The delegate hit its turn cap after the lanes had finished; the lane files and lane-reduce.json were read directly (as on PMAT-222).
- Every named sentence was corrected in 25b17924; the pipeline stage-cache sentences were left standing because `should_skip_stage` is a path this diff does not touch, and the book now says what a task resource additionally requires.

## Diff review round 3 (phase 3) — three lanes on 25b17924

- Diff: main (643363b3) ... 25b17924. Lane 1, conv-46a2f668: FAIL, 961 s; lane 2, conv-611d36ba: FAIL, 842 s; lane 3, conv-e73116e2: FAIL, 747 s. All three confirmed C1–C5, C7, C8, C10, C11; lanes 2 and 3 confirmed C6, C9 and C12(a)/(b) and refuted C12(c) on two sentences the diff made incomplete — the FJ-2701 comment at the cache-hit call site in machine_b.rs and one line of the platform-features book page — both corrected in the next commit. Lane 1 refuted C6, C9 and C12(a); the delegate's read-only cross-checks found each contradicted by the text at 25b17924 (the module doc's sentence is at lines 25–29, the cited tests exist by name, the three round-2 sites are corrected), so lane 1 is recorded as a lane error on those three and treated as unhunted there.
- The delegate ran sed/grep cross-checks only; no lane ran cargo.

## Diff review round 4 (phase 3) — three lanes on the final tree

- Diff: main (643363b3) ... de070d12. Lane 1, conv-1cea7f91: PASS, all twelve claims confirmed, 819 s. Lane 3, conv-5df448d2: PASS, all twelve confirmed by reading and grep, 761 s. Lane 2, conv-037ef0e1: FAIL, 813 s — C1–C5 and C7–C11 confirmed; C6 and C12 refuted on the tests_ambient.rs comment and the task-framework spec block, both corrected at 25b17924 and confirmed by lanes 1 and 3 here and by round 3's delegate cross-checks; recorded as a lane error, as round 3's lane 1 was.
- The delegate hit its turn cap before reducing; the orchestrator ran lane-reduce (not agreed: 2 PASS / 1 FAIL) and read the lane files directly.
- Over four rounds every code claim (C1–C5, C7, C8, C10, C11) was confirmed by all twelve lane readings; no lane ever found a functional defect. The merge proceeds on that record with the dissent named, not on a unanimous PASS.
