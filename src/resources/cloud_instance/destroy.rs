//! `state: absent` script generation for `cloud_instance` (moved out of `mod.rs`
//! to keep it under the file-size cap; the generated script is unchanged).

use super::{describe_by, lookup_ids, owner_of, CloudInstance};
use crate::core::shell_escape::sh_squote;

/// `state: absent`: terminate ONLY the instances carrying this resource's own
/// `forjar-owner` tag, cancel their spot requests, and report what is true.
/// Never launches. Not gated: the kill switch exists to stop spend, and this
/// only ever removes owned instances (none owned = a no-op success).
pub(super) fn destroy_script(s: &CloudInstance) -> String {
    let region = sh_squote(&s.region);
    let owner = owner_of(s);
    let ids = describe_by(
        s,
        "forjar-owner",
        &owner,
        "Reservations[].Instances[].InstanceId",
    );
    let sirs = describe_by(
        s,
        "forjar-owner",
        &owner,
        "Reservations[].Instances[].SpotInstanceRequestId",
    );
    let lookup = lookup_ids("ids", &ids);
    format!(
        "set -eu\n\
         {lookup}\
         if [ -z \"$ids\" ]; then echo 'absent: no owned instance'; exit 0; fi\n\
         sirs=\"$({sirs})\" || {{ echo 'spot-request-lookup-failed' >&2; echo 'ERROR: spot request lookup failed (provider query exited non-zero)' >&2; exit 1; }}\n\
         set -- $sirs; sirs=\"$*\"\n\
         for r in $sirs; do\n\
         h=\"${{r#sir-}}\"\n\
         case \"$r\" in None) continue;; sir-*) ;; *) h='!';; esac\n\
         case \"$h\" in *[!0-9a-z]*) h='!';; esac\n\
         if [ \"${{#h}}\" -ne 8 ]; then echo 'spot-request-malformed' >&2; echo 'ERROR: spot request lookup returned something that is not a spot request id' >&2; exit 1; fi\n\
         done\n\
         set -- $ids\n\
         aws ec2 terminate-instances --region {region} --instance-ids \"$@\" >/dev/null\n\
         for r in $sirs; do case \"$r\" in None) continue;; esac; \
         aws ec2 cancel-spot-instance-requests --region {region} --spot-instance-request-ids \"$r\" >/dev/null; done\n\
         left=\"$({ids})\" || {{ echo 'instance-lookup-failed' >&2; echo 'ERROR: post-terminate instance lookup failed; destroy not confirmed' >&2; exit 1; }}\n\
         if [ -n \"$left\" ]; then echo \"NOT destroyed: still pending or running: $left\" >&2; exit 1; fi\n\
         echo \"destroyed $ids\"\n",
    )
}
