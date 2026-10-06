# Claims — forjar#688: a failed resource is recorded as `converged`

Briefed to every lane: round 1 at head c19c6151, round 2 at head f17a11c5, both on main 61d5a703.

- C1: `ResourceRunStatus` had no failure variant; a failure was `Converged { failed: true }`, and `#[serde(tag = "action")]` writes the variant name, so meta.yaml said `action: converged` for a failed resource. It now has `Failed { exit_code, duration_secs }`, written `action: failed`, and `Converged` no longer carries `failed`.
- C2: a run dir written before this change still reads, and its `converged` + `failed: true` row reads as `Failed`.
- C3: `fail()` writes the run row, so all three paths that ran a resource and failed (non-zero exit, a zero exit the completion_check refuses, a transport failure) record `failed`; before, only the non-zero exit wrote a row.
- C4: a resource skipped because its dependency failed records `skipped` with a reason naming that dependency; before, it had no row.
- C5: the falsifier runs the binary on a fixture and reads the meta.yaml apply wrote, untyped; with the src at origin/main it fails, and ci.yml runs it.
