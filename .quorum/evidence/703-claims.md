# Claims — forjar#703: quinn-proto 0.11.17 -> 0.11.18 (RUSTSEC-2026-0338, -0340, -0343) and a lock guard

Briefed to every lane with the ticket row and the full diff against main
9a37a2ff; head c98fb5c6. The PR carries `Refs #703`, not a closing keyword:
the cargo-deny unmaintained advisory on unsafe-libyaml (RUSTSEC-2024-0449) is
not reachable by any bump and stays open on the ticket.

- C1: the lock moves quinn-proto 0.11.17 -> 0.11.18, version and checksum only; no other Cargo.lock line changes.
- C2: with that lock `cargo audit` reports 0 vulnerabilities (main: 3, all quinn-proto 0.11.17), and an independent advisory source agrees.
- C3: no ignore or exemption is added: the diff touches only Cargo.lock, the roadmap row and the new test; deny.toml and .cargo/audit.toml are untouched.
- C4: the guard discriminates: on main's 0.11.17 lock the test target goes red, on the branch's lock it is green.
- C5: the guard refuses in both directions: a vulnerable, pre-release, unparseable or absent quinn-proto entry fails, so zero matches cannot pass.
- C6: scope is stated honestly: `cargo deny check advisories` stays red at this head, and only on RUSTSEC-2024-0449.
