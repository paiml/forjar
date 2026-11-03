# Claims — forjar#415: plan --refresh consults the host before diffing

Briefed to every lane; final round at head 4b3f22c2 (378578f9 with main
61d5a703 merged in), base 61d5a703.

- C1: `plan --refresh` runs the drift detectors per machine in scope and marks each lock entry they measured as changed `Drifted` in an in-memory copy; the unchanged planner plans `Drifted` as an update.
- C2: a refreshed plan writes nothing: the lock bytes and the target are unchanged.
- C3: a query the target did not answer is unmeasured, not drift: it is planned from the lock alone, counted and disclosed, naming `forjar drift`.
- C4: `plan --refresh --out` is refused by name; a sealed plan could never apply.
- C5: the machine surface says which quantifier ran: `lock_relative: false` and a `refresh` object iff `--refresh` ran, and the lock-relative sentence is not printed.
