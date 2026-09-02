# Quorum evidence — #404 (CRUX audit E02) — lane summaries

## probe lane
Captured the binary's own `ssh` spawns through a shim first on PATH rather
than timing anything on a shared build box. Unfixed: the first spawns are bare
gate queries with no control-socket option; fixed: the first spawn is the master open
and every later remote command carries the socket. Scope was probed the same
way — an out-of-scope resource must not appear in stderr and must not come
back `drifted` in the lock. Measured the two regressions the agy lane named
(`--exclude` unscoped; whole-fleet masters under `-r`) before fixing them.

## crux lane
Ansible relies on the ssh client's own ControlPersist and opens connections
lazily against the FILTERED inventory, bounded by `forks`. Salt-SSH opens per
target as execution demands, from a worker pool. pyinfra and Fabric connect on
the first remote operation against a specific host. None connects to hosts it
will not use, and none runs unbounded. The first cut was below that bar on
both counts — eager masters to the unfiltered fleet, unbounded threads — and
is now at it: masters only for machines with an in-scope resource, waves of
32 for the gate.

## design lane
The hoist is a guard held for the whole of `cmd_apply`; dropping it early
closes sockets the executor is about to use. The executor's own master start
had to learn the difference between a socket it opened and one it found. The
gate's scope predicate is the executor's filter predicate inverted, literally,
and a locked id with no declaration is out of scope unconditionally — the
executor cannot touch what `config.resources` no longer holds.

## judges
Three options for the fan-out bound and two for the orphan rule were scored;
see the judges file.

## agy /teamwork
Independent stack review in plan mode. Five charges: three taken and pinned
(exclude scope, fleet-wide masters, unbounded fan-out with error hiding), two
refuted against the code (`--plan-file`, "deterministic" wording). See the agy
file.
