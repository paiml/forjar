# Quorum evidence — 1.27.0 fleet P0 — lane rulings

Three independent lanes on the release branch at 9ad01216, each in its own standalone clone. Verdicts: 3 FAIL. Every finding was re-run by the orchestrator before it was acted on; none was accepted on the lane's word.

## The finding two lanes found independently, and it reproduced

Lanes 2 and 3 both found that `drift --dry-run` bypasses the out-of-scope refusal. `cmd_drift_dry_run` calls `machine_state_dirs` directly rather than going through `collect_machine_locks`, where the guard lived. Measured against the real binary:

```
drift            -f one.yaml -m faraway  ->  error: machine 'faraway' is not
                                             declared by this config
drift --dry-run  -f one.yaml -m faraway  ->  0 resource(s) would be checked
                                             exit 0
```

That is the same false green over zero machines the guard exists to prevent, reached through the command an operator uses FIRST when they are unsure what a run will do. Lane 3 named the fix precisely, down to the call site. The refusal now runs in both doors, and the mutation confirms which line the new case measures.

Worth recording plainly: the branch introduced a guard against a false green and left a door open to the same false green. Four green falsification cases did not catch it because none of them exercised `-m` at all — the acceptance criterion had no test. Lanes 1 and 3 both said so.

## The finding one lane found, and it reproduced

Lane 2 found that the rustup bootstrap block still hard-codes `export PATH="$HOME/.cargo/bin:$PATH"` after the new prelude prepends `${CARGO_HOME:-$HOME/.cargo}/bin`. Measured with `CARGO_HOME=/opt/toolchain`:

```
after prelude:                 /opt/toolchain/bin:...
after the bootstrap export:    <user home>/.cargo/bin:/opt/toolchain/bin:...
```

`rustup-init` honours `CARGO_HOME` too, so the hard-coded line puts a directory rustup did not install into ahead of the one it did. Both now use the same expression, and the unit test that asserts the repair asserts the `CARGO_HOME` form rather than the literal `.cargo/bin:$PATH` it used to match.

## The finding that was answered rather than fixed

Lane 1 refused the branch for carrying PMAT-213 and PMAT-214 alongside PMAT-212, on the ground that PMAT-212's ticket does not ask for them. That is accurate about the ticket and it is a deliberate choice, not an oversight.

The operator declared four issues P0 with the fleet blocked and the drift lane unable to be green. The self-hosted pool was measured at 14 of 16 runners busy; three separate pull requests would serialise three CI passes behind that queue. The bundling buys one pass. What it must not cost is the per-ticket record, so each ticket keeps its own commit, its own implementation receipt, its own falsification test and its own roadmap row, and the pull request body says the bundling is deliberate and why.

A reviewer who disagrees with that trade can still read the branch one ticket at a time, which is the property that makes the trade acceptable.
