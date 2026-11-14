//! FJ-861: `cloud_instance` — ONE declared spot instance, inert until gated.
//!
//! Nothing in this module calls a cloud. It *generates* shell; the shell runs
//! `aws` only when the machine it is applied on has it, and `apply_script`
//! refuses outright unless a parity-gate file, a budget cap and a kill switch
//! are all declared (and the gate file says pass, and the kill switch is
//! absent). `plan_diff` is pure Rust: no process, no network.
//!
//! Reads (`check`, `state_query`, `state: absent`) run only read-only
//! `describe-instances` and teardown of OWNED instances, and are not gated. A
//! lookup that fails or returns something that is not a list of instance ids is
//! an error (non-zero), never "absent".
//!
//! Known limits:
//! - Ownership is `forjar-owner = <name>-<region>`. Two config files (or
//!   stacks) in one account that declare the same `name` in the same region
//!   share it: one sees the other's box as its own, and its `state: absent`
//!   terminates it. Uniqueness is enforced within one config only. Keep names
//!   distinct across configs.
//! - The instance key includes the image digest, so changing `image:` on a
//!   running box launches a second box; the old one lives until its lifetime
//!   ends or `state: absent`. The spend cap is per launch, not summed.

use crate::core::shell_escape::sh_squote;
use crate::core::types::Resource;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_b1;

/// A validated `cloud_instance` declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudInstance {
    /// The resource's own name (`name:`), unique per declared resource.
    pub name: String,
    pub region: String,
    pub instance_type: String,
    pub max_price: String,
    pub image: String,
    pub max_lifetime_min: u32,
    pub tags: Vec<String>,
}

/// Resolve a validated [`CloudInstance`] from a resource declaration.
///
/// # Errors
///
/// Returns `Err` when the declaration is not aws, lacks a field, names an
/// image without an `@sha256:<64 hex>` digest, has a non-positive price or
/// lifetime, or carries a value outside the strict allow-list (every value
/// reaches a boot script or an `aws` command line).
pub fn spec_of(resource: &Resource) -> Result<CloudInstance, String> {
    if resource.provider.as_deref() != Some("aws") {
        return Err("cloud_instance requires `provider: aws`".into());
    }
    require(
        matches!(resource.state.as_deref(), None | Some("present" | "absent")),
        "state must be present or absent",
    )?;
    let name = field(&resource.name, "name")?;
    require(
        is_ident(&name, 63),
        "name must be 1-63 characters of [A-Za-z0-9._-], starting alphanumeric",
    )?;
    let region = field(&resource.cloud.region, "cloud_region")?;
    let instance_type = field(&resource.cloud.instance_type, "cloud_instance_type")?;
    let max_price = field(&resource.cloud.max_price, "cloud_max_price")?;
    let image = field(&resource.image, "image")?;
    require(
        is_word(&region, "-") && region.starts_with(|c: char| c.is_ascii_alphanumeric()),
        "cloud_region has characters outside [a-z0-9-]",
    )?;
    require(
        is_word(&instance_type, ".-")
            && instance_type.starts_with(|c: char| c.is_ascii_alphanumeric()),
        "cloud_instance_type has unexpected characters",
    )?;
    require(
        is_positive_decimal(&max_price),
        "cloud_max_price must be a positive decimal",
    )?;
    require(
        has_digest(&image),
        "image must be pinned by @sha256:<64 hex digits>",
    )?;
    require(
        image_name(&image).is_some_and(is_image_name),
        "image name must be 1-255 characters of [A-Za-z0-9._/:-], starting alphanumeric",
    )?;
    let max_lifetime_min = resource.cloud.max_lifetime_min.filter(|m| *m > 0);
    let max_lifetime_min = max_lifetime_min.ok_or("cloud_max_lifetime_min must be set and > 0")?;
    for t in &resource.tags {
        require(
            is_tag(t),
            &format!(
                "tag '{t:.64}' must be key=value of [A-Za-z0-9_.:/-], at most 128 characters each"
            ),
        )?;
        require(
            !t.starts_with("forjar-key=") && !t.starts_with("forjar-owner="),
            "tags forjar-key and forjar-owner are reserved for ownership",
        )?;
    }
    Ok(CloudInstance {
        name,
        region,
        instance_type,
        max_price,
        image,
        max_lifetime_min,
        tags: resource.tags.clone(),
    })
}

fn field(v: &Option<String>, name: &str) -> Result<String, String> {
    match v.as_deref() {
        Some(s) if !s.is_empty() => Ok(s.to_string()),
        _ => Err(format!("cloud_instance requires `{name}`")),
    }
}

