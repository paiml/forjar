# Judges — forjar#417: the untyped-error-site count and its ratchet

Lanes and their findings: see `PMAT-417-lanes.md`.

## CONFIRMED

1. [ratchet] C1 — The live-tree test reads both ceilings from the committed baseline and fails by name when either count is above its ceiling; a count below the ceiling prints a request to lower it and stays green.
   - evidence: the REGRESSION assertion at `tests/falsification_untyped_error_sites_ratchet.rs:152` and the lower-it message at `tests/falsification_untyped_error_sites_ratchet.rs:159`; with `untyped_error_fns` set to 777 the test went red naming the key, and at 778 it passes.
2. [predicates] C2 — The signature predicate strips `pub(..)`, `pub`, `const`, `async` and `unsafe` and requires `fn`, `-> Result<` and `, String>`; the lock predicate skips `//` and `///` lines.
   - evidence: `is_untyped_error_fn` at `tests/falsification_untyped_error_sites_ratchet.rs:68` and `is_lock_literal` at `tests/falsification_untyped_error_sites_ratchet.rs:79`, with ten positive and negative lines pinned at `tests/falsification_untyped_error_sites_ratchet.rs:178`; all three lanes confirmed it.
3. [planted] C3 — The planted tree holds one counted site of each kind plus one of each exclusion (state module, tests.rs, a comment, a cfg(test) block), and asserts exact totals of files 4, fns 2 and literals 1.
   - evidence: the tree at `tests/falsification_untyped_error_sites_ratchet.rs:215` and the totals at `tests/falsification_untyped_error_sites_ratchet.rs:239`; with the `#[cfg(test)]` stop at `tests/falsification_untyped_error_sites_ratchet.rs:115` removed it measured (3, 2) and the live tree 813, both red.
4. [coupling] C4 — The fallback and the count reach zero together: `legacy_prose_class` defined must equal a non-zero `untyped_error_fns` ceiling, and the fallback's doc comment now names the ratchet that counts its callers.
   - evidence: the assertion at `tests/falsification_untyped_error_sites_ratchet.rs:169` and the fallback at `src/core/error.rs:280`; with the ceiling set to 0 while the fallback is defined the test went red.
5. [unmeasured] C5 — A walk that saw fewer than 100 `.rs` files under src/ is reported as UNMEASURED and red, so a broken walk cannot report a count of zero as a pass.
   - evidence: `MIN_FILES` at `tests/falsification_untyped_error_sites_ratchet.rs:44` and the UNMEASURED assertion at `tests/falsification_untyped_error_sites_ratchet.rs:142`; the live walk sees well over the floor, and all three lanes confirmed the guard.

## REFUTED

1. [contract] R1 — The prose-matching fallback in `src/core/error.rs` is the named, documented, deliberately temporary path that `error_taxonomy_is_total` allows, with a count of the sites still on it.
   - corrected: the path was named but the count did not exist, and `forjar undo --resume` with nothing to resume exited 2 because its message contains "partial"; the count now exists and is held at `tests/falsification_untyped_error_sites_ratchet.rs:138`.
2. [exactness] R2 — The signature predicate counts exactly the functions that return an untyped error, so the ceiling is the true number of untyped sites.
   - corrected: the sonnet and haiku lanes showed it is a floor (multi-line signatures and code after a mid-file `#[cfg(test)]` are not seen, and a nested `String>` could over-count); the module doc at `tests/falsification_untyped_error_sites_ratchet.rs:33` now says so, and the planted tree pins both limits.
