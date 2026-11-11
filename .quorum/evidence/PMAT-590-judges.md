# PMAT-590 (`forjar#946`) — Judges Claims Digest

### CONFIRMED

1. [code] Pre-codegen template resolution in lockless_check and refresh (`C1`)
   - evidence: `src/core/executor/lockless_check.rs:46` and `src/core/executor/refresh.rs:76` invoke `resolver::resolve_resource_templates` before `codegen::check_script(&resolved)`, confirming local `--refresh` already resolves `{{params.*}}` in `completion_check` on `main` without production edits (`src/core/executor/tests_refresh_closure.rs:90`).

2. [test] Positive regression pin for templated completion_check seeding (`C2`)
   - evidence: `src/core/executor/tests_refresh_closure.rs:90` adds `refresh_seeds_a_templated_guard_like_a_plain_one` comparing `[ -d / ]` against `[ -d {{params.root}} ]` (`root: /`); perturbing `lockless_check.rs:50` to pass unresolved `resource` failed the test immediately on `t2fw13` (`FJ946_REV1_KILLED`).

3. [test] Negative control rejecting failing templated completion_check (`C3`)
   - evidence: `src/core/executor/tests_refresh_closure.rs:94` pairs the positive pin with `refresh_does_not_seed_a_templated_guard_whose_check_fails` (`root: /definitely/not/a/dir`), ensuring `--refresh` never vacuously seeds a templated guard whose resolved check exits non-zero (`5/5` green on `t2fw13`).

### REFUTED

1. [issue] Issue #590 premise that main runs unresolved params templates in local refresh (`R1`)
   - evidence: Ticket `#590` reported on `1.31.0` that one `{{params.*}}` template flipped a satisfied guard from `unchanged` to `FAILED` under `apply --refresh`, whereas `src/core/executor/tests_refresh_closure.rs:90` passes `5/5` on unmodified `main` production code.
   - corrected: Local `params`/`machines` template resolution is already wired into `host_check_passes` and `refresh_check_fails`; `#946` lands the missing positive and negative regression pins so no future refactor can drop that resolution step silently.

2. [doc] Test doc-comment wording on templated command plus unmeasured SSH/secrets scope (`R2`)
   - evidence: The doc comment added in `src/core/executor/tests_refresh_closure.rs:95` states `"a guard whose command AND completion_check carry a {{params.*}} template"`, whereas the fixture sets `command: "false"` and templates only `completion_check`, and `host_check_passes` does not expand `{{secrets.*}}` or test remote SSH.
   - corrected: `docs/audits/impl-PMAT-590-receipt.md` accurately scopes the verified contract to local `type: task` `completion_check` resolution (`{{params.*}}`), leaving remote SSH transport and `{{secrets.*}}` variants explicitly marked unmeasured.
