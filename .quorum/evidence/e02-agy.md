# Independent review — agy /teamwork (plan mode) — #404

Verdict as delivered: "REJECTED … unbounded thread spawning and unqualified
fleet-wide SSH connections, breaks the scoping logic for `--exclude`, silently
hides concurrent errors, and hallucinates support for `--plan-file`." Each
charge was checked against the code.

TAKEN (changed the branch):

> "`ssh_machines_in_scope` only filters the fleet by `machine_filter` (`-m`).
> If an operator restricts execution using `-r`, `-t`, or `-g`, … attempts to
> simultaneously open SSH connections to the entire fleet."

Correct, and the sharpest finding of the review: an O(fleet) setup bill for
an O(1) apply is the cost this issue exists to remove. Masters are now opened
only for machines that host at least one in-scope declared resource.

> "`--exclude` (or `subset`) … removing the resource from `config.resources`,
> but fails to pass the exclusion to the gate … explicitly excluded resources
> are still probed and written as drifted."

Correct. The first cut had measured this itself and left it as "a semantics
call". A locked id with no declaration is now out of scope unconditionally.

> "Threads are entirely unbounded … and `r?` … completely dropping and hiding
> the errors (and success outputs) of any subsequent machines."

Both correct. Waves of 32; every failure reported by machine name.

> "`should_multiplex` explicitly encodes `config.policy.tripwire && !force` …
> duplicating the gate's own internal short-circuit logic."

Correct. One `gate_will_run`, called from both.

REFUTED (did not survive the code):

> "`--plan-file` never executes the new `open_control_masters` function."

True and irrelevant: that path never runs the drift gate either, so there are
no pre-master handshakes on it. The executor starts its own master before its
first remote command, as it always has. Localhost is excluded because there is
no SSH handshake to amortise.

> "Sorting the inputs to a concurrent `std::thread::scope` only makes the
> spawn and join order deterministic."

Agreed on the mechanism, so the comment was corrected — it claimed
"deterministic" and now says what the sort actually fixes: the order the
findings are collected and printed in.

Its CRUX assessment — "below the industry default … eagerly fan out unbounded
threads to open ControlMasters for the entire unfiltered `config.machines`
list" — was correct about the first cut and is what this revision answers:
lazy-by-scope masters and a bounded pool are Ansible's shape.
