# Quorum evidence — PMAT-206 — the claims as put to the lanes

The 1.26.0 merges took CB-200 (functions below TDG grade A) from a recorded ceiling of 651 to 654 on main, and gate B of forjar-dogfood refused the release cut. This branch claims three behaviour-preserving reductions and one measurement fix, and the claims below were put first to five lanes (three blind claim lanes, one /teamwork-preview lane, one CRUX lane; 4 FAIL, 1 PASS) and then, after every reproducible finding was acted on, to three refuter-judges (3 FAIL on one claim's wording, everything else confirmed).

## Claims

- C1 `observe::classify` is a static table of 32 (field, observability) pairs with a fallthrough to `classify_e01::classify`, and every field maps to exactly the value the replaced match gave it — pinned at src/core/observe/mod.rs:70 and src/core/observe/mod.rs:55.
- C2 `redact_quoted_paths` and `clause_verdict` are decomposed (src/core/purifier_sec017.rs:191, src/core/purifier_sec017.rs:164, src/core/purifier_sec017.rs:293, src/core/purifier_sec017.rs:309, src/core/purifier_sec017.rs:391, src/core/purifier_sec017.rs:367) and every shape pinned by tests/falsification_chmod_path_is_not_a_mode.rs and tests/falsification_chmod_gate_survives_review.rs keeps its verdict.
- C3 `examples/cron_secret_encryption_falsification.rs` asserts every criterion it asserted before, with the same labels, through one `criterion` helper.
- C4 The ratchet's cache removal cannot reach a path outside comply's cache root: the glob is anchored to the root, a case pins it, `-d` requires a directory, `${dir:?}` refuses an empty variable.
- C5 The ratchet never swallows a measurement: an empty comply result is UNMEASURED and exit 1; a stale cache is printed as a NOTE and removed, not silently refreshed.
- C6 The ceiling stays 651; with a fresh comply index this branch measures exactly 651 (`scripts/cb200-ratchet.sh` exit 0, observed twice, once with the cache fresh and once after touching a source so the cache was older than the tree).
- C7 Nothing in the diff is outside the ticket's two acceptance criteria.

## Measurement

```text
main 2acefaec, comply cache as found ........ 654
+ classify as a table ........................ 653
+ purifier decompositions .................... 652
+ example split, criterion helper ............ 652   (unchanged: comply was grading a stale cache)
comply cache removed, same tree .............. 651   (= ceiling; the three reductions had landed all along)
ratchet, cache fresh ......................... 651  exit 0
ratchet, source newer than cache ............. NOTE printed, cache removed, 651  exit 0
```

