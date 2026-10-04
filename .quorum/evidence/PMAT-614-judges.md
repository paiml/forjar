# Judges — the nightly's Windows leg builds OpenSSL with Strawberry perl (PMAT-614)

Round count and heads: see `PMAT-614-lanes.md`.

## CONFIRMED

1. [step] C1 — On the Windows leg the build step exports OPENSSL_SRC_PERL as the Strawberry perl the image ships, and cargo is started with exactly that value; all three round-2 lanes read the step and agreed.
   - evidence: the test plants the perl and asserts cargo recorded `C:/Strawberry/perl/bin/perl.exe` at `tests/falsification_nightly_windows_openssl_perl.rs:146`, and that the perl was probed with `-MParams::Check` at `tests/falsification_nightly_windows_openssl_perl.rs:152`.
2. [step] C2 — A Strawberry perl that cannot load Params::Check, or no perl at all, stops the step with a non-zero exit and an ::error:: naming the perl, before cargo is ever started, so an unusable perl is never handed to openssl-src.
   - evidence: both cases run in one loop at `tests/falsification_nightly_windows_openssl_perl.rs:161`, which asserts failure at `tests/falsification_nightly_windows_openssl_perl.rs:168`, the message at `tests/falsification_nightly_windows_openssl_perl.rs:172` and no cargo call at `tests/falsification_nightly_windows_openssl_perl.rs:176`.
3. [legs] C3 — The Linux x86_64 cargo leg, the aarch64 cross leg and the macOS cargo leg build with OPENSSL_SRC_PERL unset, so the Windows fix cannot reach a leg that was already green.
   - evidence: the three legs are enumerated at `tests/falsification_nightly_windows_openssl_perl.rs:186`, and the recorded value must be `unset` at `tests/falsification_nightly_windows_openssl_perl.rs:199`.
4. [test] C4 — The falsifier lifts the parsed run script of the step named Build release binary, asserts shell bash, substitutes only the two expressions GitHub would, and runs it under bash with errexit and pipefail. Each declared mutation reddened its named tests and the restore went green.
   - evidence: the step is selected at `tests/falsification_nightly_windows_openssl_perl.rs:58`, the shell is asserted at `tests/falsification_nightly_windows_openssl_perl.rs:60`, and it is run at `tests/falsification_nightly_windows_openssl_perl.rs:113`. The mutations are declared at `tests/falsification_nightly_windows_openssl_perl.rs:24`: deleting the block gave 2 failed, dropping the exit gave 1, and an unconditional block gave 1. The restore gave 3 passed.
5. [scope] C5 — ci.yml runs the falsifier as its own step, and after round 1 the branch changes only the two workflows, the new test and one appended roadmap entry: 242 insertions and zero deletions against origin/main.
   - evidence: `git diff --stat origin/main 215f2e8a` lists four files with no deletion, and the step runs `cargo test --locked --test falsification_nightly_windows_openssl_perl`, which names `tests/falsification_nightly_windows_openssl_perl.rs:1`.

## REFUTED

1. [roadmap] R1 — The author claimed that the first commit changed only what PMAT-614 asked for. In fact it carried pmat's YAML round-trip of the roadmap: 326 lines reflowed, 68 kind fields stripped and timestamps unquoted, which is the regression the cb21xx ratchet notes forbid.
   - corrected: all three round-1 lanes refuted it. Commit 215f2e8a restored origin/main's roadmap byte for byte and appended the PMAT-614 entry as text, so `git diff origin/main` shows 22 insertions and 0 deletions for the file.
2. [roadmap] R2 — The PMAT-614 entry was registered inprogress with no release field, so CB-2114 would count it as NO-RELEASE; the flash-high lane in round 1 named it.
   - corrected: the entry now carries `release: 1.33.0` and the matching labels. That value is the milestone forjar#614 already carries on GitHub, and the ratchet notes name the milestone as the authority for the field.
