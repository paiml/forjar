# Implementation receipt — PMAT-663 — a service's apply reaches what its check asserts, for every `state:`

verdict: PASS — for each of the four states and each of the four starting conditions, the generated apply script, executed against a fake `systemctl`, leaves the unit in the declared state, the generated check then exits 0, and a second apply makes no mutating call. `state: enabled` no longer fails its own check on an inactive unit, and `state: disabled` disables instead of enabling. A contradictory `enabled:` is a validation error. Reverting the fix turns the `enabled` and `disabled` rows red.

## Identity

- ticket PMAT-663 (forjar#663); kind: code
- branch `fix/663-service-state`, from `main` @ 49b33fbd
- files: `src/resources/service.rs`, `src/core/parser/resource_types.rs`, `src/resources/tests_service_converge.rs` (new), `src/resources/mod.rs`, `docs/book/src/03-resources.md`, `docs/roadmaps/roadmap.yaml`

## Which meaning was implemented

The issue proposed `enabled` = enable + start and `disabled` = stop + disable.
The book documents something else, in `docs/book/src/03-resources.md`, "Service States":

| State | Action |
|-------|--------|
| `enabled` | `systemctl enable` (no start/stop) |
| `disabled` | `systemctl disable` (no start/stop) |

`contracts/forjar-unit-exec-parity-v1.yaml` says the same thing. Its `present(u)`
constrains activity only for `running` and `stopped`. The documented meaning is
the one implemented. `state: enabled|disabled` now manages boot enablement
only, and its check asserts enablement only. A unit that must also run is
declared `state: running`, which defaults to `enabled: true`.

## Before / after, per state (no `enabled:` declared)

| state | apply before | check before | apply after | check after |
|---|---|---|---|---|
| running | start if inactive; enable | active ∧ enabled | unchanged | unchanged |
| stopped | stop if active; enable | ¬active ∧ enabled | unchanged | unchanged |
| enabled | enable | active ∧ enabled (never converges from inactive) | enable | enabled |
| disabled | **enable** | active ∧ enabled | disable | ¬enabled |

## Evidence (rust 1.93.0, `-j 8`)

RED on main's `service.rs`, with the new tests applied (`cargo test --lib test_663`), 3 of 7 failing:

    state=enabled enabled=None start=(active=false, enabled=true): check exited 1 after apply
    state=enabled enabled=None start=(active=false, enabled=false): check exited 1 after apply
    state=disabled enabled=None start=(active=true, enabled=true): apply left (true, true), want (true, false)
    state=disabled enabled=None start=(active=true, enabled=true): check passed on a host that is not converged
    ...
    test_663_contradictory_enabled_is_a_validation_error: state: enabled with enabled: false must be refused, got []

GREEN: `cargo test --lib service` reported 176 passed and 0 failed.

Revert proof. Only `service.rs` was restored to main, with the validator and the tests kept:

| test | result |
|---|---|
| test_663_enabled_converges | FAILED (4 divergences: every inactive start, for `enabled:` absent and `true`) |
| test_663_disabled_converges | FAILED (9 divergences: every start enabled by apply, and the check passing an unconverged host) |
| test_663_running_converges, test_663_stopped_converges | ok |
| test_663_fake_systemctl_discriminates, both validation tests | ok |

Only `resource_types.rs` was restored to main: `test_663_contradictory_enabled_is_a_validation_error` FAILED and the other 6 passed.

Full checks on this tree: `cargo fmt --all -- --check` rc=0, `make lint` rc=0,
and `cargo test --locked --no-fail-fast` gave 18548 passed and 5 failed, with
the lib alone at 13573 passed and 0 failed. All 5 failures are in four
integration targets that this change does not touch. Each fails the same way
on main @ 49b33fbd on the same host:

- `falsification_comply_count_cannot_run_inside_itself::the_process_cap_is_applied_and_fails_closed`: the account runs fewer than 513 processes, so the cap never bites.
- `falsification_replace_running_binary::the_probe_can_fail`: `cp` over a running binary succeeds on this kernel.
- `falsification_verb_cli_reader_parity` ×2: the tempdir lists in sorted order.
- `falsification_release_workflow_shape::rule2_release_yml_is_the_only_v_star_tag_push_producer`: bench.yml, ci.yml and coverage.yml also push on `v*` tags. This is a pre-existing red on main, not caused by this host.

IMPL-PMAT-663-RECEIPT-END
