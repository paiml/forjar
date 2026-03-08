# Recipe Quality Score (ForjarScore v2)

> Spec ID: FJ-2800 | Status: IMPLEMENTED | Replaces: ForjarScore v1

Recipe quality scoring for the forjar-cookbook. Measures recipe design quality (static analysis) and operational quality (runtime verification) independently, replacing the v1 system where runtime-dependent dimensions zeroed out the score for unqualified recipes.

## Problem Statement (v1 Defects)

ForjarScore v1 has five structural defects identified via falsification:

1. **Cliff at pending/blocked**: `composite=0` for any non-qualified recipe. Two recipes with SAF=100 and SAF=40 both score F. Zero signal for design quality before runtime qualification.
2. **55% runtime wall**: COR(20%) + IDM(20%) + PRF(15%) = 55% requires containers. Maximum static-only score = 45 points → always grade D. Well-designed recipes cannot score above D without runtime data.
3. **Zero variance among qualified recipes**: Every qualified strong-idempotency recipe scores 94. Every weak-idempotency recipe scores 93. The score cannot distinguish a 3-resource trivial recipe from a 13-resource multi-machine topology.
4. **RES penalizes correct architecture**: CIS hardening recipes (independently-applicable controls via `--tag`) score RES=20 because <30% resources have `depends_on`. Independent resources are the correct design for tagged control sets.
5. **DOC rewards volume over quality**: 40 points for ≥15% comment ratio. Copy-pasted `# Deploy managed configuration file` scores identically to thoughtful explanations. Self-documenting recipes with clear names and descriptions are penalized.

## v2 Design: Two-Tier Grading

Score recipes on two independent axes:

```
┌─────────────────────────────────────────────────────┐
│  ForjarScore v2                                      │
│                                                      │
│  Static Grade (design quality)     ── always available│
│    SAF  Safety           25%                         │
│    OBS  Observability    20%                         │
│    DOC  Documentation    15%                         │
│    RES  Resilience       20%                         │
│    CMP  Composability    20%                         │
│                                                      │
│  Runtime Grade (operational quality) ── after apply  │
│    COR  Correctness      35%                         │
│    IDM  Idempotency      35%                         │
│    PRF  Performance      30%                         │
│                                                      │
│  Overall = min(static_grade, runtime_grade)           │
│  If runtime not available: Overall = static_grade     │
│  with suffix: A → A-pending, B → B-pending           │
└─────────────────────────────────────────────────────┘
```

**Key change**: Static grade is always computed and always meaningful. A recipe with SAF=100, OBS=90, DOC=85, RES=80, CMP=95 earns **static grade A** before any container runs. Runtime grade elevates or constrains the overall grade after qualification.

## Static Dimensions (v2)

### SAF — Safety (25%)

Starts at 100, deductions applied. Unchanged from v1 except:

| Check | Deduction | Notes |
|-------|-----------|-------|
| `mode: 0777` | -30 (critical) | Hard cap at 40 on any critical |
| File without explicit `mode` | -5 | |
| File without explicit `owner` | -3 | |
| Package without version pin | -3 per package | |
| `curl\|bash` pipe pattern | -30 (critical) | Same-line pipe to shell |
| Secrets in plaintext params | -10 per secret | Param name matches `password`, `token`, `secret`, `key` with non-template value |

### OBS — Observability (20%)

| Feature | Points |
|---------|--------|
| `tripwire: true` | +15 |
| `lock_file: true` | +15 |
| `outputs:` section | +10 |
| File mode coverage (% with explicit mode) | 0-15 proportional |
| File owner coverage (% with explicit owner) | 0-15 proportional |
| Notify hooks (on_success/on_failure/on_drift) | `(count × 20) / 3` |
| Output descriptions present | +10 (NEW) |

### DOC — Documentation (15%)

Replace comment-ratio volume metric with quality signals:

