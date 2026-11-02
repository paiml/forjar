# Judges — forjar#698 (phase 1): the I8 gate on bashrs 7.4.2

Lanes and their findings: see `698-lanes.md`. Line numbers are at head
6ba722fb. Every measurement below ran on a build host (cargo 1.93.0) at that
head with `cargo test --test falsification_698_i8_sc1066_quoted_equals`, and
the tree was restored and clean afterwards.

## CONFIRMED

1. [lock] C1 — The lock resolves bashrs 7.4.2, which the manifest's existing `bashrs = "7.4"` requirement already admits, so only Cargo.lock moves for the bump.
   - evidence: `Cargo.lock:279` reads `version = "7.4.2"` under `name = "bashrs"`; `cargo tree -i bashrs --depth 0` printed `bashrs v7.4.2` at 6ba722fb, before and after the 7.4.1 probe.
2. [pass] C2 — On bashrs 7.4.2 the I8 gate passes a `$a=$b` pair inside double quotes in both form variants and in one combined script, which is the false SC1066 the ticket's infra task hit.
   - evidence: the assignment form at `tests/falsification_698_i8_sc1066_quoted_equals.rs:24`, the echo form at `tests/falsification_698_i8_sc1066_quoted_equals.rs:30` and the combined script at `tests/falsification_698_i8_sc1066_quoted_equals.rs:36` all call `validate_script` through `assert_passes`; at 6ba722fb: 4 passed, 0 failed.
3. [control] C3 — The gate still rejects a real `$foo=bar`, and the rejection names SC1066, so the passes in C2 were not bought by bashrs dropping the rule.
   - evidence: `tests/falsification_698_i8_sc1066_quoted_equals.rs:47` calls `expect_err` on `$foo=bar` and `tests/falsification_698_i8_sc1066_quoted_equals.rs:48` asserts the error contains `SC1066`; it passed on 7.4.2 and on 7.4.1.
4. [discriminates] C4 — With only the lock moved back to bashrs 7.4.1 and the test file unchanged, the three pass-cases go red on SC1066 while the control stays green, so the test measures the bashrs fix.
   - evidence: `cargo update -p bashrs --precise 7.4.1` then the test: 1 passed, 3 failed; `tests/falsification_698_i8_sc1066_quoted_equals.rs:24` failed with `SC1066 ... Use WANT= instead of $WANT=` and `tests/falsification_698_i8_sc1066_quoted_equals.rs:30` with `Use p= instead of $p=`. Lock restored: 4 passed.
5. [resolution] C5 — The windows-sys and getrandom edges in the lock diff are exactly what `cargo update -p bashrs --precise 7.4.2` produces from main's lock, so they are cargo's resolution and not a hand edit.
   - evidence: main's lock (9a37a2ff) was written over the tree and the one command run; `git diff -- Cargo.lock` against the committed lock was empty. The edges at `Cargo.lock:1012` and `Cargo.lock:2889` are among the reproduced lines; all three windows-sys versions were already packages in main's lock.

## REFUTED

1. [r1-lock] R1 — Round r1, both sonnet seats: "the lock diff downgrades windows-sys 0.61.2 to 0.52.0 and getrandom 0.4.3 to 0.3.4, unrelated to the ticket, apparently a stale or hand-edited resolution".
   - corrected: measured as C5. One `cargo update -p bashrs --precise 7.4.2` from main's lock reproduces the committed lock byte for byte, including `Cargo.lock:1012`; no package enters or leaves, and the edges are Windows-only targets. Both seats PASSed in round r2 with this measurement.
2. [r1-scope] R2 — Round r1, haiku: "the ticket names an SC2075 false positive (bashrs#441) but the fix targets SC1066, so the ticket is not met".
   - corrected: the PR is phase 1 and says so: its body states it does not close PMAT-698 and the commit trailer is `Refs`. The SC1066 falsifier at `tests/falsification_698_i8_sc1066_quoted_equals.rs:24` is what this phase claims; the SC2075 fixture of the fw16 completion check stays open on the ticket.
3. [r1-receipt] R3 — Round r1, haiku: "a receipt must exist at docs/audits/impl-PMAT-698-receipt.md at HEAD per forjar gate A".
   - corrected: forjar has no such convention; its merge evidence is `.quorum/<branch>.json` with `.quorum/evidence/*.md`, which this receipt is. The measurements were handed to round r2 in the brief instead, and the falsifier they concern is `tests/falsification_698_i8_sc1066_quoted_equals.rs:44`.
