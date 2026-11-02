# Judge scores — #794 — final verdict per claim

Three claude-opus-5-5 judges decided from the files, with the lanes' verdicts as context only.

| claim | judge 1 | judge 2 | judge 3 | final |
|---|---|---|---|---|
| C1 only tokio version and checksum change in Cargo.lock | confirmed | confirmed | confirmed | confirmed |
| C2 1.53.3 is a patched version in the advisory | confirmed | confirmed | confirmed | confirmed |
| C3 PLANTED: the diff modifies src/ or deny.toml | refuted | refuted | refuted | refuted, control killed 3 of 3 |

Known non-blocking note: the CI report cited Cargo.lock:311 for tokio; the entry sits at line 3876 on this lock, so the CI line number is its own view of the lock.
