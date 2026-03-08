# Recipe Quality Scoring

ForjarScore v2 measures recipe quality on two independent axes: **static** (design quality, always available) and **runtime** (operational quality, after `forjar apply`).

## Why Two Tiers?

ForjarScore v1 had a critical flaw: recipes that hadn't been runtime-qualified scored zero, regardless of their design quality. A well-crafted recipe with proper safety controls, observability hooks, and composability features would get the same grade (F) as a broken one — simply because no container had run it yet.

v2 fixes this by separating the score into two independent grades:

```
ForjarScore: A/pending    ← static grade A, runtime not yet tested
ForjarScore: A/A          ← static A, runtime A (fully qualified)
ForjarScore: B/F          ← good design, bad runtime
```

## Static Dimensions

Static scoring analyzes the YAML recipe without executing anything.

### SAF — Safety (25%)

Starts at 100 with deductions for unsafe patterns:

| Pattern | Penalty |
|---------|---------|
| `mode: '0777'` | -30 (critical) |
| File without `mode` | -5 |
| File without `owner` | -3 |
| Package without version pin | -3 |
| `curl\|bash` pipe | -30 (critical) |
| Plaintext secrets in params | -10 per secret |

```yaml
# Good: explicit mode, owner, version pin
resources:
  config:
    type: file
    mode: '0644'
    owner: root
  nginx:
    type: package
    version: "latest"
```

### OBS — Observability (20%)

Points awarded for monitoring and operational visibility:

- `tripwire: true` (+15) — drift detection enabled
- `lock_file: true` (+15) — prevents concurrent modification
- `outputs:` section (+10) — recipe produces inspectable values
- Explicit file modes/owners (0-15 proportional)
- Notify hooks: `on_success`, `on_failure`, `on_drift` (+20/3 each)

### DOC — Documentation (15%)

Quality over quantity. v2 rewards **distinct** comments, not copy-paste volume:

- Header metadata: `Recipe:`, `Tier:`, `Idempotency:`, `Budget:` (+8 each)
- `description:` field (+15)
- 3+ unique inline comments (+15)
- Output descriptions (+10)
- 3+ documented params (+10)

### RES — Resilience (20%)

Context-aware — doesn't penalize correct architecture:

- `failure: continue_independent` (+15)
- Dependency DAG ratio ≥50% **OR** tagged independent resources ≥50% (+20)
- `deny_paths:` present (+10)

A CIS hardening recipe with tagged independent controls scores the same as a deeply chained deployment recipe. Both are valid resilience patterns.

### CMP — Composability (20%)

Rewards reusable, parameterized recipes:

- `params:` section (+15)
- Template usage `{{...}}` (+10)
- `includes:` (+10)
- Tags and resource groups (+15 each)
- Multiple machines (+10)
- `{{ secrets.* }}` templates (+5)

## Runtime Dimensions

Runtime scoring requires executing the recipe.

### COR — Correctness (35%)

Measures whether the recipe actually works:

- `forjar validate` passes (+15)
- `forjar plan` passes (+15)
- First apply succeeds (+40)
- All resources converged (+15)

### IDM — Idempotency (35%)

The core IaC guarantee — second apply changes nothing:

- Second apply passes (+25)
- Zero changes on re-apply (+25)
- Hash stable across applies (+20)
- Idempotency class: strong (+20), weak (+10)

### PRF — Performance (30%)

Measured against the recipe's declared budget:

- First apply ≤ 50% of budget (+40)
- Idempotent apply ≤ 2s (+30)

## Grade Thresholds

| Grade | Requirements |
|-------|-------------|
| A | composite ≥ 90, minimum dimension ≥ 80 |
| B | composite ≥ 75, minimum dimension ≥ 60 |
| C | composite ≥ 60, minimum dimension ≥ 40 |
| D | composite ≥ 40 |
| F | below 40 |

Overall grade = `min(static_grade, runtime_grade)`. If runtime is not available, display as `A/pending`.

## Scoring a Recipe

```bash
# Score a single recipe
cookbook-runner score --file recipes/92-cis-hardening.yaml

# Output:
# ForjarScore v2: A/pending
#   Static:  SAF=100 OBS=90 DOC=85 RES=80 CMP=95 → A (90.0)
#   Runtime: pending (not yet qualified)
```

## Falsification

Every scoring dimension has explicit rejection criteria. If a dimension fails its falsification test, it proves the dimension is measuring the wrong thing. See the [platform spec](../specifications/platform/16-recipe-quality-score.md) for the full Popperian falsification framework, including:

- Per-dimension boundary tests
- Cross-dimension discrimination requirement (σ ≥ 5)
- Monotonicity invariant (adding safety features never decreases score)
- Mutation testing target (≥85% mutants killed in scorer code)
