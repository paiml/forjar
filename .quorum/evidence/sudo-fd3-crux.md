# Crux lane — competitive survey — PMAT-159

The lane briefed as crux returned one claim and no survey. The survey below came
from refuter R3, which carried both roles, and is the crux evidence this receipt
relies on. Four systems, each asked the same question: how does the script cross
the privilege boundary, and would sudo's closefrom break it?

## Ansible `become`

Writes a ZIP payload of modules into the control user's home directory over
SFTP or SCP, applies POSIX ACLs (or chown/chmod through a wrapper) so the
privileged user can read it, and then invokes sudo with the ABSOLUTE PATH to the
module. Cleanup is driven from the control node or a wrapper's finally block.
The payload crosses as a file argument, so closefrom cannot affect it.

## Terraform `remote-exec`

Transfers the script to a remote temporary path over SSH or SCP, sets the
executable bit, and runs it by path through the SSH exec channel. File
permissions come from the connection's umask; cleanup is manual or left to the
operator. Again a path, again immune to closefrom.

## SaltStack

Sidesteps the boundary entirely: a long-lived salt-minion already runs as root,
and the payload arrives over the message bus into the agent's cache. No per
command sudo transition exists, so closefrom is inapplicable.

## Puppet

The same shape as SaltStack — a long-lived agent running as root pulls its
catalog over HTTPS and applies resources in-process, using its own secured
directories for any temporary script. sudo is not invoked.

## Verdict

forjar's temp-file transport MATCHES Ansible and Terraform in the property that
matters here: the script crosses the boundary as a file argument, which is
exactly what makes it immune to closefrom, and the two agent-based systems avoid
the transition altogether rather than solving it. R3's objection is that forjar
falls short of the field's defensive posture — the surveyed tools avoid an
attacker-controlled TMPDIR by using a per-run private directory, set an explicit
umask, check that the temporary directory is not world-writable without the
sticky bit, and clean up resiliently on SIGKILL or a crashed parent, whereas
forjar relies on mktemp's default 0600 and a bare shell trap.

All three judges ruled those four gaps NON-BLOCKING, and for a stated reason
rather than by dismissal: the surveyed comparators execute on a REMOTE host as a
different principal, while forjar's wrapper is generated for the invoking user's
own local shell. TMPDIR is that user's own variable, so pointing it somewhere
hostile attacks themselves; mktemp opens with O_EXCL at 0600, which is what an
explicit umask and a sticky-bit check would be defending; and SIGKILL is
uncatchable by POSIX design, so no shell trap in any of the four systems covers
it either. The residue on a kill is a 0600 file owned by the user who ran the
command. Recorded as a known limit, not as a solved problem.
