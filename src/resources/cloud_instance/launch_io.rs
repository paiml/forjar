//! Launch I/O for `cloud_instance` (FJ-861): a deterministic client token,
//! adopt-by-token, one parseable id line and a capacity exit code.
//!
//! Contract of `apply` after the gates and the owner/key lookups:
//! - stdout's last line is `launched <id>` (new box), `adopted <id>` (found by
//!   its client token after a failed or unusable launch) or, from the caller,
//!   `present <ids>`;
//! - exit 75 ([`CAPACITY_EXIT`]): aws reported a capacity error CODE;
//! - the id's state is read before `launched` (pending or running only); a dead
//!   box's token is spent and the chain moves on (see `TAIL`), at most 5 calls,
//!   then `REFUSED: launch-token-chain-exhausted`;
//! - exit 1: no usable id and nothing adoptable, a state read that failed or
//!   was unknown (`instance-state-failed` / `instance-state-unknown`), or a token lookup that failed
//!   or returned something that is not an instance id (never read as "no box");
//! - any other aws failure keeps aws's own exit code.

use super::{lookup_ids, CloudInstance};
use crate::core::shell_escape::sh_squote;

/// Exit code for "no capacity here, try the next candidate".
pub(super) const CAPACITY_EXIT: i32 = 75;

/// Opens the `run_once` function: the `run-instances` command, whose last
/// argument is the client token (`$1`), so a retry can send the same
/// command with the next token of the chain.
pub(super) const OPEN: &str = "run_once() { ";

/// Longest chain of tokens (run-instances calls) one apply may walk.
const MAX_GENERATIONS: u32 = 5;

/// Token for the launch parameters and a UTC day number: `fj-` plus 48 hex
/// digits of a hash, so ASCII, 51 characters and no user text. The day bounds
/// how long a destroyed box's token can answer a later launch.
pub(super) fn token_on(input: &str, day: u64) -> String {
    let h = crate::tripwire::hasher::hash_string(&format!("{input}|{day}"));
    format!("fj-{}", &h["blake3:".len()..][..48])
}

/// Everything `run-instances` is built from that the resource can change:
/// the instance key plus instance type, price, lifetime, image reference
/// and labels. EC2 rejects a reused token whose parameters differ, so any
/// of these changing must give a new token. The key stays as it is.
pub(super) fn token_input(s: &CloudInstance) -> String {
    format!(
        "{}|{}|{}|{}|{}|{:?}",
        super::key_of(s),
        s.instance_type,
        s.max_price,
        s.max_lifetime_min,
        s.image,
        s.tags
    )
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
    token_on(&token_input(s), day_of(now))
}

/// EC2 answers a reused token with the ORIGINAL reply, even after that box
/// died. So after an id comes back its state is read (fail closed); a dead
/// box spends the token and the next one is `sha256(token + dead id)`, which
/// depends only on values EC2 returns, so a retried apply walks the same
/// chain and lands on the same live box.
const TAIL: &str = r#" --client-token "$1" --query 'Instances[0].InstanceId' --output text 2>&1; }
tok=@TOKEN@; gen=0
while :; do
gen=$((gen+1))
res="$(run_once "$tok")" && rc=0 || rc=$?
new=''; for w in $res; do new=$w; done
h="${new#i-}"
case "$new" in i-*) ;; *) h='!';; esac
case "$h" in *[!0-9a-f]*) h='!';; esac
if [ "$rc" -eq 0 ] && [ "${#h}" -ge 8 ] && [ "${#h}" -le 17 ]; then
st="$(@STATE@)" || { printf '%s
' "$st" >&2; echo 'instance-state-failed' >&2; exit 1; }
case "$st" in
pending|running) echo "launched $new"; exit 0;;
shutting-down|terminated|stopping|stopped) ;;
*) printf '%s
' "$st" >&2; echo 'instance-state-unknown' >&2; exit 1;;
esac
if [ "$gen" -ge @GENS@ ]; then echo 'REFUSED: launch-token-chain-exhausted' >&2; exit 1; fi
tok="$(printf '%s%s' "$tok" "$new" | sha256sum | cut -c1-64)"
if [ "${#tok}" -ne 64 ]; then echo 'launch-token-derive-failed' >&2; exit 1; fi
continue
fi
printf '%s\n' "$res" >&2
if [ "$rc" -ne 0 ]; then case "$res" in
@CAPACITY@) echo 'launch-capacity-unavailable' >&2; exit @EXIT@;;
esac; fi
@LOOKUP@if [ -n "$adopted" ]; then echo "adopted $adopted"; exit 0; fi
if [ "$rc" -ne 0 ]; then exit "$rc"; fi
echo 'launch-output-malformed' >&2; exit 1
done
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
    let region = sh_squote(&s.region);
    // `$tok` is the current token of the chain: `fj-` + hex, or 64 hex digits.
    let find = format!(
        "aws ec2 describe-instances --region {region} \
         --filters \"Name=client-token,Values=$tok\" \
         'Name=instance-state-name,Values=pending,running' \
         --query 'Reservations[].Instances[].InstanceId' --output text",
    );
    let state = format!(
        "aws ec2 describe-instances --region {region} --instance-ids \"$new\" \
         --query 'Reservations[0].Instances[0].State.Name' --output text 2>&1"
    );
    // Match the structured `An error occurred (<Code>)` prefix of the aws CLI.
    let codes = CAPACITY_CODES
        .map(|c| format!("*'An error occurred ({c})'*"))
        .join("|");
    TAIL.replace("@TOKEN@", &sh_squote(&token(s)))
        .replace("@STATE@", &state)
        .replace("@GENS@", &MAX_GENERATIONS.to_string())
        .replace("@CAPACITY@", &codes)
        .replace("@EXIT@", &CAPACITY_EXIT.to_string())
        .replace("@LOOKUP@", &lookup_ids("adopted", &find))
}
