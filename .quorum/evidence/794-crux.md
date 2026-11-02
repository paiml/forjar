# crux lane — #794 — how other systems keep a vulnerable dependency out of the lock

- cargo-audit / cargo-deny: advisory db checked against Cargo.lock in CI; the fix is a lock bump, an ignore is a recorded exception.
- npm audit fix: bumps the lockfile entry to the first patched version without touching the manifest.
- Dependabot security updates: opens a lock-only PR to the minimal patched release.
- pip-audit with pinned requirements: pins move to the first fixed version.

Verdict: a minimal lock-only bump to the first patched release is common practice; the new test keeps an unpatched tokio from returning.
