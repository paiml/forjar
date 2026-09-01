# Quorum evidence — #390-E — lane summaries

## probe lane (empirical, against the PUBLISHED 1.24.0 binary)
Rather than reason about the generated shell, ran it. Three defects reproduced:
strictness loss (apply reported 1 converged with a failing line), stdin-eating
(second line never ran), delimiter collision (trailing lines escaped the heredoc).
The control — same config minus  — failed correctly, isolating the cause
to the wrapper rather than to the command.

## crux lane (competitive survey)
Five systems checked. GitHub Actions injects  per step; GNU make
uses .SHELLFLAGS=-ec; systemd ExecStart is a direct exec; Terraform remote-exec
does not nest; Ansible runs modules as separate processes. forjar was below the
field default, not making a trade-off.

## design lane
Prototyped  in real bash before touching the generator: it
fixes strictness AND stdin in one change, because the script arrives on a
non-stdin fd. Delimiter collision solved by extending the delimiter until absent
from the body — deterministic, as recipe-determinism-v1 requires.

## judges
Three positions weighed on the delimiter: hash-suffix (opaque), always-extend
(noisy for the common case), extend-on-collision (chosen — the common case keeps
the plain readable delimiter and only a colliding body pays).

## agy /teamwork
Independent review of the 1.24.0 release identified this as the highest-severity
open item and "security-adjacent", recommending it ship separately and fast
rather than bundled behind a message-formatting change.
