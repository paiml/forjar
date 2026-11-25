//! Script generators for `s3_bucket`. Every value reaching a command line has passed
//! the allow-lists in `spec_of`, and is single-quoted anyway.

use super::{spec_of, S3Bucket};
use crate::core::shell_escape::sh_squote;
use crate::core::types::Resource;
use crate::resources::verdict;

/// Name of the lifecycle rule forjar owns.
pub(super) const RULE_ID: &str = "forjar-noncurrent-expire";

pub(super) const Q_ENC: &str =
    "ServerSideEncryptionConfiguration.Rules[0].ApplyServerSideEncryptionByDefault.SSEAlgorithm";
pub(super) const Q_PAB: &str =
    "PublicAccessBlockConfiguration.[BlockPublicAcls,IgnorePublicAcls,BlockPublicPolicy,RestrictPublicBuckets]";
pub(super) const Q_VER: &str = "Status";
pub(super) const Q_POL: &str = "Policy";

pub(super) const Q_LC: &str =
    "Rules[?ID==`forjar-noncurrent-expire`]|[0].[Status,NoncurrentVersionExpiration.NoncurrentDays]";

fn reject(msg: &str) -> String {
    format!("echo {} >&2; exit 1", sh_squote(&format!("REFUSED: {msg}")))
}

/// `aws s3api <op> --bucket B --region R`, ready for extra arguments.
fn api(s: &S3Bucket, op: &str) -> String {
    format!(
        "aws s3api {op} --bucket {} --region {}",
        sh_squote(&s.bucket),
        sh_squote(&s.region)
    )
}

/// Shell that sets `have=yes|no` from `head-bucket`, or EXITS NON-ZERO.
///
/// Only a real HeadBucket 404 or NoSuchBucket error code means "no such bucket" (a bare "404" anywhere in the text, such as a host name or port, does not). Expired credentials, a 403 (someone else's
/// bucket), a 301 (other region), throttling or a missing `aws` are ERRORS: reading
/// one as "absent" would make apply try to create a bucket it cannot own.
fn probe(s: &S3Bucket) -> String {
    format!(
        "LC_ALL=C; export LC_ALL\n\
         if err=\"$({head} 2>&1)\"; then\n  have=yes\nelse\n  case \"$err\" in\n    \
         *'An error occurred (404) when calling the HeadBucket operation'*|*'(NoSuchBucket)'*) have=no;;\n    \
         *) echo 'bucket-lookup-failed' >&2; echo 'ERROR: head-bucket failed and is not a 404' >&2; exit 1;;\n  esac\nfi\n",
        head = api(s, "head-bucket")
    )
}

/// One read: `var` gets the query result, or empty when the read fails.
fn read(s: &S3Bucket, var: &str, op: &str, query: &str) -> String {
    format!(
        "{var}=\"$({} --query {} --output text 2>/dev/null)\" || {var}=''\n",
        api(s, op),
        sh_squote(query)
    )
}

/// Every read-only AWS question the check, the state query and the apply read-back
/// ask. A failed read leaves the variable empty, which no assertion accepts.
fn reads(s: &S3Bucket) -> String {
    let mut o = String::new();
    o += &read(s, "enc", "get-bucket-encryption", Q_ENC);
    o += &read(s, "pab", "get-public-access-block", Q_PAB);
    o += &read(s, "ver", "get-bucket-versioning", Q_VER);
    o += &read(s, "lc", "get-bucket-lifecycle-configuration", Q_LC);
    o += &read(s, "pol", "get-bucket-policy", Q_POL);
    o += "set -f; set -- $pab; pab=\"$*\"; set -- $lc; lc=\"$*\"\n";
    o += "pol=\"$(printf '%s' \"$pol\" | tr -d ' \\t\\r\\n')\"\n";
    o += "has() { case \"$pol\" in *\"$1\"*) return 0;; esac; return 1; }\n";
    o
}

/// The assertions: one per declared property, each with its own marker pair.
fn assertions(s: &S3Bucket) -> Vec<String> {
    let b = &s.bucket;
    let mut a = vec![
        verdict::assert_that(
            "[ \"$have\" = yes ]",
            &format!("s3-bucket-present:{b}"),
            &format!("s3-bucket-absent:{b}"),
        ),
        verdict::assert_that(
            "[ \"$enc\" = 'AES256' ]",
            &format!("s3-sse-aes256:{b}"),
            &format!("s3-sse-missing:{b}"),
        ),
        verdict::assert_that(
            "[ \"$pab\" = 'True True True True' ]",
            &format!("s3-public-access-blocked:{b}"),
            &format!("s3-public-access-open:{b}"),
        ),
    ];
    if s.versioning {
        a.push(verdict::assert_that(
            "[ \"$ver\" = 'Enabled' ]",
            &format!("s3-versioning-enabled:{b}"),
            &format!("s3-versioning-off:{b}"),
        ));
    }
    if let Some(d) = s.noncurrent_expire_days {
        a.push(verdict::assert_that(
            &format!("[ \"$lc\" = 'Enabled {d}' ]"),
            &format!("s3-lifecycle-present:{b}"),
            &format!("s3-lifecycle-missing:{b}"),
        ));
    }
    if s.tls_only {
        a.push(verdict::assert_that(
            &tls_condition(b),
            &format!("s3-tls-only-policy:{b}"),
            &format!("s3-tls-only-policy-missing:{b}"),
        ));
    }
    a
}

