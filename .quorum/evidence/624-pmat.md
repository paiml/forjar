# pmat MCP lane — #624

`analyze_vacuous_tests` run repo-wide (a single-file `-p` is refused) and
filtered to the paths this branch touches: 19853 tests examined; 0 vacuous tests
and 0 conditional skips in tests/falsification_nightly_ships_every_release_linux_target.rs.
The workflow and roadmap hunks carry no tests.

Falsification, test kept: with either musl leg deleted from nightly.yml the
test is RED (rc=101) naming the missing target; with both legs present it is
green (rc=0).
