# Quorum evidence — #446 — lane summaries

## probe lane
RED commit `30f33afd` (tests only): 0/8 falsifier cases pass — every one dies on `unrecognized subcommand`. Branch: 8/8 falsifier, 14/14 unit tests in `src/cli/tests_446.rs`, `verb::partition` 20/20, full lib 13,392 green after the partition rows; clippy `-D warnings` 0; stable rustfmt clean. The first facts script failed forjar's own bashrs I8 gate (SC2242) — caught by the falsifier, fixed by replacing `continue` with a `skip` flag.

## crux lane
Ansible: `ansible <host> -m shell -a '…'` (ad-hoc exec) and `-m setup` (facts) are the two verbs every operator learns first; Salt: `salt '*' cmd.run` and grains; Puppet: `facter` plus `bolt command run`. All four separate "run this now" from "converge this". forjar had neither verb: an operator adding disk/permission probes to a YAML and waiting on the gate to see a `df` is the ticket's literal complaint. `exec` and `facts` put forjar at the industry default for the one-off half; `doctor --machine` goes one step past `facter` by joining the facts with the config (which directories the declared resources will write, which executables their providers need) — closer to Bolt's `puppet-agent --noop` preflight than to bare facts.

## design lane
One transport (`exec_script`), three thin verbs; the facts script is the single source both `facts` and `doctor --machine` read; the doctor's checks are pure functions over `Facts` + the config so every threshold and message is unit-tested without a host.

## judges
One decision scored: facts as a report now vs. a resolver-visible facts model now. See the judges file.

## agy /teamwork
Independent plan-mode review in a scrubbed HOME — see the agy file.
