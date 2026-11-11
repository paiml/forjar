# PMAT-590 (`forjar#946`) — Extracted Claim Manifest (`C1..C3`, `R1..R2`)

Base commit: `5b8f8123be40731340e2a74a6e4ae365648cab45`
Head commit: `f84ece9b2456abbfda44ac5f9a1404f602e2ce0c`
Branch: `f20-r2/590-refresh-template`

1. **C1 (`[code]`)**: On `main`, `lockless_check::host_check_passes` (`src/core/executor/lockless_check.rs:46`) and `refresh::refresh_check_fails` (`src/core/executor/refresh.rs:76`) already call `resolver::resolve_resource_templates` before `codegen::check_script(&resolved)`.
2. **C2 (`[test]`)**: `refresh_seeds_a_templated_guard_like_a_plain_one` (`src/core/executor/tests_refresh_closure.rs:90`) pins positive parity between plain and `{{params.root}}`-templated `completion_check` guards, failing if `check_script` is called on the unresolved resource (`FJ946_REV1_KILLED`).
3. **C3 (`[test]`)**: `refresh_does_not_seed_a_templated_guard_whose_check_fails` provides the negative control so a failing templated `completion_check` is never vacuously seeded.
4. **R1 (`[issue]`)**: Issue `#590`'s hypothesis that local `apply --refresh` skips `{{params.*}}` resolution on `main` is refuted.
5. **R2 (`[doc]`)**: The new doc comment in `tests_refresh_closure.rs` claims `command` AND `completion_check` carry `{{params.*}}` (whereas `command` is `"false"`), and remote SSH / `{{secrets.*}}` paths remain unmeasured.
