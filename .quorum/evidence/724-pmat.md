# pmat — forjar#724

Tool: pmat 3.42.0 `analyze vacuous-tests`, run on the branch head c72ccb3a over
the two trees the branch touches.

- src/transport: 185 tests examined over 18 files, 0 unmeasured, 0 conditional
  skips reported. ONE vacuous test: `test_154_kill_group_nonexistent_is_noop`
  (tests_dispatch.rs:490, kind no-failure-mode). That file is touched by the
  branch (TEST-NET substitutions at lines 8, 9 and 232) but this test is not:
  the branch's diff of tests_dispatch.rs contains no line of it. Accepted as
  pre-existing, not introduced and not in scope here.
- tests/: 4938 tests examined over 378 files, 84 vacuous repo-wide, 0 in
  tests/falsification_is_local_addr_interface.rs, 0 unmeasured.

Limit, named rather than hidden: the held-address tests return early where the
probe must decline (nonlocal-bind sysctl unreadable or set; no IPv4 route).
The detector did not flag those as conditional skips. They are deliberate —
asserting a held address where the probe is designed to decline would assert
the opposite of the contract — and the tests that cannot skip on Linux carry
the discrimination: 127.0.0.2 dispatch, the broadcast refusals, the
unreadable-sysctl and sysctl-path unit tests.
