//! Launch I/O for `cloud_instance` (FJ-861): a deterministic client token,
//! adopt-by-token, one parseable id line and a capacity exit code.
//!
//! Contract of `apply` after the gates and the owner/key lookups:
//! - stdout's last line is `launched <id>` (new box), `adopted <id>` (found by
//!   its client token after a failed or unusable launch) or, from the caller,
//!   `present <ids>`;
//! - exit 75 ([`CAPACITY_EXIT`]): aws reported a capacity error CODE;
//! - exit 1: no usable id and nothing adoptable, or a token lookup that failed
//!   or returned something that is not an instance id (never read as "no box");
//! - any other aws failure keeps aws's own exit code.

use super::{lookup_ids, CloudInstance};
use crate::core::shell_escape::sh_squote;

/// Exit code for "no capacity here, try the next candidate".
pub(super) const CAPACITY_EXIT: i32 = 75;

/// Opens the capture of `run-instances` (stdout and stderr in `res`).
pub(super) const OPEN: &str = "res=\"$(";

/// Token for an instance key and a UTC day number: `fj-` plus 48 hex digits
/// of a hash, so ASCII, 51 characters and no user text. The day bounds how
/// long a destroyed box's token can answer a later launch.
pub(super) fn token_on(key: &str, day: u64) -> String {
    let h = crate::tripwire::hasher::hash_string(&format!("{key}|{day}"));
    format!("fj-{}", &h["blake3:".len()..][..48])
}

/// UTC day number of a Unix time in seconds.
pub(super) fn day_of(secs: u64) -> u64 {
    secs / 86_400
}

/// Today's token for this declaration.
pub(super) fn token(s: &CloudInstance) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    token_on(&super::key_of(s), day_of(now))
}

const TAIL: &str = r#" --client-token @TOKEN@ --query 'Instances[0].InstanceId' --output text 2>&1)" && rc=0 || rc=$?
new=''; for w in $res; do new=$w; done
h="${new#i-}"
case "$new" in i-*) ;; *) h='!';; esac
case "$h" in *[!0-9a-f]*) h='!';; esac
if [ "$rc" -eq 0 ] && [ "${#h}" -ge 8 ] && [ "${#h}" -le 17 ]; then echo "launched $new"; exit 0; fi
printf '%s\n' "$res" >&2
if [ "$rc" -ne 0 ]; then case "$res" in
@CAPACITY@) echo 'launch-capacity-unavailable' >&2; exit @EXIT@;;
esac; fi
@LOOKUP@if [ -n "$adopted" ]; then echo "adopted $adopted"; exit 0; fi
if [ "$rc" -ne 0 ]; then exit "$rc"; fi
echo 'launch-output-malformed' >&2; exit 1
"#;

/// aws error codes that mean "no capacity for this candidate".
const CAPACITY_CODES: [&str; 4] = [
    "InsufficientInstanceCapacity",
    "SpotMaxPriceTooLow",
    "MaxSpotInstanceCountExceeded",
    "Unsupported",
];

/// Shell that follows the `run-instances` arguments: adds the token and the
/// id-only output, then decides the outcome (see the module doc).
pub(super) fn tail(s: &CloudInstance) -> String {
    let token = token(s);
    let find = format!(
        "aws ec2 describe-instances --region {} --filters {} \
         'Name=instance-state-name,Values=pending,running' \
         --query 'Reservations[].Instances[].InstanceId' --output text",
        sh_squote(&s.region),
        sh_squote(&format!("Name=client-token,Values={token}")),
    );
    // Match the structured `An error occurred (<Code>)` prefix of the aws CLI.
    let codes = CAPACITY_CODES
        .map(|c| format!("*'An error occurred ({c})'*"))
        .join("|");
    TAIL.replace("@TOKEN@", &sh_squote(&token))
        .replace("@CAPACITY@", &codes)
        .replace("@EXIT@", &CAPACITY_EXIT.to_string())
        .replace("@LOOKUP@", &lookup_ids("adopted", &find))
}
