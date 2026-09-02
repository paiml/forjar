# Quorum evidence — #390-C — lane summaries

## probe lane
Ran `apply --json` against a failing task and read the actual payload rather than
assuming its shape. That is what exposed both the defect and, later, the bug in
the first test suite: the reports are under `machines[].resource_reports`, not at
the root.

## crux lane
Ansible's JSON callback carries `msg`, `stdout`, `stderr` and `rc` per task.
Terraform's `-json` machine-readable UI emits `diagnostic` records with summary and
detail. Both carry MORE in the machine surface than on the console, on the theory
that a human can scroll and a pipeline cannot. forjar carried strictly less.

## design lane
Two halves, deliberately coupled: filling the error without restricting the rows
would make a stale row look authoritative. `exit_code` is left null rather than
parsed out of the error string.

## judges
Weighed widening `record_failure` to take an exit code (six call sites, several
with none to give) against leaving `exit_code: None` and naming it. Chose the
latter: a faked exit code is worse than an absent one.

## agy /teamwork
The 1.24.0 review called this "the single biggest thing I am leaving on the table
for the reporter, who is on a stateless CI runner", and set the precondition that
made it safe: all six record_failure strings bounded first.
