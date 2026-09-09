# Quorum evidence — PMAT-220 — CRUX

The question: **when a tool can execute inside an isolated namespace on the same host, what stops it from reading the host's filesystem and calling that the namespace's state?** Three systems, claims `[X]` from documentation memory.

| system | what prevents it |
|---|---|
| Docker / OCI tooling | the API is the boundary. `docker exec` and `docker cp` name a container, and there is no call that accidentally reads the host path instead — the wrong thing is not expressible [X] |
| systemd-nspawn with `machinectl` | the same shape: `machinectl shell` targets a machine, and host paths are reached only by an explicit bind mount the operator declared [X] |
| Ansible with the `community.docker` or `chroot` connection plugins | the connection plugin owns every read and write, so a module cannot reach the controller's filesystem by naming a path; the plugin decides where a path resolves [X] |

**What the comparison says.** All three make the mistake unrepresentable by putting the boundary in the API rather than in a predicate. forjar's boundary is a function each caller must remember to consult, which is why this defect appeared at four call sites and needed a tree-wide rule to keep it away.

accept(the transport design, which buys one place to run from) and reject(the predicate-per-caller shape). The forward-looking note, carried from PMAT-219's survey and now with evidence behind it: a design where "whose filesystem" is answered at the transport boundary rather than by each caller consulting a predicate would make this class unrepresentable. Two tickets have now fixed instances of it, and the tree-wide rule added here is a guard rather than a cure.
