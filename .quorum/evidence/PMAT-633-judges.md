# Judges — the I8 gate judges with bashrs 7.4 (PMAT-633)

Round count and heads: see `PMAT-633-lanes.md`.

## CONFIRMED

1. [falsifier] C1 — The falsifier's script puts `break` inside a one-line `for` loop nested in an `if`, the forjar#633 shape, and asserts the I8 gate accepts it; it is red on 6.68.0 and green on 7.4.
   - evidence: the loop at `tests/falsification_633_i8_break_in_loop.rs:15` sits inside the `if` opened at `tests/falsification_633_i8_break_in_loop.rs:13`, and the assertion at `tests/falsification_633_i8_break_in_loop.rs:17` prints the gate's Err; with only Cargo.toml and Cargo.lock reverted it failed with `[error] SC2105: 'break' is only valid in loops`, and it passes on 7.4.
2. [guard] C2 — The guard test feeds a `break` with no enclosing loop and requires the rejection to name SC2105, so a bump that silenced or dropped the rule instead of fixing its loop detection would turn it red.
   - evidence: the bare `break` at `tests/falsification_633_i8_break_in_loop.rs:26`, the `expect_err` at `tests/falsification_633_i8_break_in_loop.rs:28`, and `err.contains("SC2105")` at `tests/falsification_633_i8_break_in_loop.rs:29`; all three lanes confirmed it, and it passes on both 6.68.0 and 7.4.
3. [lock] C3 — The Cargo.toml diff is the single bashrs line, and every Cargo.lock change is a transitive dependency of bashrs 7.4.1 (zstd, schemars, rand_chacha, toml, windows-*, objc2-*, fd-lock and others); no unrelated top-level crate moved.
   - evidence: all three lanes read the lock diff against bashrs 7.4.1's dependency list in Cargo.lock; `cargo deny check advisories` is ok and `cargo test --lib` passes 13555 of 13555.
4. [fleet] C4 — Linting all nine paiml/infra machine YAMLs with 1.32.0 and with this branch gives identical Error counts except lambda-labs 29/15 to 30/16, and the one new finding is DET002 on a stamp written with `date`, which forjar's own generators already avoid.
   - evidence: the per-machine table and the reduced-script probe (7.4 Err DET002, 6.68.0 Ok) in the round 2 brief; paiml/infra#1101 replaces it with `touch`, since only `test -f` reads the stamp, and brings lambda-labs back to 29/15 on both builds.

## REFUTED

1. [gate] R1 — The released I8 gate, on bashrs 6.68.0, refuses a `break` that is inside a loop as SC2105, so `forjar apply` stopped a correct task on gx10 (paiml/infra#1088) and the fleet worked around a linter two majors behind its own.
   - corrected: bashrs moves to 7.4 and the falsifier at `tests/falsification_633_i8_break_in_loop.rs:12` pins the shape; a real `break` outside any loop is still refused, as `tests/falsification_633_i8_break_in_loop.rs:25` asserts.
2. [evidence] R2 — C4 as first briefed claimed a fleet-wide lint sweep and an infra fix without giving the lane any way to check them, and the round 1 sonnet lane rightly refused to confirm a claim with no evidence in scope.
   - corrected: the round 2 brief carried the raw per-machine counts, the lambda-labs finding diff, the reduced-script probe and the paiml/infra#1101 diff, and all three lanes confirmed C4 against the forjar sources. The sonnet lane's roadmap finding was dismissed: PMAT-631 is also `planned` on origin/main after its merge.
