# pmat MCP lane — #416

`analyze_vacuous_tests` over the whole worktree: 17756 tests examined; 112 `no-failure-mode` finding(s) in files this branch touches, every one a PRE-EXISTING smoke test in a file edited only to delete withdrawn-verb cases (named and accepted in the receipt); ZERO in the falsification file or any production file. The falsifier asserts on functional substrings (`[UNKNOWN]`, clap's
own rejection naming `'lock-audit-trail'`, `signed:false` in the payload, a
successful provenance run) rather than on exit codes alone.

Falsification, tests kept, one hunk at a time:

- `prove` UNKNOWN hunk reverted → `prove_exits_nonzero_on_unknown` **RED**
- provenance banner reverted → `provenance_does_not_claim_slsa_level_3` **RED**
- JSON `predicateType` reverted alone → the same test **RED** on the payload
- `-m` scoping reverted → `prove_machine_filter_isolates_other_machines_unknowns` **RED**
- the whole file against main → **0 passed, 3 failed**

Full lib suite green (three `fj1401` tests updated to the honest outcome);
clippy `-D warnings` 0; fmt clean — counts in `gates`.
