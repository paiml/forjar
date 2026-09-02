# Quorum evidence — #407 (CRUX audit E05) — lane summaries

## probe lane
Drove the verb through the binary — `forjar verb call drift --json` — over a
machine at 203.0.113.9 with a bait file on the controller whose bytes match
the locked hash. Unfixed: `drifted: false`, no `unchecked`, no census, in
0.00s; fixed: the host is contacted and cannot answer (`actual_hash:
MISSING`, `ssh: connect to host 203.0.113.9`). Then planted a
`completion_check: touch FIRED` in a locked task and watched for the file:
the first cut created it; the fix does not, and the census says why.

## crux lane
Ansible's `--check` and `ansible-inventory` never execute a play's own
commands under a read-only flag — check mode is a contract, and modules
that cannot honour it skip and REPORT the skip. Terraform's `plan` refreshes
state by querying providers against the TARGET (that is the whole value of
the refresh) but never runs `local-exec` provisioners. Salt's `test=True`
returns what WOULD change and names states it could not evaluate. Puppet
`--noop` reports every resource it did not apply. Every one of them queries
the target and none executes user-declared shell under a read-only promise;
the verb was below that bar on both counts and is now at it.

## design lane
One detector, two callers. The verb now makes the CLI's own call with
template-resolved resources, through `sanitize_config`, and carries the
CLI's census shape. The one option the CLI has that the verb must not take
— `run_task_checks` — is set false here and disclosed twice, derived from
the detector's recorded skips so it cannot rot into a second list.

## judges
Two decisions scored: where the falsifier lives, and whether the verb should
run the config's completion_check. See the judges file.

## agy /teamwork
Independent stack review; see the agy file.
