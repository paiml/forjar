# Judge scores — #404 fan-out bound and orphan scope

## Bounding the gate's fan-out

| option | honest | cost | verdict |
|---|---|---|---|
| leave `std::thread::scope` over the whole slice (parity with `forjar drift`) | **no** — one thread and one ssh per machine at once; fine at 5, not at 5,000 | none | rejected |
| a semaphore/pool crate | yes | a new dependency for one call site | rejected |
| **waves of `GATE_FANOUT` = 32 via `chunks`** | yes | none; a wave's slowest machine gates the next wave | **chosen** |

Waves cost latency only when a wave has one straggler — the same trade
Ansible makes with `forks`. Every failure in a wave is collected, not the
first: an operator who cannot reach ten hosts is told ten names.

## A locked id with no declaration

| option | honest | verdict |
|---|---|---|
| probe it when no `-t`/`-g` is set (first cut) | **no** — writes `drifted` on a resource the run cannot repair | rejected |
| **out of scope unconditionally; `forjar drift` owns orphans** | yes | **chosen** |

The first cut called this "a semantics call". The judges disagreed: the apply
gate exists to record drift THIS RUN will repair, and nothing about a pruned
or deleted declaration is repairable by this run.
