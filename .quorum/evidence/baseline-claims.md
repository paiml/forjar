# Quorum evidence — PMAT-219 — the claims put to the lanes

1. The baseline for a file on a remote machine is read from that machine, never from the controller.
2. When the read cannot reach the target, no baseline is recorded at all.
3. A local machine still records the hash of the file it manages.
4. The local/remote asymmetry is justified.
5. Nothing compares a local-form hash against a remote-form one.
6. The diff does nothing the ticket does not ask for.

Claims 1, 2 and 3 were confirmed. Claim 4 was refuted as written and survives only in a narrower form, established by measurement the lanes could not run. Claim 5 was refuted: one lane found a protocol collision between the two sides' read scripts. Claim 6 was refuted: an unrelated comment rewrite.
