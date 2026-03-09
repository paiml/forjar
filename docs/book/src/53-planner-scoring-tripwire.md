# Planner Analysis, Scoring & Tripwire Anomaly Detection

Forjar's planner classifies every resource operation formally. The scoring
engine grades recipe quality across 8 dimensions. Tripwire anomaly detection
uses ML-inspired algorithms for drift analysis.

## Proof Obligation Taxonomy (FJ-1385)

Every resource+action pair is classified into one of four categories:

```rust
use forjar::core::planner::proof_obligation::{classify, label, is_safe};

let po = classify(&ResourceType::File, &PlanAction::Create);
assert_eq!(label(&po), "idempotent");
assert!(is_safe(&po));  // only Destructive returns false
```

| Category | Meaning | Safe? |
|----------|---------|-------|
| Idempotent | `f(f(x)) = f(x)` — re-runnable | Yes |
| Monotonic | Only adds state | Yes |
| Convergent | Same fixed point from any start | Yes |
| Destructive | Removes unreconstructable state | **No** |

## Reversibility Classification (FJ-1382)

```rust
use forjar::core::planner::reversibility::{classify, count_irreversible};

let rev = classify(&resource, &PlanAction::Destroy);
// File+content → Reversible, User → Irreversible

let n = count_irreversible(&config, &plan);
```

## Change Explanation (FJ-1379)

```rust
use forjar::core::planner::why::{explain_why, format_why};

let reason = explain_why("nginx-pkg", &resource, "web-01", &locks);
println!("{}", format_why(&reason));
// nginx-pkg on web-01 -> Update
//   - hash changed: blake3:abc... -> blake3:def...
//   - version changed: 1.24 -> 1.25
```

## Scoring Engine v2 (FJ-2800)

Two-tier grading: static (design quality) + runtime (operational quality).

```rust
use forjar::core::scoring::{compute, ScoringInput, RuntimeData};

let input = ScoringInput {
    status: "qualified".into(),
    idempotency: "strong".into(),
    budget_ms: 5000,
    runtime: Some(RuntimeData { .. }),
    raw_yaml: None,
};
let result = compute(&config, &input);
// result.grade = "A/A" (static/runtime)
```

**Static dimensions** (5): SAF(25%), OBS(20%), DOC(15%), RES(20%), CMP(20%)
**Runtime dimensions** (3): COR(35%), IDM(35%), PRF(30%)

## Recipe Input Validation

```rust
use forjar::core::recipe::{validate_inputs, RecipeInput, RecipeMetadata};

let resolved = validate_inputs(&recipe, &provided)?;
// Validates: string, int (min/max), bool, path (absolute), enum (choices)
```

## Tripwire Hasher (FJ-014)

```rust
use forjar::tripwire::hasher::{hash_string, hash_file, composite_hash};

let h = hash_string("content");       // "blake3:abcd..."
let h = hash_file(Path::new("f"))?;   // same format
let h = composite_hash(&["a", "b"]);  // combined hash
```

## Anomaly Detection (FJ-051)

```rust
use forjar::tripwire::anomaly::{isolation_score, ewma_zscore, AdwinDetector};

// Isolation scoring (0.0-1.0, higher = more anomalous)
let score = isolation_score(&population, target);

// EWMA z-score for recent-bias detection
let z = ewma_zscore(&series, target, 0.3);

// ADWIN streaming drift detector
let mut detector = AdwinDetector::new();
detector.add_element(1.0);
assert_eq!(*detector.status(), DriftStatus::Stable);
```

## Falsification Example

```bash
cargo run --example scoring_recipe_planner
cargo test --test falsification_planner_proof_reversibility
cargo test --test falsification_scoring_recipe_validation
cargo test --test falsification_tripwire_hasher_anomaly
```