| Feature | Points | Notes |
|---------|--------|-------|
| Header metadata: `Recipe:` | +8 | First 5 lines |
| Header metadata: `Tier:` | +8 | |
| Header metadata: `Idempotency:` | +8 | |
| Header metadata: `Budget:` | +8 (NEW) | |
| `description:` field present | +15 | |
| Name is kebab-case | +3 | |
| Unique inline comments (≥3 distinct) | +15 (NEW) | Deduplicated — copy-paste doesn't count |
| Output `description:` fields (≥50% have descriptions) | +10 (NEW) | |
| Param documentation (≥3 params with non-empty values) | +10 (NEW) | |

**Removed**: Raw comment-ratio metric (15% → 40 points). Replaced with unique-comment check that rewards distinct explanations.

### RES — Resilience (20%)

Context-aware scoring that doesn't penalize correct architecture:

| Feature | Points | Notes |
|---------|--------|-------|
| `failure: continue_independent` | +15 | |
| `ssh_retries > 1` | +10 | |
| Dependency DAG ratio (≥50% with `depends_on`) | +20 | |
| **OR** Tagged independent resources (≥50% with `tags` and `resource_group`) | +20 (NEW) | Either deep DAG or tagged independence scores — not both required |
| `pre_apply:` present | +8 | |
| `post_apply:` present | +8 | |
| `deny_paths:` present | +10 (NEW) | Security-conscious policy |
| Multi-machine with `parallel_machines: true` | +5 (NEW) | Fleet-aware |

**Key change**: A recipe with tagged independent controls (like CIS hardening) earns the same resilience points as one with deep dependency chains. The scoring recognizes that tagged independence IS a resilience pattern — it enables selective application and blast radius control.

### CMP — Composability (20%)

| Feature | Points |
|---------|--------|
| `params:` section | +15 |
| Template usage (`{{...}}` in resources) | +10 |
| `includes:` present | +10 |
| Resources have `tags` | +15 |
| Resources have `resource_group` | +15 |
| Multiple machines | +10 |
| Multiple includes (recipe nesting) | +10 |
| Secrets via `{{ secrets.* }}` template | +5 (NEW) |

## Runtime Dimensions (v2)

### COR — Correctness (35%)

| Event | Points |
|-------|--------|
| `forjar validate` passes | +15 |
| `forjar plan` passes | +15 |
| First `forjar apply` passes | +40 |
| All resources converged | +15 |
| State lock written | +10 |
| Warnings | -2 per (max -10) |

### IDM — Idempotency (35%)

| Event | Points |
|-------|--------|
| Second apply passes | +25 |
| Zero changes on re-apply | +25 |
| Hash stable across applies | +20 |
| Idempotency class bonus | strong: +20, weak: +10, eventual: 0 |
| Changed resources | -10 per |

### PRF — Performance (30%)

| Metric | Points |
|--------|--------|
| First apply ≤ 50% of budget | +40 |
| First apply ≤ 100% of budget | +25 |
| Idempotent apply ≤ 2s | +30 |
| Efficiency ratio ≤ 5% | +20 |
| Efficiency ratio ≤ 10% | +15 |

## Grade Thresholds (v2)

| Grade | Static | Runtime | Overall |
|-------|--------|---------|---------|
| A | composite ≥ 90, min ≥ 80 | composite ≥ 90, min ≥ 80 | min(static, runtime) |
| B | composite ≥ 75, min ≥ 60 | composite ≥ 75, min ≥ 60 | |
| C | composite ≥ 60, min ≥ 40 | composite ≥ 60, min ≥ 40 | |
| D | composite ≥ 40 | composite ≥ 40 | |
| F | < 40 | < 40 | |

**Display format**: `A/A` (static/runtime), `A/pending`, `B/F` (good design, bad runtime).

## Migration from v1

1. All existing qualified recipes retain their A-grade (runtime data unchanged)
2. Pending/blocked recipes gain a meaningful static grade instead of F
3. CSV format adds `static_grade` and `runtime_grade` columns alongside `grade` (overall)
4. `score_version` column changes from `1.0` to `2.0`
5. Backward compatible: v1 `grade` column = overall grade

## Implementation

