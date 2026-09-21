# Judges — a file check asks for the declared content and mode (PMAT-600)

Round count and heads: see `PMAT-600-lanes.md`.

## CONFIRMED

1. [diff] C1 — The `state: file` check asserts existence, the sha256 of the declared bytes, an octal mode and owner/group where declared, each read into a variable and then tested with its own divergent marker, so a failing check names what is stale instead of only that something is.
   - evidence: `src/resources/file.rs:35` routes the file arm through `file_assertions` at `src/resources/file.rs:64`; the content assertion is `src/resources/file.rs:75`, the mode `src/resources/file.rs:87`, owner/group `src/resources/file.rs:116`; all three lanes of round 3 cited these lines.
2. [test] C2 — With no lock entry, `apply --refresh` now rewrites a stale file for both the content and the source shape, where on main it seeded the stale file as converged and left the old bytes in place.
   - evidence: `tests/falsification_refresh_writes_a_stale_file.rs:114` asserts the file content after a refresh apply for both shapes; RED on main at commit 59db2bc6 with "Content: --refresh left the stale file in place".
3. [test] C3 — With a lock entry that calls the file converged, `apply --refresh` now rewrites it, which is the path measured on intel where one run printed the content drift and then reported zero converged.
   - evidence: `tests/falsification_refresh_writes_a_stale_file.rs:130` applies once, overwrites the file, refreshes and asserts the declared content; RED on main with "Content: --refresh reported and did not write".
4. [test] C4 — A drifted mode is restored by `apply --refresh`, because mode is part of what the file declares and the check now asserts it.
   - evidence: `tests/falsification_refresh_writes_a_stale_file.rs:148` chmods the target to 0600 and requires 0644 after a refresh; RED on main with "--refresh left mode 0600".
5. [test] C5 — A file that already matches is not rewritten by `apply --refresh`, so the fix cannot be passed by a check that reports divergence unconditionally; this test passes on main and here.
   - evidence: `tests/falsification_refresh_writes_a_stale_file.rs:160` compares the target's mtime before and after a refresh apply across a one-second sleep and requires them equal.

## REFUTED

1. [codegen] R1 — The first content assertion put the hash pipeline inside the test brackets, and forjar's own I8 gate refused every generated file check because bashrs read the substitution's operator as a test operator at Error severity.
   - corrected: the value is read into a variable first and then tested (`src/resources/file.rs:75`); 13553 lib tests pass; the bashrs rule is fixed in paiml/bashrs#367 (issue #366).
2. [scope] R2 — The issue named refresh seeding of unlocked resources as the mechanism, which would have fixed only one door; a locked resource went through the same existence-only check and printed drift while converging nothing.
   - corrected: the fix is in `check_script` (`src/resources/file.rs:35`), which both doors consult; `tests/falsification_refresh_writes_a_stale_file.rs:114` covers the unlocked door and `tests/falsification_refresh_writes_a_stale_file.rs:130` the locked one.
