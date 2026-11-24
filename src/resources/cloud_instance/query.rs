//! Read-only provider queries and the state-query script for `cloud_instance`.

use super::{key_of, owner_of, spec_of, CloudInstance};
use crate::core::shell_escape::sh_squote;
use crate::core::types::Resource;

pub(super) fn squote_keep_newlines(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\"'\"'"))
}

pub(super) fn reject(msg: &str) -> String {
    format!("echo {} >&2; exit 1", sh_squote(&format!("REFUSED: {msg}")))
}

/// Read-only provider query printing `query` for live instances whose
/// `tag_key` tag equals `tag_val`.
pub(super) fn describe_by(s: &CloudInstance, tag_key: &str, tag_val: &str, query: &str) -> String {
    format!(
        "aws ec2 describe-instances --region {region} \
         --filters {tag} 'Name=instance-state-name,Values=pending,running' \
         --query {query} --output text",
        region = sh_squote(&s.region),
        tag = sh_squote(&format!("Name=tag:{tag_key},Values={tag_val}")),
        query = sh_squote(query),
    )
}

/// Shell that sets `var` from the provider query `cmd`, or EXITS NON-ZERO.
///
/// A failed lookup (expired credentials, throttling, no `aws`) or a reply that
/// is not a list of instance ids is an ERROR, never "absent": reading a failure
/// as an empty answer would report success while a box may still be billing.
/// Pathname expansion is switched off and the C locale forced BEFORE any word
/// splitting, the reply is normalised to single-space-separated tokens (a
/// whitespace-only reply is empty = absent), and every token must be a whole
/// instance id (`i-` + 8 to 17 hex digits; `*` and `?` are malformed). Works
/// with or without `set -e`.
pub(super) fn lookup_ids(var: &str, cmd: &str) -> String {
    format!(
        "LC_ALL=C; export LC_ALL; set -f; unset IFS\n\
         {var}=\"$({cmd})\" || {{ echo 'instance-lookup-failed' >&2; echo 'ERROR: instance lookup failed (provider query exited non-zero)' >&2; exit 1; }}\n\
         set -- ${var}; {var}=\"$*\"\n\
         for i in ${var}; do\n\
         h=\"${{i#i-}}\"\n\
         case \"$i\" in i-*) ;; *) h='!';; esac\n\
         case \"$h\" in *[!0-9a-f]*) h='!';; esac\n\
         if [ \"${{#h}}\" -lt 8 ] || [ \"${{#h}}\" -gt 17 ] || [ \"$h\" = '!' ]; then\n\
         echo 'instance-lookup-malformed' >&2; echo 'ERROR: instance lookup returned something that is not an instance id' >&2; exit 1\n\
         fi\n\
         done\n"
    )
}

/// Instances of this exact declaration (key = name + region + image digest).
pub(super) fn describe(s: &CloudInstance) -> String {
    describe_by(
        s,
        "forjar-key",
        &key_of(s),
        "Reservations[].Instances[].InstanceId",
    )
}

/// Script reporting observed state (read-only provider query), by OWNER: any
/// live instance this resource owns counts, whatever image it was launched
/// with. `state: absent` reads the same way as the absent check.
pub fn state_query_script(resource: &Resource) -> String {
    let s = match spec_of(resource) {
        Ok(s) => s,
        Err(e) => return reject(&e),
    };
    let owned = describe_by(
        &s,
        "forjar-owner",
        &owner_of(&s),
        "Reservations[].Instances[].InstanceId",
    );
    format!(
        "{}if [ -n \"$ids\" ]; then echo \"present $ids\"; else echo absent; fi\n",
        lookup_ids("ids", &owned)
    )
}
