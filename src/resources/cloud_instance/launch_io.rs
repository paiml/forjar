//! Launch I/O for `cloud_instance` (FJ-861): a deterministic client token,
//! adopt-by-token, one parseable id line and a capacity exit code.
//!
//! Contract of `apply` after the gates and the owner/key lookups:
//! - stdout's last line is `launched <id>` (new box), `adopted <id>` (found by
//!   its client token after a failed or unusable launch) or, from the caller,
//!   `present <ids>`;
//! - exit 75 ([`CAPACITY_EXIT`]): aws reported a capacity error CODE;
//! - the id's state is read before `launched` (pending or running only); a dead
//!   or forgotten box's token is spent and the chain moves on (see `TAIL`), at most 32 calls,
//!   then `REFUSED: launch-token-chain-exhausted`;
//! - exit 1: no usable id and nothing adoptable, a state read that failed or
//!   was unknown (`REFUSED: instance-state-failed <id>` / `instance-state-unknown`), or a token lookup that failed
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

/// Longest chain of tokens (run-instances calls) one apply may walk. The
/// chain restarts at the day's first token on every apply and a box with a
/// one-hour lifetime can die 24 times a UTC day, so the bound is 24 plus margin.
const MAX_GENERATIONS: u32 = 32;

/// A "does not exist" state read for a launch older than this (seconds) is a
/// replay of an instance EC2 has forgotten; a fresher one may be a real new box.
const FORGOTTEN_AFTER_SECS: u64 = 900;

/// Tries of the state read after a launch, and the fixed pause between them (s).
const STATE_TRIES: u32 = 3;
const STATE_PAUSE_SECS: u32 = 2;

/// Token for the launch parameters and a UTC day number: `fj-` plus 48 hex
/// digits of a hash, so ASCII, 51 characters and no user text. The day bounds
/// how long a destroyed box's token can answer a later launch.
pub(super) fn token_on(input: &str, day: u64) -> String {
    let h = crate::tripwire::hasher::hash_string(&format!("{input}|{day}"));
    format!("fj-{}", &h["blake3:".len()..][..48])
}

/// Everything `run-instances` is built from that the resource can change:
/// the instance key plus instance type, price, lifetime, image reference,
/// the AMI reference (the pin or the default `resolve:ssm:` reference as
/// written, NOT the id it resolves to) and labels (sorted `key=value`
/// joined by U+001F, so declaration order does not matter), the volume tag specification and a hash of the
/// declared user-data file content (read here, only hashed, never kept). EC2 rejects a reused token whose parameters differ, so any
/// of these changing must give a new token. The key stays as it is.
pub(super) fn token_input(s: &CloudInstance) -> String {
    let mut sorted = s.tags.clone();
    sorted.sort();
    let labels = sorted.join("\u{1f}");
    let ud = s
        .user_data_file
        .as_deref()
        .map_or_else(String::new, user_data_hash);
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}",
        super::key_of(s),
        s.instance_type,
        s.max_price,
        s.max_lifetime_min,
        s.image,
        super::ami::image_id(&s.ami),
        s.storage.volume_tagspec(&sorted),
        ud,
        labels
    )
}

/// Hash of the declared user-data file BYTES, or the fixed word `unreadable` when the path is not a regular
/// file, is over the guard's size limit or cannot be read (so `/dev/zero` or a FIFO is never read here).
/// Every such file gives the same word; the apply guard refuses them before run-instances.
fn user_data_hash(p: &str) -> String {
    let path = std::path::Path::new(p);
    let small = std::fs::metadata(path)
        .is_ok_and(|m| m.is_file() && m.len() <= super::userdata::MAX_BYTES as u64);
    if !small {
        return "unreadable".into();
    }
    crate::tripwire::hasher::hash_file(path).unwrap_or_else(|_| "unreadable".into())
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

/// EC2 answers a reused token with the ORIGINAL reply (id and launch time),
/// even after that box died. So after a reply comes back the id's state is
/// read. A dead box spends the token and the next one is
/// `sha256(token + dead id)`, which depends only on values EC2 returns, so a
/// retried apply walks the same chain and lands on the same live box. EC2
/// stops listing a terminated box after about an hour but still replays its
/// id: a state read that says "does not exist" is then a spent token only if
/// the reply's launch time is older than 15 minutes; a fresher one may be a new
/// box not yet visible, so it is retried and then refused, never chained.
/// Any other state-read failure is retried the same way, then refused.
/// The launch time is checked by a strict pattern before `date -u -d`.
const TAIL: &str = r#" --client-token "$1" --query 'Instances[0].[InstanceId,LaunchTime]' --output text 2>&1; }
tok=@TOKEN@; gen=0
while :; do
gen=$((gen+1))
res="$(run_once "$tok")" && rc=0 || rc=$?
new=''; lt=''; for w in $res; do new=$lt; lt=$w; done
h="${new#i-}"
case "$new" in i-*) ;; *) h='!';; esac
case "$h" in *[!0-9a-f]*) h='!';; esac
b="${lt%"${lt#???????????????????}"}"; r="${lt#"$b"}"
case "$b" in [0-9][0-9][0-9][0-9]-[01][0-9]-[0-3][0-9]T[0-2][0-9]:[0-5][0-9]:[0-5][0-9]) ;; *) b='';; esac
case "$r" in Z|+00:00) ;; .*Z|.*+00:00) f="${r#.}"; f="${f%Z}"; f="${f%+00:00}"; case "$f" in ''|*[!0-9]*) b='';; esac;; *) b='';; esac
lte=''; if [ -n "$b" ]; then lte="$(date -u -d "${b}Z" +%s 2>/dev/null)" || lte=''; fi
case "$lte" in ''|*[!0-9]*) lte='';; esac
if [ "$rc" -eq 0 ] && [ "${#h}" -ge 8 ] && [ "${#h}" -le 17 ] && [ -n "$lte" ]; then
try=0
while :; do
try=$((try+1))
st="$(@STATE@)" && src=0 || src=$?
cls=fail; old=0
if [ "$src" -eq 0 ]; then case "$st" in ''|None) cls=nf;; *) cls=ok;; esac
else case "$st" in *'An error occurred (InvalidInstanceID.NotFound)'*) cls=nf;; esac; fi
if [ "$cls" = ok ]; then break; fi
if [ "$cls" = nf ]; then
now="$(date -u +%s 2>/dev/null)" || now=0 # bashrs disable-line=DET002
case "$now" in ''|*[!0-9]*) now=0;; esac
if [ $((now - lte)) -gt @AGE@ ]; then old=1; break; fi
fi
if [ "$try" -ge @TRIES@ ]; then break; fi
sleep @PAUSE@
done
if [ "$old" -eq 1 ]; then :
elif [ "$cls" = ok ]; then
case "$st" in
pending|running) echo "launched $new"; exit 0;;
shutting-down|terminated|stopping|stopped) ;;
*) printf '%s
' "$st" >&2; echo 'instance-state-unknown' >&2; exit 1;;
esac
else printf '%s
' "$st" >&2; echo "REFUSED: instance-state-failed $new" >&2; exit 1; fi
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
        .replace("@AGE@", &FORGOTTEN_AFTER_SECS.to_string())
        .replace("@TRIES@", &STATE_TRIES.to_string())
        .replace("@PAUSE@", &STATE_PAUSE_SECS.to_string())
        .replace("@CAPACITY@", &codes)
        .replace("@EXIT@", &CAPACITY_EXIT.to_string())
        .replace("@LOOKUP@", &lookup_ids("adopted", &find))
}
