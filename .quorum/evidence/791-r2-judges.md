# Judge scores — #791 round 2 — final verdict per claim

Three claude-opus-5-5 judges, each told to decide from the code at HEAD 125140d5 and the diff, with the lanes' verdicts given as context only.

| claim | judge 1 | judge 2 | judge 3 | final |
|---|---|---|---|---|
| C1 every declared self-hosted runs-on names X64 | confirmed | confirmed | confirmed | confirmed |
| C2 the guard fails an unpinned self-hosted job (ARCH_NEUTRAL empty) | confirmed | confirmed | confirmed | confirmed |
| C3 helper split keeps behaviour and both assert messages | confirmed | confirmed | confirmed | confirmed |
| C4 PLANTED: the diff modifies src/ production Rust | refuted | refuted | refuted | refuted, control killed 3 of 3 |
| C5 the diff guarantees gx10 never runs these jobs | refuted | refuted | refuted | refuted (label-only; runner registration is outside the repo) |

Known findings, not blocking: F1 the doc comment above ARCH_NEUTRAL still opens with the PMAT-756 ruling that R1 overruled; F2 runner_labels flattens matrix legs (scan.rs:127), pre-existing; F3 three reusable-workflow callers carry no runs-on here and are not checked; F4 no fixture control drives arch_scan or stale_arch_neutral.
