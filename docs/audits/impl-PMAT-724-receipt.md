# Implementation receipt — PMAT-724 — is_local_addr recognises addresses held by a local interface (PR #724)

verdict: DONE (after-the-fact receipt) — PR #724 merged on 2026-10-20 without a harness receipt; this one records a re-run of every test file that PR added or changed, at release head 645990ca, on kfocus: 61 of 61 lib tests and 6 of 6 falsification tests passed, both commands rc=0. It does not record a RED-first run; that is listed under Gaps.

## Identity

| field | value |
|---|---|
| ticket | PMAT-724 (alias: PMAT-1835, the infra id PR #724 names) |
| PR | #724 `fix(transport): is_local_addr recognises addresses held by a local interface` |
| PR branch | fix/is-local-addr-interfaces |
| merged | 2026-10-20T11:43:31Z, merge commit 1ea4559d3c73ee2f548c49110aa18a756b32339c |
| measured at | 645990ca2c23f39930d45001298042b28d3a9643 (chore/bump-1.34.0, PR #886); 1ea4559d is an ancestor of it |
| kind of receipt | written after the merge, from a fresh measurement; not written by the implementation harness at the time |

## What PR #724 changed

Read from `gh pr view 724 --json files`:

| file | + | - |
|---|---|---|
| src/transport/local_iface.rs | 76 | 0 |
| src/transport/mod.rs | 7 | 1 |
| src/transport/tests_local_iface.rs | 168 | 0 |
| src/transport/tests_dispatch.rs | 3 | 3 |
| src/transport/tests_dispatch_b.rs | 11 | 4 |
| tests/falsification_is_local_addr_interface.rs | 129 | 0 |
| .quorum/fix-is-local-addr-interfaces.json and .quorum/evidence/724-*.md (6 files) | 363 | 0 |

The four test files are the ones run below. All four exist at 645990ca.

## Measured

Host kfocus (Linux 6.8.0-147-generic), fresh clone of paiml/forjar at `/home/noah/tmp/ic1111-forjar-886`, detached at 645990ca2c23f39930d45001298042b28d3a9643, clean tree (`git status --short` 0 lines). Toolchain from the repo's `rust-toolchain.toml`: rustc 1.93.0 (254b59607 2026-01-19), cargo 1.93.0 (083ac5135 2025-12-15). `CARGO_BUILD_JOBS=8`, default target dir inside the clone (the environment sets no `CARGO_TARGET_DIR`). No other cargo or rustc process was running at the start. Start 2026-11-11T10:14:55Z, end 2026-11-11T10:18:13Z.

| # | command | result | rc | finished UTC |
|---|---|---|---|---|
| 1 | `nice -n 10 cargo test --lib -- transport::tests_local_iface transport::tests_dispatch` | `test result: ok. 61 passed; 0 failed; 0 ignored; 0 measured; 13706 filtered out; finished in 5.42s` | 0 | 2026-11-11T10:17:08Z |
| 2 | `nice -n 10 cargo test --test falsification_is_local_addr_interface` | `test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s` | 0 | 2026-11-11T10:18:13Z |

Command 1 by module, counted from the `... ok` lines of the log:

| module | tests ok |
|---|---|
| transport::tests_local_iface | 9 |
| transport::tests_dispatch | 26 |
| transport::tests_dispatch_b | 26 |

`tests_local_iface.rs` has 10 `#[test]` functions; one is `#[cfg(not(target_os = "linux"))]` and is not compiled on this host, so 9 is the whole Linux set.

The 9 `tests_local_iface` tests: `link_local_is_a_false_negative`, `mapped_v6_is_judged_as_v4`, `nonlocal_bind_never_claims_local`, `route_source_addr_is_local`, `sysctl_value_parse`, `sysctl_path_follows_family`, `unowned_addrs_stay_remote`, `whole_loopback_net_is_local`, `unreadable_sysctl_declines` (each prefixed `test_local_iface_`).

The 6 falsification tests: `falsify_mapped_held_addr_is_local`, `falsify_iface_addr_is_local`, `falsify_directed_broadcast_stays_remote`, `falsify_unowned_addr_stays_remote`, `falsify_iface_addr_dispatches_locally`, `falsify_loopback_alias_dispatches_locally`.

Full log on kfocus: `/home/noah/tmp/ic1111-run.log` (sha256 prefix 7311c12178bd8bc8); the script that produced it: `/home/noah/tmp/ic1111-run.sh`.

## Host conditions the tests depend on

Several of these tests return early, still reported `ok`, when the nonlocal-bind sysctl is unreadable or set, or when the host has no IPv4 route. On kfocus at the time of the run:

- `net.ipv4.ip_nonlocal_bind = 0` and `net.ipv6.ip_nonlocal_bind = 0`, both files readable.
- `ip -4 route show default` printed 2 routes; `ip -4 route get 1.1.1.1` exited 0.

So the conditions for those early returns were not present. The run does not print which branch each test took (cargo captures the output of a passing test), so this is read from the host and not from the tests themselves.

## Gaps

- No RED-first run. PR #724's body says 3 of 4 new tests failed against the unfixed function; that was not re-measured here. This receipt shows the tests pass at 645990ca, not that they fail without the fix.
- Only the four test files PR #724 touched were run. The full lib suite (13706 other tests filtered out) and the other integration tests were not run by this receipt.
- Linux only. The macOS path and the one `#[cfg(not(target_os = "linux"))]` test were not exercised.
- The early-return branches were not instrumented; see the section above.
- The original onboarding case (a machine declared by an overlay address, applied on itself with no sshd) was not reproduced end to end.

IMPL-PMAT-724-RECEIPT-END
