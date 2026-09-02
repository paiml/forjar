# Quorum evidence — #390-A/B — lane summaries

## probe lane
Ran the same config both ways and compared, rather than asserting parallel "fails".
The decisive moment was the falsification step: the first suite passed against
fully-reverted code, exposing that a one-resource fixture never reaches the wave
path at all (machine.rs:183 requires >1 change). Rewritten with two-resource
fixtures plus a guard-the-guard test.

## crux lane
Kubernetes runs admission control on every object regardless of how many are
applied concurrently; Terraform applies its `-parallelism` graph but every node
still passes the same post-apply read; Ansible's free strategy varies ORDER, never
which checks run. No surveyed system varies its verification by scheduler. forjar
did, silently.

## design lane
The signature change is why this survived: the script was consumed by `and_then`
inside the spawn closure. Introduced `WaveResult` as a named type rather than
widening a bare tuple, so the fourth element is documented where it is declared.

## judges
Weighed capturing inside the thread (racy on the run dir, and `ensure_run_dir` is
not synchronised) against carrying the script out and capturing in the existing
sequential Phase-3 loop. Chose the latter: Phase 3 already serialises, so no new
concurrency is introduced.

## agy /teamwork
The 1.24.0 review flagged #390-B as "HIGHER SEVERITY THAN #390 ITSELF" and warned
the two must land together — capture without verification leaves `forjar logs
--failures` blind, because `summary.failed` is only incremented by
`RunMeta::record_resource`.
