# Quorum evidence — #446 — pmat lane

`pmat analyze vacuous-tests --path .` on the branch: 360 of 19,237 `#[test]` fns cannot fail (1.9%) across 2,087 parsed files; 5 skip silently when a fixture is missing.

In the paths this branch adds: 0 no-failure-mode tests. The first cut had 1 silent-skip — `fj446_doctor_machine_fails_on_unwritable_destination` returned early under root; after the agy lane's refutation the case asserts the naming half under root and skips only the exit code, so the flag is gone.

`pmat work`: PMAT-149 opened before the first edit.
