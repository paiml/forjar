# crux lane — S1 — how other systems keep a declared count honest

The question: should a summary count in a spec file be checked against the list it summarises?

- OpenAPI linters (Spectral): rules compare declared fields against the structure they describe and fail the build on drift.
- Cargo: Cargo.lock is rejected under --locked when it disagrees with the manifest, so a derived figure cannot rot silently.
- Kubernetes CRD schema validation: declared replica or item counts are validated against the objects present.
- Protobuf: a declared length prefix that disagrees with the payload is a parse error, not a warning.

Verdict: checking a summary against its list is common practice; pv's VS-COUNT-001 does it and the new test makes the same check run in cargo test without pv installed.
