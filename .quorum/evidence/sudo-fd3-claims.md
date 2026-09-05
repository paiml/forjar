# Quorum evidence — PMAT-159 — the seventeen claims as put to the refuters

The dossier below is what the three refuters attacked and what the three judges
then adjudicated. It is reproduced here as it was posed, imprecise citations and
all, because the corrections in `sudo-fd3-judges.md` are only reviewable next to
the text they correct. Claim ids: A1-A5 (transport lane), B1-B5 (falsifier/test
lane), C1 (the lane briefed as crux, which returned one claim and no survey),
T1-T6 (the independent-stack review).

## Lane A — transport

- A1: the elevated script is written to a private `mktemp` file and immediately
  safeguarded by a shell trap on `EXIT`, `INT` and `TERM`. Cited at
  `dispatch.rs:279`; relies on mktemp's default 0600 rather than an explicit chmod.
- A2: the script text crosses into the temp file via a securely quoted heredoc
  and is passed to the elevated bash as a file argument, removing the dependency
  on an inherited descriptor.
- A3: the wrapper propagates the script's exit status without interference from
  the trap, and leaves the elevated shell's stdin untouched.
- A4: `in_declared_privilege_context` is the singular emitter of the wrapper —
  three callers, one whole script each — so the bare `EXIT` trap and the fixed
  variable name cannot collide.
- A5: the `timeout:` feature is the only remaining user of `/dev/fd/`, and it is
  safe because the descriptor is created inside the already elevated shell.

## Lane B — falsifier and tests

Lane B returned bare assertions with no long-form prose; the refuters were told
to treat each as unsupported and verify it themselves.

- B1: the emulated test uses a fake sudo to close descriptors at or above 3 and
  confirms execution through the production emitter.
- B2: the RED evidence is credible because the test verifies the old `/dev/fd/3`
  form fails with "No such file or directory".
- B3: the live test properly manages four combinations of sudo presence and
  `FORJAR_REQUIRE_SUDO_TESTS`, panicking when the capability is required.
- B4: the strictness test implements ordered byte-offset checks which pin
  operational order but still risk passing vacuously on broken textual scripts.
- B5: the updated tests in `tests_sudo.rs` are stronger but retain a vacuous-test
  shape, asserting only text properties through `.contains()`.

## Lane C — one claim, no survey

- C1: the sudo wrapper previously used an fd-3 heredoc transport that failed when
  sudo's closefrom closed the descriptor. Cited at `dispatch.rs:297`.

The crux role this lane was briefed for is carried instead by refuter R3, whose
four-system survey is in `sudo-fd3-crux.md`.

## Independent-stack review

- T1: the `mktemp` invocation creates a private 0600 file that root can read,
  thwarting symlink exposure and TOCTOU.
- T2: the `EXIT INT TERM` trap cleans up on signals without clobbering the exit
  status, so 130 on ^C still propagates.
- T3: `mktemp` and `cat` are guarded with `|| exit 1`, preventing a partial
  script from being elevated when TMPDIR is invalid or the disk is full.
- T4: the heredoc delimiter is single-quoted, disabling expansion in the payload.
- T5: the emulated test is a real falsifier, proving the transport depends on
  file behaviour rather than on a string check.
- T6: the commit message bounds the safety of the unscoped trap rather than
  overstating it.

## Outcome

Fifteen survived, two were killed (B3 and B4). The adjudication, with the
correction for each killed claim and the anchor each judge moved, is in
`sudo-fd3-judges.md`, which is this receipt's `claims_digest`.