- **Location**: `forjar-cookbook/crates/cookbook-qualify/src/score/`
- **Spec ID**: FJ-2800 (scoring model), FJ-2801 (static dimensions), FJ-2802 (runtime dimensions), FJ-2803 (falsification)
- **Tests**: Each dimension function has unit tests with boundary conditions
- **CLI**: `cookbook-runner score --file recipe.yaml` reports both grades

## Popperian Falsification

ForjarScore v2 is a measurement instrument. Following Popper (1959), any non-falsifiable metric is pseudoscience. Each scoring dimension must state conditions under which it would be rejected as invalid. Literature review (Konala et al. 2025, Dalla Palma et al. 2020/2022, Groce et al. ASE 2018, Hatton 1997) confirms no existing IaC scoring system combines static metrics into composite grades — ForjarScore is novel and therefore requires explicit falsifiability.

### Rejection Criteria by Dimension

Each dimension specifies a concrete test that, if it fails, proves the dimension is measuring the wrong thing. These are **mandatory** quality gates for the scoring code itself.

**SAF (Safety)**:
- **Falsifier**: Construct a recipe with `mode: '0777'` on a file resource containing `{{ secrets.db_password }}`. If SAF > 40, the dimension fails to detect a critical safety violation.
- **Boundary**: A recipe with zero file resources must score SAF=100 (no deductions possible). If it doesn't, the deduction logic has false positives.
- **Construct validity**: SAF must correlate with actual misconfiguration risk. Per Rahman et al. (ICSE 2019), security smells in IaC predict real vulnerabilities — SAF's checks map to their 7-smell taxonomy. Rejection: if ≥20% of SAF deductions fire on recipes with no actual security concern.

**OBS (Observability)**:
- **Falsifier**: A recipe with `tripwire: true`, `lock_file: true`, `outputs:` with descriptions, all file modes explicit, and all three notify hooks must score OBS ≥ 90. If it doesn't, the point allocation is broken.
- **Boundary**: A recipe with no policy section and no outputs must score OBS ≤ 15. If it scores higher, phantom points are being awarded.

**DOC (Documentation)**:
- **Falsifier**: Create two recipes — one with 20 identical copy-pasted comments (`# Deploy file`), one with 5 distinct explanatory comments. If the copy-paste recipe scores DOC ≥ the distinct-comment recipe, the unique-comment check is broken. This directly addresses v1 defect #5.
- **Hatton warning**: Hatton (1997) falsified "smaller modules = fewer bugs" with a U-shaped curve. Similarly, DOC must not assume more documentation = better quality. Rejection: if DOC score correlates with comment count (r > 0.7) rather than comment distinctness across the cookbook corpus.

**RES (Resilience)**:
- **Falsifier**: Recipe 92 (CIS hardening) has tagged independent controls with `failure: continue_independent`. It must score RES ≥ 70. A recipe with 100% `depends_on` chains and no tags must also score RES ≥ 70. Both are valid resilience patterns. If either scores < 60, the dimension has a design bias.
- **Construct validity**: Tagged independence enables selective application (`--tag audit`) and blast radius control. Per v1 defect #4, penalizing this pattern is a measurement error. Rejection: if any recipe with ≥80% tagged+grouped resources AND `failure: continue_independent` scores RES < 60.

**CMP (Composability)**:
- **Falsifier**: A recipe with params, templates, includes, tags, resource_groups, multiple machines, and secrets must score CMP ≥ 85. A recipe with none of these features must score CMP = 0. If either fails, point allocation is wrong.
- **Boundary**: CMP points are additive. Sum of all feature points must equal 90 (15+10+10+15+15+10+10+5). If composite CMP ever exceeds 100, normalization is broken.

**COR (Correctness)**:
- **Falsifier**: A recipe where `forjar apply` converges all resources with zero warnings must score COR ≥ 90. A recipe where apply fails on 1 of 10 resources must score COR < 70 (the failure deduction outweighs partial convergence).
- **Invariant**: COR must be zero if `forjar validate` fails. Validation is a prerequisite for correctness. If COR > 0 when validation fails, the gating logic is broken.

