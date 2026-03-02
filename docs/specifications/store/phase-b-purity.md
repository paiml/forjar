# Phase B: Purity Model (FJ-1305–FJ-1309)

**Status**: ✅ Complete
**Implementation**: `src/core/store/purity.rs`, `src/core/store/closure.rs`, `src/core/store/validate.rs`

---

## 1. Purity Levels (novel — no prior art found in literature)

| Level | Name | Definition | Example |
|-------|------|------------|---------|
| 0 | **Pure** | All inputs hashed, sandboxed, deterministic | `store: true` + `sandbox: full` + pinned version |
| 1 | **Pinned** | Version-locked but not sandboxed | `version: "1.24.0"` + `store: true` |
| 2 | **Constrained** | Provider-scoped but floating version | `provider: apt` + `packages: [nginx]` (no version) |
| 3 | **Impure** | Unconstrained network/side-effect access | `curl \| bash` install scripts |

Mokhov et al.'s build system taxonomy [5] classifies by scheduler/metadata axes but does not propose purity hierarchies. Malka et al. [6] formalize "reproducibility in space" vs "in time" as 2 axes. Our 4-level model is orthogonal — it classifies a single resource's input discipline.

## 2. Purity Monotonicity

A recipe's purity level is the **maximum** (least pure) of all its transitive dependencies. A pure recipe that depends on a constrained input is at most constrained. This is enforced statically.

## 3. Static Analysis (FJ-1305)

Purity classification reuses existing detection: `version:` field presence, SAF `curl|bash`/`wget|sh` pattern matching, `sandbox:` block presence, transitive `depends_on`/`recipe` closure.

## 4. Validation Command (FJ-1306)

`forjar validate --check-recipe-purity` reports each resource's purity level (Pure/Pinned/Constrained/Impure) with reason.

## 5. Input Closure Tracking (FJ-1307)

Each resource's input closure is the set of all transitive inputs. The closure hash is `composite_hash()` over all input hashes, sorted lexicographically. Identical closures produce identical store paths.

---

## References

- [5] A. Mokhov, N. Mitchell, S. Peyton Jones, "Build Systems à la Carte," ICFP 2018
- [6] J. Malka, S. Zacchiroli, T. Zimmermann, "Reproducibility of Build Environments through Space and Time," arXiv:2402.00424, 2024
