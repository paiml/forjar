# Independent review — agy /teamwork (plan mode) — #408

Five claims put to the reviewer; two confirmed, three charged. Each charge was
checked against the code.

TAKEN (changed the branch):

> "`file:<PATH>` can be tricked since `read_key_file` only calls `.trim()`,
> failing to warn on world-readable files …"

The world-readable half is right and is what ssh refuses outright. forjar
now warns with the mode and `chmod 600`; pinned by a case that also proves a
0600 file does NOT warn. Multiple keys separated by inner newlines are not a
trick — the contract is "the file's contents, trimmed", and a two-line file
is a two-line key.

> "`empty_key_file_…` and `unset_env_ref_…` only assert `code == 1`. They can
> pass vacuously on any generic initialization failure."

They also asserted no signature was written, but the point stands: the
reason was unasserted. Each now requires the key source's own message.

CONFIRMED BY THE REVIEWER (attacked and survived):

> "All relevant verbs taking key material (`lock-sign`, `lock-verify-sig`,
> `lock-rotate-keys`, and `lock-verify-chain`) correctly resolve through
> `key_source`. None bypass it to hash the literal directly."

> "The deprecation warning uses a direct `eprintln!` to `stderr`, meaning it
> cannot be suppressed by the `--json` output flag."

REFUTED (did not survive the code):

> "`key_source::read_key_env` uses `std::env::var(var)` but fails to clean up
> the environment with `std::env::remove_var(var)`. This means any spawned
> child process … inherits the plaintext key material."

The operator placed the variable in forjar's environment; children inheriting
it is the same model cosign and docker use, and `remove_var` is unsound in a
multi-threaded process (unsafe from Rust 2024). Recorded as a known limit
with its mitigation, not fixed with an unsoundness.

CRUX: the reviewer placed the design AT the default set by GnuPG and docker
(accept and warn on an argv literal) and BELOW OpenSSH, age and cosign (refuse
it). Agreed — that gap is the 2.0.0 removal the warning announces.