**IDM (Idempotency)**:
- **Falsifier**: A strong-idempotency recipe with zero changes on re-apply and stable hashes must score IDM ≥ 90. A recipe where re-apply modifies 3 resources must score IDM < 50. The penalty for non-idempotent behavior must dominate.
- **Groce criterion** (ASE 2018): Mutation-as-falsification — inject a deliberate state mutation between applies (e.g., delete a managed file). If `forjar apply` re-converges and IDM still scores ≥ 90, the re-convergence is correctly scored. If it scores < 50, the scoring penalizes correct recovery behavior (false negative).

**PRF (Performance)**:
- **Falsifier**: A recipe with budget `first_apply < 60s` that completes in 20s (33% of budget) must score PRF ≥ 70. The same recipe completing in 120s (200% of budget) must score PRF ≤ 25.
- **Boundary**: If no budget is specified, PRF must use a default budget (not score zero). Missing budget metadata is a documentation gap, not a performance failure.

### Cross-Dimension Falsification

- **Discrimination**: The scoring engine must discriminate at two levels:
  1. **Inter-maturity**: Qualified vs pending/blocked recipes must separate by ≥ 30 composite points (dogfood: 48.5 point gap, σ_full = 20.4).
  2. **Intra-cohort (static)**: Among qualified recipes, static composite σ ≥ 4 with ≥ 8 unique score values. Dogfood: σ = 4.5, 11 unique values, range 62-80. v1 had σ = 0 (every recipe scored 94).
  Note: Combined composite σ within qualified cohort is ~2.2 because runtime dimensions (COR/IDM/PRF) cluster when all recipes pass. This is correct — it means the runtime tier is working (all qualified recipes converge). Static dimensions carry the discriminating signal.
- **Cross-project generalization warning**: Per ACM "Debunked Software Theories" (2022), metrics-based cross-project defect prediction was empirically falsified. ForjarScore explicitly does NOT claim cross-project validity. Grade thresholds are calibrated to the forjar-cookbook corpus only. Using ForjarScore to compare recipes across unrelated projects would be a category error.
- **Monotonicity**: Adding a safety feature (e.g., `deny_paths`) must never decrease the overall score. If it does, dimension interactions have a sign error. Formally: for any recipe R and safety feature f, `score(R + f) ≥ score(R)`.
- **Dalla Palma threshold** (NOT YET MEASURED): Dalla Palma et al. (2022) achieved AUC-PR=0.93 predicting IaC defects from static metrics. ForjarScore v2 static grade should correlate with runtime failure rate at r ≥ 0.5 across the qualified corpus. If correlation is below 0.3, the static dimensions are measuring orthogonal concerns and the two-tier design is validated (they should be independent). If correlation is between 0.3 and 0.5, the dimensions are weakly coupled and warrant investigation. Status: No correlation test exists yet. Requires historical failure data across multiple qualification runs to compute. Scheduled as a quarterly check once sufficient runtime data accumulates (target: 3 qualification cycles).

### Mutation Testing the Scorer

The scoring code itself must pass mutation testing (per Groce et al.'s mutation-as-falsification principle and batuta oracle conventions):

- **Mutation score target**: ≥ 85% of mutants killed in `cookbook-qualify/src/score/`
- **Critical mutants**: Any mutant that changes a grade threshold (90→89, 75→74) or flips a grade boundary must be killed. Surviving grade-boundary mutants are P0 bugs.
- **Boundary value tests**: Each dimension must have tests at threshold boundaries (0, 40, 60, 75, 80, 90, 100) per batuta's boundary falsification category.
- **TPS Jidoka**: Scoring must stop-on-error — if any dimension computation panics or returns NaN, the overall grade must be `Error`, not a degraded score.

### Falsification Schedule

| Check | Frequency | Rejection Action |
|-------|-----------|-----------------|
| Dimension boundary tests | Every CI run | Block merge |
| Cross-recipe variance | Weekly `pmat comply check` | Open P1 if static σ < 4 or inter-maturity gap < 30 |
| Monotonicity audit | On dimension formula change | Block merge |
| Mutation score ≥ 85% | Monthly batuta sweep | Open P0 if below |
| Static/runtime correlation | Quarterly | Update dimension weights if warranted |
| Copy-paste DOC falsifier | On scorer release | Block release if fails |
