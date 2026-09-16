# PMAT-576 — the instruments this booking was measured with

| tool | what it said |
|---|---|
| `scripts/dogfood/tagged.sh` on origin/main | `GATE T FAIL v1.31.0 is reachable from HEAD, at or above the floor v1.25.0, and has no row in docs/roadmaps/releases.yaml: a release that was cut and never declared` |
| `scripts/release-goal.sh window v1.31.0` | the row, measured: cut 2026-09-16T08:50:38Z, prs [548, 563, 568, 569, 571, 575], tickets [PMAT-547, PMAT-557, PMAT-560, PMAT-562, PMAT-564, PMAT-574], cookbook 710b0877 |
| `scripts/release-goal.sh cut v1.31.0 --next v1.32.0` | wrote that row, declared v1.32.0 due 2026-09-18T08:50:38Z, and moved three `release:v1.31.0` labels (PMAT-526, PMAT-528, PMAT-529) to `release:v1.32.0` because those tickets were not in this window |
| `scripts/dogfood/tagged.sh` on this branch | `GATE T v1.31.0 cut 2026-09-16T08:50:38Z: 6 PR(s), 6 ticket(s) labelled release:v1.31.0 ok` |
| `cargo publish --locked` | `Published forjar v1.31.0 at registry crates-io` |
| crates.io API | `max_version` 1.31.0 |
| `cargo update -p forjar --precise 1.31.0` (cookbook) | `Updating forjar v1.30.0 -> v1.31.0`, 91 dependencies unchanged; `cargo check --workspace` clean |
| `analyze_vacuous_tests` | see below |
| `pmat analyze vacuous-tests` | not re-run for this branch: it adds no test and changes no `.rs`, so the repository's standing 432 of 19790 (2.2%) cannot have moved for a reason this branch caused |

## The quality gate

This branch touches `docs/roadmaps/*.yaml` and one `docs/audits` receipt. It
adds no code, so the code-shaped gates have nothing to measure here and are not
claimed. Gate T is the gate this branch exists to satisfy, and it is quoted
above on both sides.
