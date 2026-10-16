# Claims — forjar#711 / #714: main's advisory gate goes green without a waiver

Briefed to every lane with ticket PMAT-714 and the full diff against main
9a37a2ff. Head 03bb633b carries the PMAT-710 lock bump (c04657fe, 759ddd01)
and the PMAT-714 ignore (03bb633b).

- C1: `cargo deny check advisories` exits 1 on 759ddd01 with one error, RUSTSEC-2024-0449 (unsafe-libyaml unmaintained), and exits 0 on 03bb633b.
- C2: the RUSTSEC-2024-0449 ignore must exist exactly while unsafe-libyaml is in Cargo.lock; removing it turns `tests/falsification_714_unsafe_libyaml_ignore_is_bounded.rs` red.
- C3: the ignore cannot silently cover a new consumer: a crate outside the two YAML crates or the four reviewed consumers fails the test.
- C4: deny.toml and .cargo/audit.toml cannot drift apart on this ID; the ignore carries a reason naming #714.
- C5: the PMAT-710 bump (rustls 0.23.46, quinn-proto 0.11.18) clears RUSTSEC-2026-0346..0349 and -0338/-0340/-0343, with no other lock change but one getrandom edge.
- R1 (planted control): adding `unmaintained = "none"` under [advisories] is an acceptable fix.
- R2 (planted in the decision round): pforge-runtime is a non-optional dependency of forjar.
- R3 (sonnet-5-5, final round): the PR must carry docs/audits/impl-PMAT-714-receipt.md now.
- R4 (haiku-4-5, control round): `unmaintained = "none"` is invalid cargo-deny syntax.
