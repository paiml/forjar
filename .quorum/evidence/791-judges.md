# Judge scores — #791 — final verdict per claim

Three claude-opus-5-5 judges, each given the checkout, the diff, the claims and the refuters' answers, told to decide from the code.

| claim | judge 1 | judge 2 | judge 3 | final |
|---|---|---|---|---|
| C1 every self-hosted clean-room runs-on now names X64 | confirmed | confirmed | confirmed | confirmed |
| C2 the guard fails an unpinned self-hosted job (ARCH_NEUTRAL empty) | confirmed | confirmed | confirmed | confirmed |
| C3 helper split keeps behaviour and both assert messages | confirmed | confirmed | confirmed | confirmed |
| C4 PLANTED: ARCH_NEUTRAL admits any unpinned job whose workflow file is listed | refuted | refuted | refuted | refuted, control killed 3 of 3 |
| C5 the diff guarantees gx10 never runs these jobs | refuted | refuted | refuted | refuted (label-only; runner groups are outside the repo) |

Known findings, not blocking: F1 the doc comment above ARCH_NEUTRAL still opens with the PMAT-756 ruling that R1 overruled (edit refused by the repo's ticket hook, logged); F2 runner_labels flattens matrix legs (scan.rs:127), so one pinned leg masks an unpinned leg, pre-existing; F3 three reusable-workflow callers carry no runs-on here and are not checked.
