# Implementation receipt — PMAT-217 — a corpus test decided whether to assert by looking at what else was on the developer's disk, so the workspace gate was vacuously green in CI and red on a fleet workstation

verdict: DONE — `coverage_map_enrichment` now carries the `aprender-corpus` gate its sibling already had, `cargo test --workspace` goes from one failure to 311 binaries green with exit 0 from the corpus-visible path that was red, and the ratchet figure is pinned in every living place it appears. Closes forjar#452's instance.

## Identity

| field | value |
|---|---|
| ticket | PMAT-217 (kind: code) |
| issue | forjar#452 |
| branch | PMAT-217-contracts-test-reaches-outside-the-repo |
| base | 2826c449 |
| model gate | `model=opus class=opus decision=admit basis=file` |
| discover.json | `gate_cmd=cargo test --workspace`, `gate_cmd_fallback=true` |

## The defect

```rust
let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
if !root.parent().is_some_and(|p| p.join("aprender").exists()) { return; }
...
let params = QueryParams { query: "softmax".to_string(), ... };
assert!(!output.results.is_empty());
```

A neighbouring checkout is the wrong question. It does not put softmax contracts into forjar's index, which ships an IaC corpus. So the guard passed on any workstation with aprender beside forjar and the assertion then could not hold, while CI and every worktree outside `~/src` took the early return and reported a pass.

Found while gating PMAT-215, where it turned the workspace gate red and first produced a **wrong conclusion**: the branch ran in `~/src/forjar` and `main` ran in a worktree under `/tmp`, so the branch "failed" and main "passed" and neither result meant anything. Re-running `origin/main` at 54e36f13 from `~/src/forjar-mainck` reproduced it with no changes present.

## Measured

| command | before | after |
|---|---|---|
| `cargo test --workspace` from a corpus-visible path | 1 failure | **311 binaries green, exit 0** |
| `cargo test -p forjar-contracts --lib` | 1 failed, 43 ignored | 1332 passed, 0 failed, **44 ignored** |
| `cargo test -p forjar-contracts --lib --features aprender-corpus` | 38 failures | **39 failures** |

That last row is the independent check on the count: with the feature on and no corpus present, exactly the gated set fails, and it is now 39 — the same figure the ratchet records and the tree carries.

## The figure lives in five places

`scripts/dogfood/coverage.sh` keeps two EXACT numbers, not ceilings, and its own comment says why: a ceiling catches somebody parking a new test and silently accepts the other direction. Exact figures only stay honest while they match, and nothing was checking that they do outside a full gate F run, which cannot complete on this host at all (PMAT-216).

| place | before | after | found by |
|---|---|---|---|
| `coverage.sh` `APRENDER_ANNOTATIONS` | 38 | 39 | me |
| `coverage.sh` `IGNORED_EXPECTED` | 43 | 44 | me |
| the `cargo` shim in `falsification_coverage_gate_mutation_scope.rs` | 43 | 44 | **my own new invariant**, after the gate went red inside its own tests |
| `crates/forjar-contracts/Cargo.toml` | 38 | 39 | review lane 1 |
| `crates/forjar-contracts/VENDORED.md` | 38 (twice) | 39 | review lane 1 |

`docs/roadmaps/roadmap.yaml:2016` still says `(38, aprender corpus paths)` and stays that way: the row is `status: completed` and its title records what was true when it was filed. Rewriting a finished ticket to match a later number would falsify the record. The still-planned row at 2712 simply stops carrying a count.

The new invariant now covers the tree, both recorded figures, the shim and the living prose. Mutating any one of them fails it.

## What review changed

Three lanes, 3 of 3 FAIL. Every finding re-run before acting.

- **The lanes contradicted each other, all claiming "measured".** Lane 2 said nothing was left behind; lane 1 named `Cargo.toml` and `VENDORED.md`; lane 3 named two roadmap lines. At most one could be right. Re-grepping: lane 2 was wrong, lanes 1 and 3 each found real sites the other missed, and one of lane 3's two was historical text that must not move. A reduced verdict would have hidden this; the disagreement is the useful part.
- **The attribute parser was dead code.** All three said so and all three were right: once the first rule was replaced by the ratchet invariant, nothing read the parsed attributes. Removed; the helper is `test_bodies` now and its doc says why.
- **The sibling rule was evadable one call deep**, and `cross_project_tests.rs` was passing it by exactly that accident rather than by decision. The scan is file-wide now and that file is **exempt by name with its reason**, with a case that fails if the exemption stops applying. Measured: the helper-hidden shape in a non-exempt file is caught, and was not before.
- **The module docstring overclaimed.** It said the rule was "about querying aprender's KERNELS"; it has no such semantics. It now states plainly that it is a text ratchet and names the evasions it cannot catch.

## Scope, which no lane covered

The delegate reported brief item 5 UNCOVERED, so it is answered here rather than assumed. Every file in the diff traces to one cause, the gated set growing by one:

| file | why |
|---|---|
| `query_tests_coverage.rs` | the fix |
| `coverage.sh` | the two recorded figures |
| `falsification_coverage_gate_mutation_scope.rs` | the shim's canned count, or gate F fails inside its own tests |
| `falsification_contracts_corpus_tests_are_feature_gated.rs` | the new rules |
| `Cargo.toml`, `VENDORED.md` | living prose stating the figure |
| `roadmap.yaml` | this ticket's row, and one live row that carried a count |

Nothing else. No production code changed.

## Gaps

- No lane ran any cargo command; the delegate forbade it after measuring 213G of build output against 220G free, citing the PMAT-160/161 incident where a lane filled the disk. Every test count above is my own run.
- The rule is a text ratchet and says so. `PathBuf::from`, a local binding, or a differently spelled existence check would evade it. It catches the defect coming back the way it went in.
- Gate F's mutation arm remains unmeasured on this host (PMAT-216), so no `cargo mutants` figure is claimed. Every mutation above was run by hand and named.

IMPL-PMAT-217-RECEIPT-END