fn require(ok: bool, msg: &str) -> Result<(), String> {
    if ok {
        Ok(())
    } else {
        Err(msg.to_string())
    }
}

fn is_word(s: &str, extra: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || extra.contains(c))
}

fn is_ident(s: &str, max: usize) -> bool {
    s.len() <= max
        && s.starts_with(|c: char| c.is_ascii_alphanumeric())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
}

fn image_name(image: &str) -> Option<&str> {
    image.rsplit_once("@sha256:").map(|(n, _)| n)
}

fn is_image_name(n: &str) -> bool {
    n.len() <= 255
        && n.starts_with(|c: char| c.is_ascii_alphanumeric())
        && n.chars()
            .all(|c| c.is_ascii_alphanumeric() || "._/:-".contains(c))
}

fn is_positive_decimal(s: &str) -> bool {
    // digits, optionally `digits.digits`; short enough that it can never
    // overflow to infinity (inf > inf is false and would defeat the cap check).
    let (int, frac) = s.split_once('.').unwrap_or((s, "0"));
    let digits = |p: &str| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit());
    digits(int)
        && digits(frac)
        && s.len() <= 12
        && s.parse::<f64>().is_ok_and(|v| v.is_finite() && v > 0.0)
}

fn has_digest(image: &str) -> bool {
    image.rsplit_once("@sha256:").is_some_and(|(name, hex)| {
        !name.is_empty()
            && hex.len() == 64
            && hex.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'))
    })
}

fn is_tag(t: &str) -> bool {
    let plain = |s: &str| {
        !s.is_empty()
            && s.len() <= 128
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.:/".contains(c))
    };
    t.split_once('=').is_some_and(|(k, v)| plain(k) && plain(v))
}

/// Idempotency key (`name@region@digest12`): the resource's own name, its region and the first 12 hex
/// digits of the image digest. Two declarations get the same key only if they
/// share a name (refused by [`validate_unique_names`]).
fn key_of(s: &CloudInstance) -> String {
    let digest = s.image.rsplit_once("@sha256:").map_or("", |(_, d)| d);
    format!("{}@{}@{}", s.name, s.region, &digest[..12])
}

/// Ownership key for teardown: name plus region, stable across image changes.
/// `@` joins the parts: neither the name nor the region allow-list can contain
/// it, so two different (name, region) pairs never give the same string.
fn owner_of(s: &CloudInstance) -> String {
    format!("{}@{}", s.name, s.region)
}

/// Plan-time: two `cloud_instance` resources may not share a name and region,
/// since the name is what keeps their instances and their teardown apart.
/// Values still holding a `{{` template are skipped (checked once resolved).
pub fn validate_unique_names(config: &crate::core::types::ForjarConfig) -> Vec<String> {
    let mut seen: std::collections::HashMap<(String, String), &str> = Default::default();
    let mut errors = Vec::new();
    for (id, r) in &config.resources {
        let (Some(name), Some(region)) = (&r.name, &r.cloud.region) else {
            continue;
        };
        if r.resource_type != crate::core::types::ResourceType::CloudInstance
            || name.contains("{{")
            || region.contains("{{")
        {
            continue;
        }
        if let Some(first) = seen.insert((name.clone(), region.clone()), id) {
            errors.push(format!(
                "resource '{id}' (cloud_instance): name '{name}' in {region} is already used by '{first}'; \
                 each declared instance needs its own name"
            ));
        }
    }
    errors
}

/// The three apply gates a declaration must carry: (parity gate, budget cap, kill switch).
fn gates_of(r: &Resource) -> Result<(String, String, String), Vec<&'static str>> {
    let abs = |v: &Option<String>| {
        v.clone()
            .filter(|p| p.starts_with('/') && !p.chars().any(char::is_control))
    };
    let budget = r
        .cloud
        .budget_cap_usd_day
        .clone()
        .filter(|b| is_positive_decimal(b));
    let (gate, kill) = (abs(&r.cloud.parity_gate), abs(&r.cloud.kill_switch));
    match (gate, budget, kill) {
        (Some(g), Some(b), Some(k)) => Ok((g, b, k)),
        (g, b, k) => Err([
            ("parity_gate", g.is_none()),
            ("budget", b.is_none()),
            ("kill_switch", k.is_none()),
        ]
        .iter()
        .filter(|(_, missing)| *missing)
        .map(|(n, _)| *n)
        .collect()),
    }
}

fn status(v: &Option<String>) -> String {
    v.clone().unwrap_or_else(|| "MISSING".into())
}

