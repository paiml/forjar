# Runtime Scoring Dimensions

Falsification coverage for FJ-2800 through FJ-2803.

## Two-Tier Grading

Forjar Score v2 separates quality into:

- **Static Grade** (design quality): SAF(25%) + OBS(20%) + DOC(15%) + RES(20%) + CMP(20%)
- **Runtime Grade** (operational quality): COR(35%) + IDM(35%) + PRF(30%)

Grade thresholds: A (composite>=90, min>=80), B (>=75, min>=60), C (>=60, min>=40), D (>=40), F (<40).

Display: `A/A` (both tiers), `B/pending` (no runtime), `C/blocked` (blocked status).

## COR — Correctness (35%)

| Check | Points |
|-------|--------|
| `validate_pass` | +15 |
| `plan_pass` | +15 |
| `first_apply_pass` | +40 |
| `all_resources_converged` | +15 |
| `state_lock_written` | +10 |
| Per warning (max 5) | -2 each |

## IDM — Idempotency (35%)

| Check | Points |
|-------|--------|
| `second_apply_pass` | +25 |
| `zero_changes_on_reapply` | +25 |
| `hash_stable` | +20 |
| Class "strong" | +20 |
| Class "weak" | +10 |
| Per changed resource (max 5) | -10 each |

## PRF — Performance (30%)

Three sub-components, capped at 100 total:

### Budget Points (first apply vs budget)

| Ratio | Points |
|-------|--------|
| 0-50% | 50 |
| 51-75% | 40 |
| 76-100% | 30 |
| 101-150% | 15 |
| >150% | 0 |

### Idempotent Points (second apply speed)

| Duration | Points |
|----------|--------|
| 0-2000ms | 30 |
| 2001-5000ms | 25 |
| 5001-10000ms | 15 |
| >10000ms | 0 |

### Efficiency Points (second/first ratio)

| Ratio | Points |
|-------|--------|
| 0-5% | 20 |
| 6-10% | 15 |
| 11-25% | 10 |
| >25% | 0 |

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_scoring_runtime.rs` | 31 | 499 |
