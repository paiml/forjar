# crux lane — #835 — how other systems bound an advisory exception

- cargo-deny: an ignore is a recorded exception with a reason; the tool warns when an ignore matches nothing, which is the same hygiene the new test enforces.
- cargo-audit: an ignore list in audit.toml silences advisories, so keeping it in sync with deny.toml avoids two policies.
- npm audit with an overrides allowlist: an accepted advisory carries a justification and an expiry owner.
- Dependabot security updates: bumps the direct dependency (here zstd to 0.14) and leaves transitive pins to their owner, tracked upstream.

Verdict: bumping what we own and ignoring only the transitive path we cannot move, with the removal condition written in the reason, is common practice.
