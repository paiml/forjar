# Judge scores — #835 — final verdict per claim

degraded: same-family. Three claude-haiku-4-5 judges decided from the diff, with the lanes' replies as context only.

| claim | judge 1 | judge 2 | judge 3 | final |
|---|---|---|---|---|
| C1 zstd 0.13 to 0.14 in Cargo.toml, lock at 0.14.3 | confirmed | confirmed | confirmed | confirmed |
| C2 14 ignores per file, reasoned, no wildcard | confirmed | confirmed | confirmed | confirmed |
| C3 blake3 contract total_obligations 4 to 3 only | confirmed | confirmed | confirmed | confirmed |
| C4 new falsification test and PMAT-840 roadmap row | confirmed | confirmed | confirmed | confirmed |
| C5 PLANTED: the diff modifies src/ | refuted | refuted | refuted | refuted, control killed 3 of 3 |

All three judges agreed with the lanes' PASS. Lanes caught the canary 3 of 3, judges 3 of 3.