/// The ONE policy forjar writes: a single Deny statement for every principal and every
/// action on the bucket and its objects when `aws:SecureTransport` is `false`.
/// `principal` is the spelling of the Principal value. No whitespace, so it is also
/// the whitespace-stripped form the check compares against.
fn tls_policy(b: &str, principal: &str) -> String {
    format!(
        "{{\"Version\":\"2012-10-17\",\"Statement\":[{{\"Sid\":\"forjar-tls-only\",\"Effect\":\"Deny\",\"Principal\":{principal},\"Action\":\"s3:*\",\"Resource\":[\"arn:aws:s3:::{b}\",\"arn:aws:s3:::{b}/*\"],\"Condition\":{{\"Bool\":{{\"aws:SecureTransport\":\"false\"}}}}}}]}}"
    )
}

/// The stored policy, whitespace stripped, must EQUAL the policy forjar writes (AWS may
/// echo `"*"` as `{"AWS":"*"}`). Whole-document equality, not substring predicates: a
/// substring match lets predicates from different statements combine, and lets
/// `NotResource`, `NotPrincipal` or `NotAction` satisfy one. Any extra statement is
/// drift too, which is right because `put-bucket-policy` replaces the whole document.
pub(super) fn tls_condition(b: &str) -> String {
    format!(
        "{{ [ \"$pol\" = {} ] || [ \"$pol\" = {} ]; }}",
        sh_squote(&tls_policy(b, "\"*\"")),
        sh_squote(&tls_policy(b, "{\"AWS\":\"*\"}")),
    )
}

/// Read-only: asks AWS whether the bucket matches the declaration.
pub fn check_script(resource: &Resource) -> String {
    let s = match spec_of(resource) {
        Ok(s) => s,
        Err(e) => return reject(&e),
    };
    format!(
        "{}{}{}",
        probe(&s),
        reads(&s),
        verdict::check_script_from(&assertions(&s))
    )
}

/// Read-only: one line of observed state (what AWS says, not what is declared).
pub fn state_query_script(resource: &Resource) -> String {
    let s = match spec_of(resource) {
        Ok(s) => s,
        Err(e) => return reject(&e),
    };
    format!(
        "{}{}echo \"bucket={} present=$have sse=$enc pab=$pab versioning=$ver lifecycle=$lc tls_only=$({} && echo yes || echo no)\"\n",
        probe(&s),
        reads(&s),
        s.bucket,
        tls_condition(&s.bucket),
    )
}

fn creates(s: &S3Bucket) -> String {
    let loc = if s.region == "us-east-1" {
        String::new()
    } else {
        format!(
            " --create-bucket-configuration LocationConstraint={}",
            sh_squote(&s.region)
        )
    };
    format!(
        "if [ \"$have\" = no ]; then\n  {}{loc}\nfi\n",
        api(s, "create-bucket")
    )
}

fn puts(s: &S3Bucket) -> String {
    let b = &s.bucket;
    let mut o = format!(
        "{} --public-access-block-configuration 'BlockPublicAcls=true,IgnorePublicAcls=true,BlockPublicPolicy=true,RestrictPublicBuckets=true'\n\
         {} --server-side-encryption-configuration '{{\"Rules\":[{{\"ApplyServerSideEncryptionByDefault\":{{\"SSEAlgorithm\":\"AES256\"}}}}]}}'\n",
        api(s, "put-public-access-block"),
        api(s, "put-bucket-encryption"),
    );
    if s.versioning {
        o += &format!(
            "{} --versioning-configuration 'Status=Enabled'\n",
            api(s, "put-bucket-versioning")
        );
    }
    if let Some(d) = s.noncurrent_expire_days {
        o += &format!(
            "{} --lifecycle-configuration '{{\"Rules\":[{{\"ID\":\"{RULE_ID}\",\"Status\":\"Enabled\",\"Filter\":{{\"Prefix\":\"\"}},\"NoncurrentVersionExpiration\":{{\"NoncurrentDays\":{d}}}}}]}}'\n",
            api(s, "put-bucket-lifecycle-configuration")
        );
    }
    if s.tls_only {
        o += &format!(
            "{} --policy {}\n",
            api(s, "put-bucket-policy"),
            sh_squote(&tls_policy(b, "\"*\""))
        );
    }
    o
}

/// Converges the bucket, then re-probes and reads every property back and fails
/// unless AWS agrees with the declaration.
pub fn apply_script(resource: &Resource) -> String {
    let s = match spec_of(resource) {
        Ok(s) => s,
        Err(e) => return reject(&e),
    };
    let mut verify = assertions(&s);
    verify.push(verdict::exit_if_diverged(&format!(
        "s3_bucket {}: AWS does not match the declaration after apply",
        s.bucket
    )));
    format!(
        "set -eu\n{}{}{}{}\n{}{}\n{}\n",
        probe(&s),
        creates(&s),
        puts(&s),
        verdict::flag_init(),
        probe(&s),
        reads(&s),
        verify.join("\n"),
    )
}
