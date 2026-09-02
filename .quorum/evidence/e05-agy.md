# Independent review — agy /teamwork (plan mode) — #407

Verdict as delivered: the branch "fails the adversarial review … local-addressed
non-local transports still fall back to reading the controller, service tasks
evaluate side-effecting commands under readOnlyHint: true, and unknown machine
filters yield silent false cleans." Five charges; each checked against the code.

TAKEN (changed the branch):

> "`src/tripwire/drift/file.rs` checks `!is_local_addr(&m.addr)` without
> verifying the transport, meaning a Docker or SSH transport bound to
> `127.0.0.1` falls through to `check_file_drift`, reading the controller's
> local filesystem."

Correct for container and pepita transports. `reads_the_controller` now
says local-transport only; four routing tests pin it.

> "a `task` with `task_mode: service` … falls through to `detect_nonfile_drift`
> … thereby executing its `completion_check` via the `verdict::single`
> generator under `readOnlyHint: true`."

Correct, and the sharpest finding: `owns` excludes service tasks on purpose
(their digest was recorded against the PID-file query), and the generic
state query prefers the declared check. Declined under `run_task_checks:
false` with the same census reason; a binary-level case pins it with a trap.

> "unknown `-m` filters … return an entirely empty `DriftOutput` … completely
> silent and indistinguishable from a clean host."

Correct. Refused by name now, listing the declared machines.

> "`actual == \"ERROR\" || actual == \"MISSING\"` … `MISSING` is returned
> when the SSH transport successfully connects but the script fails."

Taken as a test tightening: the detail must show the failed attempt to reach
203.0.113.9.

REFUTED (did not survive the code):

> "A `completion_check` for a machine with no lock is entirely excluded from
> the `unattended_skipped` disclosure; because `machine_lock` uses `continue`,
> it skips `scan.absorb`."

`machine_lock` takes `&mut scan.unchecked` and records the machine before the
`continue`; an unlocked machine is disclosed as UNCHECKED, and has no converged
resource whose check could have been declined.

Its CRUX conclusion — below the industry default while any config-declared
command runs under the read-only promise — was correct about the first cut
and is what the service-task decline answers.
