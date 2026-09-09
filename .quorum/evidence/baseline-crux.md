# Quorum evidence — PMAT-219 — CRUX

The question this defect raises: **when a tool records a baseline for a file on a remote host, whose filesystem does it read?** Three systems, claims `[X]` from documentation memory.

| system | where the baseline comes from |
|---|---|
| Ansible | there is no persisted baseline. Each run gathers facts and reads files ON the target through the connection plugin, so the controller's copy of a path is never the answer to a question about the host [X] |
| Puppet | the agent runs ON the node and reports what it found there. The catalog comes from the master; the state does not [X] |
| Terraform | state is written from provider reads against the real API, and `terraform refresh` re-reads it. The equivalent mistake would be a provider answering from the machine running the CLI, which the provider protocol gives it no way to do [X] |

**What the comparison says about forjar.** All three make this defect hard or impossible to express, and for the same structural reason: the thing that reads state runs where the state is, or talks to an API that does. forjar's controller reads remote files through a transport, which is a deliberate design with a real advantage — one place to run from — and the cost is that "read this path" has two possible meanings and the code must choose the right one every time. It chose wrong here, in one branch, for every transport but containers.

accept(the transport design; make the choice once rather than at each call site). That is what the fix does: one predicate for which machine answers, one reader for how the bytes are read, both shared between the writer and the reader of the same baseline.

The forward-looking question, recorded rather than answered: forjar#495 shows the same choice is still made independently at four more call sites. A design where "whose filesystem" is answered once, at the transport boundary, rather than by each caller consulting a predicate, would make this class unrepresentable rather than merely fixed.
