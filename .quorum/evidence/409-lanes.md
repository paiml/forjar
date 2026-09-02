# Quorum evidence — #409 / #410 — lane summaries

## probe lane
Re-ran the lane's two falsifiers on the merged tree (green), then the
store module tests (15 RED on the old doctrine — re-based), the three
derivation suites, clippy `-D warnings`, fmt and `cargo test --lib`. Two
configs differing only by `store: true`: apply scripts byte-identical,
composite equal through the shipped command.

## crux lane
Nix refuses to build when `sandbox = true` cannot be honoured; Bazel's
strict sandboxing fails loudly when `linux-sandbox` is unavailable; Docker
BuildKit hard-fails when seccomp/namespaces cannot be applied; Guix will not
build a derivation it cannot isolate. None simulates a sandbox or credits
a declared one. This branch moves forjar to that default for execution
(refuse by name) and for scoring (count only what runs); the dry-run
simulation is the recorded gap.

## design lane
Stop scoring what does not execute; keep the lifecycle documented and mark
what cannot run; say "declared, not enforced" on every surface (text, JSON,
book).

## judges
Two decisions scored: option (a) vs (b) for E06; delete vs mark for the
fictional sandbox steps. See the judges file.

## agy /teamwork
Implementation by an agy accept-edits lane in a scrubbed HOME (no publish
or push credentials; its own cargo home and target dir); an independent
plan-mode review in the same scrubbed HOME — see the agy file. Six
refutations and two unique findings; five accepted and fixed, one recorded
as a limit.
