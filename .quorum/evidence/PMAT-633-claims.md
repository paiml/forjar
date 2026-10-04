# Claims — the I8 gate judges with bashrs 7.4 (PMAT-633)

Counts and heads live in `PMAT-633-lanes.md`.

- **C1** the falsifier `break_inside_a_for_inside_an_if_passes` discriminates: RED on bashrs 6.68.0 (only Cargo.toml and Cargo.lock reverted), green on 7.4
- **C2** `break_outside_any_loop_is_still_rejected` stops a bump that merely drops SC2105
- **C3** the Cargo.lock churn is bashrs 7's transitive dependencies only (`cargo update -p bashrs`)
- **C4** across all 9 paiml/infra machine YAMLs the only new Error is a DET002 on lambda-labs `date > legacy-purged.stamp`, a true positive, fixed on the infra side (paiml/infra#1101)

## The measured symptom

`bashrs = "6.68.0"` was an exact pin. On gx10 (paiml/infra#1088) the I8 gate
refused a task with `SC2105 'break' is only valid in loops` on a `break` inside
a one-line `for` loop nested in an `if`. bashrs 7.4.2 judges the same bytes clean.
