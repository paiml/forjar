# Judges — correct how the 24h nightly gate failed (PMAT-631)

Round count and heads: see `PMAT-631-lanes.md`.

## CONFIRMED

1. [comment] C1 — The gate comment in the nightly workflow now says the old gate asked a wall-clock question rather than whether HEAD was built, that a failed nightly was never retried on a quiet day, and that a dropped, queued or late scheduled run left commits outside every window.
   - evidence: the four comment lines above HEAD_SHA in the check step; all three lanes read them against the gate script and agreed, and the falsifier's own doc at `tests/falsification_nightly_gate_is_tag_vs_head.rs:7` states the same two failure modes.
2. [doc] C2 — The falsifier's module doc now names the same two failure modes and adds, in so many words, that a commit landing after the day's 04:00 run is not missed because it is under 24 hours old at the next run.
   - evidence: the rewritten defect paragraph at `tests/falsification_nightly_gate_is_tag_vs_head.rs:8` through `tests/falsification_nightly_gate_is_tag_vs_head.rs:12`, where line 11 carries the explicit not-missed sentence; agy confirmed no instance of the false claim remains in either file.
3. [date] C3 — The falsifier's commits are described as dated 2001-09-09, which is the Unix time 1000000000 the helper passes as both author and committer date, not thirty days before the run as the old doc said.
   - evidence: the doc at `tests/falsification_nightly_gate_is_tag_vs_head.rs:20` and the comment at `tests/falsification_nightly_gate_is_tag_vs_head.rs:93` now match the environment values set at `tests/falsification_nightly_gate_is_tag_vs_head.rs:94` and `tests/falsification_nightly_gate_is_tag_vs_head.rs:95`.
4. [scope] C4 — The change touches comments only: the gate script, the release step and every test body are unchanged, and the falsifier still discriminates, green on the branch and red when the 24h gate is restored.
   - evidence: run green 4 of 4, then with the 24h gate restored the tests at `tests/falsification_nightly_gate_is_tag_vs_head.rs:156` and `tests/falsification_nightly_gate_is_tag_vs_head.rs:182` failed and `tests/falsification_nightly_gate_is_tag_vs_head.rs:166` passed, and the byte-identical restore went green again.

## REFUTED

1. [mechanism] R1 — forjar#630's comment and test doc said the 24h gate never rebuilt a commit that landed after the day's run and was followed by a quiet day, but that commit is under 24 hours old at the next day's 04:00 run, so the old gate built it.
   - corrected: the comment and the doc at `tests/falsification_nightly_gate_is_tag_vs_head.rs:11` now say the late commit is not missed, and name the failed-nightly and dropped, queued or late run cases as what the window actually lost.
2. [date] R2 — The test doc said the synthetic repository's commits are all thirty days old, but the helper dates every commit at Unix time 1000000000, which is 2001-09-09, a quarter of a century before the run rather than a month.
   - corrected: the doc at `tests/falsification_nightly_gate_is_tag_vs_head.rs:20` and the comment at `tests/falsification_nightly_gate_is_tag_vs_head.rs:93` now give the date, which is what makes the history quiet whenever the test runs.