/// Declared-vs-intended diff, rendered with no network call (pure Rust).
pub fn plan_diff(resource: &Resource) -> String {
    let s = match spec_of(resource) {
        Ok(s) => s,
        Err(e) => return format!("ERROR: {e}\n"),
    };
    let c = &resource.cloud;
    format!(
        "+ cloud_instance aws/{region} spot {itype}\n\
         +   max_price: {price} USD/h\n\
         +   image: {image}\n\
         +   max_lifetime_min: {life}\n\
         +   labels: {labels}\n\
         - destroy: after {life} min (self-terminate) or when the kill switch exists\n\
         gates (apply refuses unless all declared):\n\
         \x20 parity_gate: {gate}\n\
         \x20 budget_cap_usd_day: {budget}\n\
         \x20 kill_switch: {kill}\n",
        region = s.region,
        itype = s.instance_type,
        price = s.max_price,
        image = s.image,
        life = s.max_lifetime_min,
        labels = s.tags.join(","),
        gate = status(&c.parity_gate),
        budget = status(&c.budget_cap_usd_day),
        kill = status(&c.kill_switch),
    )
}

/// Single-quote `s` for a shell word WITHOUT dropping newlines. `sh_squote`
/// strips control characters, which welded user-data into one line
/// (`#!/bin/shshutdown -h +60...`): no shebang and no lifetime bound. Only
/// pass text built from validated values.
fn squote_keep_newlines(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\"'\"'"))
}

fn reject(msg: &str) -> String {
    format!("echo {} >&2; exit 1", sh_squote(&format!("REFUSED: {msg}")))
}

