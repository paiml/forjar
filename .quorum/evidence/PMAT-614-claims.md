# Claims — the nightly's Windows leg builds OpenSSL with Strawberry perl (PMAT-614)

Counts and heads live in `PMAT-614-lanes.md`.

- **C1** on the Windows leg only, the build step exports `OPENSSL_SRC_PERL=C:/Strawberry/perl/bin/perl.exe`
- **C2** the step fails before cargo, naming the perl, when that perl is missing or cannot load `Params::Check`
- **C3** the Linux, aarch64-cross and macOS legs build with `OPENSSL_SRC_PERL` unset
- **C4** the falsifier executes the parsed step, so a comment cannot satisfy it, and each declared mutation reddens its named tests
- **C5** ci.yml runs the falsifier, and the branch changes nothing the ticket did not ask for

## The measured symptom

forjar#614: under `shell: bash`, `windows-latest` resolves Git's msys perl for
`openssl-src`'s `./Configure`. That perl lacks `Params::Check`, so the
x86_64-pc-windows-msvc leg died, the all-or-nothing `release` job was skipped,
and the `nightly` tag sat 11 days behind main while every other leg built.
fleet-bins reported NIGHTLY-STALE for forjar on every lambda run.
