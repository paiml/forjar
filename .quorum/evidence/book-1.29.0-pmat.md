# PMAT-531 — pmat measurements

## The row, re-derived

`scripts/release-goal.sh window v1.29.0` prints the row byte-identically,
including the field no release has ever carried before:

```
cookbook: 7c100454e8f9fb2b5b13f076b773f808071d5e08
```

## The ratchet, and the raise

| check | measured | ceiling | |
|---|---|---|---|
| CB-2110 | 49 | 49 | |
| CB-2111 | 49 | 49 | |
| CB-2112 | 35 | 35 | **raised 34 → 35, with a written reason** |
| CB-2114 | 34 | 34 | restored by real work: PMAT-240 has issue 530 and a release binding |
| CB-2115 | 42 | 43 | below |

The raise is legitimate — correcting PMAT-240's false `completed` turned a
closed row into an open one, and open rows are what these checks count — and it
is exactly the shape an illegitimate raise has. Until this branch, the only
thing separating them was a sentence nobody was required to write.

Removing the `justification` block turns gate B red:

```
GATE B FAIL the CB-2110..CB-2115 ceilings (Arm 7): RAISED WITHOUT A REASON:
CB-2112 raised 34 -> 35 with no justification.CB-2112
```

## Vacuity

`pmat analyze vacuous-tests`: **400 of 19650 `#[test]` fns cannot fail (2.0%)
across 2164 parsed files; 3 more skip silently.** The new case,
`a_ceiling_may_not_rise_without_a_written_reason`, is not among them — it drives
six outcomes over a temp git repository and asserts on exit codes and named
text, not on the script's spelling.

## bashrs

`bashrs lint scripts/dogfood/comply.sh`: **0 errors**.

## The lifecycle refused the correction

`pmat work edit PMAT-240 -s planned` and `-s inprogress` both return
`Invalid transition: Completed → …`. The row was corrected textually, which is
recorded in its own `notes` field and in the receipt rather than done quietly.
