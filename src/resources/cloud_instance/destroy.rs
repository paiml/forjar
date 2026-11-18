//! `state: absent` for `cloud_instance`: fail-closed teardown (FJ-861).

use super::{describe_by, lookup_ids, owner_of, CloudInstance};
use crate::core::shell_escape::sh_squote;

/// Shell that sets `sirs` from the spot-request lookup, or EXITS NON-ZERO.
/// A failed lookup is an error, never "no spot requests". Token checks happen in
/// the cancel loop, after terminate. Relies on `lookup_ids` having run first
/// (it sets the C locale, `set -f` and `unset IFS`).
fn lookup_sirs(cmd: &str) -> String {
    format!(
        "sirs=\"$({cmd})\" || {{ echo 'spot-request-lookup-failed' >&2; echo 'ERROR: spot request lookup failed (provider query exited non-zero)' >&2; exit 1; }}\n\
         set -- $sirs; sirs=\"$*\"\n"
    )
}

/// `state: absent`: terminate ONLY the instances carrying this resource's own
/// `forjar-owner` tag, cancel their spot requests, and report what is true.
/// Never launches. Not gated: the kill switch exists to stop spend, and this
/// only ever removes owned instances (none owned = a no-op success).
///
/// Fail closed: the owner lookup, the spot-request lookup (before anything is
/// terminated) and the post-terminate re-lookup all exit non-zero when the
/// provider query fails, so "destroyed" is only printed after a successful
/// lookup shows nothing alive. Only whole spot request ids (`sir-` + exactly 8
/// lowercase alphanumerics; `None` = not a spot instance) are cancelled. Any
/// other token never blocks terminate (it would keep a billing instance
/// running): it is skipped, reported on stderr, and the script exits non-zero.
pub(super) fn destroy_script(s: &CloudInstance) -> String {
    let region = sh_squote(&s.region);
    let owner = owner_of(s);
    let q = "Reservations[].Instances[].InstanceId";
    let ids = describe_by(s, "forjar-owner", &owner, q);
    let sirs = describe_by(
        s,
        "forjar-owner",
        &owner,
        "Reservations[].Instances[].SpotInstanceRequestId",
    );
    format!(
        "set -eu\n\
         {lookup}\
         if [ -z \"$ids\" ]; then echo 'absent: no owned instance'; exit 0; fi\n\
         {spot}\
         set -- $ids\n\
         aws ec2 terminate-instances --region {region} --instance-ids \"$@\" >/dev/null\n\
         bad=''\n\
         for r in $sirs; do\n\
         [ \"$r\" = None ] && continue\n\
         h=\"${{r#sir-}}\"\n\
         case \"$r\" in sir-*) ;; *) h='!';; esac\n\
         case \"$h\" in *[!0-9a-z]*) h='!';; esac\n\
         if [ \"${{#h}}\" -ne 8 ]; then bad=1; continue; fi\n\
         aws ec2 cancel-spot-instance-requests --region {region} --spot-instance-request-ids \"$r\" >/dev/null\n\
         done\n\
         if [ -n \"$bad\" ]; then echo 'spot-request-malformed' >&2; echo 'ERROR: spot lookup returned something that is not a whole spot request id; that request was not cancelled' >&2; fi\n\
         {relookup}\
         if [ -n \"$left\" ]; then echo \"NOT destroyed: still pending or running: $left\" >&2; exit 1; fi\n\
         echo \"destroyed $ids\"\n\
         [ -z \"$bad\" ] || exit 1\n",
        lookup = lookup_ids("ids", &ids),
        spot = lookup_sirs(&sirs),
        relookup = lookup_ids("left", &ids),
    )
}
