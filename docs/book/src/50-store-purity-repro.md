# Store Purity, Reproducibility & Content Addressing

Forjar's content-addressed store uses purity classification,
reproducibility scoring, input closure tracking, and reference
scanning to guarantee deterministic builds.

## Purity Classification (FJ-1305)

4-level purity model for recipe resources:

| Level | Name | Definition |
|-------|------|------------|
| 0 | Pure | All inputs hashed, sandboxed, deterministic |
| 1 | Pinned | Version-locked but not sandboxed |
| 2 | Constrained | Provider-scoped but floating version |
| 3 | Impure | Unconstrained network/side-effect access |

```rust
use forjar::core::store::purity::{classify, recipe_purity, PuritySignals};

let signals = PuritySignals {
    has_version: true,
    has_store: true,
    has_sandbox: true,
    has_curl_pipe: false,
    dep_levels: vec![],
};
let result = classify("nginx", &signals);
// result.level == PurityLevel::Pure

// Recipe purity = max of all resource levels (monotonicity invariant)
let recipe = recipe_purity(&[result.level]);
```

### What purity does and does not do

**It reports; it enforces only when you ask it to.** `forjar validate
--check-recipe-purity` exits 0 whatever it finds, including `Impure`. To gate on
it, add a threshold:

```bash
forjar validate -f forjar.yaml --check-recipe-purity --min-purity pinned
# exits 1 if the recipe is worse than Pinned
```

The JSON output carries `pass` and `min_purity` so CI does not have to re-derive
the comparison.

**Purity is not consulted by `plan` or `apply`,** and it is not recorded on the
store entry: `StoreMeta` has no purity field. A store entry therefore carries no
record of the purity of the derivation that produced it. Purity is a linting
report over the config, not a property of an artifact. If you need "nothing is
marked derived-and-proven until it has been regenerated and matched", that is
regenerate-and-compare (GH-247), not this.

**Monotonicity is real, and only recently so.** A resource is classified at
`max(own_level, max(dependency_levels))`, resolved in dependency order so it
propagates transitively. Until GH-241 the only production caller passed an empty
dependency list, so the rule never fired and a pinned resource depending on a
`curl | bash` task reported as pinned.

**`Pure` is currently unreachable.** Level 0 requires `has_sandbox`, which reads
a `sandbox:` key on the resource — and `sandbox` is not an accepted resource
field, so any config that sets it is rejected before classification runs. Every
resource in a config that validates therefore tops out at `Pinned`. The signals
are also read syntactically from the YAML (presence of `version:`, presence of
`store:`, a regex for `curl | bash`), so a command that reaches the network by
another spelling still classifies as `Pinned`.

## Reproducibility Score (FJ-1329)

0-100 composite score: purity (50%), store coverage (30%), lock pins (20%).

```rust
use forjar::core::store::repro_score::{compute_score, grade, ReproInput};

let score = compute_score(&[ReproInput {
    name: "nginx".into(),
    purity: PurityLevel::Pure,
    has_store: true,
    has_lock_pin: true,
}]);
assert_eq!(grade(score.composite), "A"); // 100/100
```

## Input Closure (FJ-1307)

Transitive closure over dependency graphs — identical closures
produce identical store paths (determinism invariant).

```rust
use forjar::core::store::closure::{input_closure, closure_hash, ResourceInputs};

let closure = input_closure("app", &graph);
let hash = closure_hash(&closure); // single BLAKE3 identity
```

## Reference Scanning (FJ-1304)

Conservative scanning discovers `blake3:<hex>` references in
store entry output files for garbage collection.

```rust
use forjar::core::store::reference::{scan_file_refs, is_valid_blake3_hash};

let refs = scan_file_refs(content, &known_hashes);
```

## Store Metadata (FJ-1301)

Each store entry has `meta.yaml` with provenance tracking:

```rust
use forjar::core::store::meta::{new_meta, write_meta, read_meta};

let meta = new_meta("store-hash", "recipe-hash", &inputs, "x86_64", "apt");
write_meta(&entry_dir, &meta)?; // atomic temp+rename
let loaded = read_meta(&entry_dir)?;
```

## Falsification Example

```bash
cargo run --example store_purity_falsification
```
