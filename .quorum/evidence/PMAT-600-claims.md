# Claims — a file check asks for the declared content and mode (PMAT-600)

Counts and heads live in `PMAT-600-lanes.md`; numbers here quote it.

The adjudicated claim set, as the round-3 head `c2e4ee24` states it:

- **C1** `check_script` for `state: file` asserts existence, the sha256 of the declared bytes (`sha256sum`, else macOS `shasum -a 256`), an octal `mode`, and `owner`/`group` where declared, each with its own divergent marker
- **C2** `apply --refresh` rewrites a stale file that has NO lock entry (the issue's reproduction), for `content:` and `source:`
- **C3** `apply --refresh` rewrites a stale file the lock calls converged — the path measured on intel, where one run printed `content changed` and then `0 converged`
- **C4** `apply --refresh` restores a drifted mode
- **C5** a file that already matches is not rewritten (mtime unchanged), so the fix cannot be an always-divergent check

## Where the claims came from

The defect was found in the field: a fleet pin file on intel stayed at `0.65.2`
under a declaration of `0.68.2` through a `--refresh` apply that reported the
drift. The issue's first mechanism (refresh seeding) was one door; the locked
resource was the other, and both close at the check.