/// Read-only provider query printing `query` for live instances whose
/// `tag_key` tag equals `tag_val`.
fn describe_by(s: &CloudInstance, tag_key: &str, tag_val: &str, query: &str) -> String {
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
/// Every token must be a whole instance id (`i-` + 8 to 17 hex digits). Works
/// with or without `set -e`.
fn lookup_ids(var: &str, cmd: &str) -> String {
    format!(
        "{var}=\"$({cmd})\" || {{ echo 'instance-lookup-failed' >&2; echo 'ERROR: instance lookup failed (provider query exited non-zero)' >&2; exit 1; }}\n\
         for i in ${var}; do h=\"${{i#i-}}\"; case \"$i\" in i-*) ;; *) h='!';; esac; \
         case \"$h\" in *[!0-9a-f]*) h='!';; esac; \
         if [ \"${{#h}}\" -lt 8 ] || [ \"${{#h}}\" -gt 17 ] || [ \"$h\" = '!' ]; then \
         echo 'instance-lookup-malformed' >&2; echo 'ERROR: instance lookup returned something that is not an instance id' >&2; exit 1; fi; done\n"
    )
}

/// Instances of this exact declaration (key = name + region + image digest).
fn describe(s: &CloudInstance) -> String {
    describe_by(
        s,
        "forjar-key",
        &key_of(s),
        "Reservations[].Instances[].InstanceId",
    )
}

fn is_absent(resource: &Resource) -> bool {
    resource.state.as_deref() == Some("absent")
}

/// Script asking the provider whether the instance exists (read-only).
pub fn check_script(resource: &Resource) -> String {
    let s = match spec_of(resource) {
        Ok(s) => s,
        Err(e) => return reject(&e),
    };
    let key = key_of(&s);
    if is_absent(resource) {
        let owned = describe_by(
            &s,
            "forjar-owner",
            &owner_of(&s),
            "Reservations[].Instances[].InstanceId",
        );
        return format!(
            "{lookup}{check}",
            lookup = lookup_ids("ids", &owned),
            check = crate::resources::verdict::check_script_from(&[
                crate::resources::verdict::assert_that(
                    "[ -z \"$ids\" ]",
                    &format!("instance-absent:{key}"),
                    &format!("instance-still-owned:{key}"),
                ),
            ]),
        );
    }
    format!(
        "{lookup}{check}",
        lookup = lookup_ids("ids", &describe(&s)),
        check = crate::resources::verdict::check_script_from(&[
            crate::resources::verdict::assert_that(
                "[ -n \"$ids\" ]",
                &format!("instance-present:{key}"),
                &format!("instance-absent:{key}"),
            )
        ]),
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

/// `state: absent`: terminate ONLY the instances carrying this resource's own
/// `forjar-owner` tag, cancel their spot requests, and report what is true.
/// Never launches. Not gated: the kill switch exists to stop spend, and this
/// only ever removes owned instances (none owned = a no-op success).
fn destroy_script(s: &CloudInstance) -> String {
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
         sirs=\"$({sirs})\"\n\
         set -- $ids\n\
         aws ec2 terminate-instances --region {region} --instance-ids \"$@\" >/dev/null\n\
         for r in $sirs; do case \"$r\" in sir-[0-9a-z]*) ;; *) continue;; esac; \
         aws ec2 cancel-spot-instance-requests --region {region} --spot-instance-request-ids \"$r\" >/dev/null; done\n\
         left=\"$({ids})\"\n\
         if [ -n \"$left\" ]; then echo \"NOT destroyed: still pending or running: $left\" >&2; exit 1; fi\n\
         echo \"destroyed $ids\"\n",
    )
}

/// Script that creates the instance, or refuses. Refuses WITHOUT touching the
/// provider unless the parity gate says pass, the kill switch is absent, and a
/// budget cap is declared.
pub fn apply_script(resource: &Resource) -> String {
    let s = match spec_of(resource) {
        Ok(s) => s,
        Err(e) => return reject(&e),
    };
    if is_absent(resource) {
        return destroy_script(&s);
    }
    let (gate, budget, kill) = match gates_of(resource) {
        Ok(g) => g,
        Err(missing) => {
            return reject(&format!(
                "cloud_instance apply needs declared: {}",
                missing.join(", ")
            ))
        }
    };
    // The lifetime bound comes first: nothing derived from user input can run
    // before it, and every value is validated (`spec_of`) and quoted.
    let worst_case_usd =
        s.max_price.parse::<f64>().unwrap_or(f64::MAX) * f64::from(s.max_lifetime_min) / 60.0;
    if budget
        .parse::<f64>()
        .map_or(true, |cap| worst_case_usd > cap)
    {
        return reject(&format!(
            "worst-case spend {worst_case_usd:.4} USD (max_price x lifetime) exceeds cloud_budget_cap_usd_day {budget}"
        ));
    }
    let user_data = format!(
        "#!/bin/sh\nshutdown -h +{}\nexport FORJAR_IMAGE={}\n",
        s.max_lifetime_min,
        sh_squote(&s.image)
    );
    let mut tags = vec![
        format!("{{Key=forjar-key,Value={}}}", key_of(&s)),
        format!("{{Key=forjar-owner,Value={}}}", owner_of(&s)),
    ];
    tags.extend(
        s.tags
            .iter()
            .filter_map(|t| t.split_once('='))
            .map(|(k, v)| format!("{{Key={k},Value={v}}}")),
    );
    format!(
        "set -eu\n\
         if [ -e {kill} ] || [ -L {kill} ]; then echo 'REFUSED: kill switch is thrown' >&2; exit 1; fi\n\
         if ! {{ [ -f {gate} ] && grep -qx 'parity-gate: pass' {gate} && [ \"$(grep -c '^parity-gate:' {gate})\" = 1 ]; }}; then\n\
         \x20 echo 'REFUSED: parity gate has not passed' >&2; exit 1\n\
         fi\n\
         # budget cap declared: {budget_note}\n\
         {lookup_owned}\
         {lookup_keyed}\
         if [ -z \"$owned\" ]; then :; else\n\
         \x20 set -- $keyed; keyed=\" $* \"; other=''\n\
         \x20 for i in $owned; do case \"$keyed\" in *\" $i \"*) ;; *) other=\"$other $i\";; esac; done\n\
         \x20 if [ -n \"$other\" ]; then echo \"REFUSED: image changed while instance$other is alive; set state: absent first or wait for its lifetime shutdown\" >&2; exit 1; fi\n\
         \x20 echo \"present $owned\"; exit 0\n\
         fi\n\
         aws ec2 run-instances --region {region} --count 1 --instance-type {itype} \
         --image-id {ami} \
         --instance-market-options {market} \
         --instance-initiated-shutdown-behavior terminate \
         --metadata-options 'HttpTokens=required,HttpPutResponseHopLimit=1,HttpEndpoint=enabled' \
         --user-data {ud} \
         --tag-specifications {tagspec}\n",
        kill = sh_squote(&kill),
        gate = sh_squote(&gate),
        budget_note = budget.replace(|c: char| !(c.is_ascii_digit() || c == '.'), ""),
        lookup_owned = lookup_ids("owned", &describe_by(&s, "forjar-owner", &owner_of(&s), "Reservations[].Instances[].InstanceId")),
        lookup_keyed = lookup_ids("keyed", &describe(&s)),
        region = sh_squote(&s.region),
        itype = sh_squote(&s.instance_type),
        ami = sh_squote("resolve:ssm:/aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-x86_64"),
        market = sh_squote(&format!(
            "MarketType=spot,SpotOptions={{MaxPrice={},SpotInstanceType=one-time,InstanceInterruptionBehavior=terminate}}",
            s.max_price
        )),
        ud = squote_keep_newlines(&user_data),
        tagspec = sh_squote(&format!("ResourceType=instance,Tags=[{}]", tags.join(","))),
    )
}
