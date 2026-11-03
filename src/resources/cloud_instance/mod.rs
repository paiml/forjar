//! FJ-861: `cloud_instance` — ONE declared spot instance, inert until gated.
//!
//! Nothing in this module calls a cloud. It *generates* shell; the shell runs
//! `aws` only when the machine it is applied on has it, and `apply_script`
//! refuses outright unless a parity-gate file, a budget cap and a kill switch
//! are all declared (and the gate file says pass, and the kill switch is
//! absent). `plan_diff` is pure Rust: no process, no network.

use crate::core::shell_escape::sh_squote;
use crate::core::types::Resource;

#[cfg(test)]
mod tests;

/// A validated `cloud_instance` declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudInstance {
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
/// image without an `@sha256:<64 hex>` digest, or has a non-positive price or
/// lifetime.
pub fn spec_of(resource: &Resource) -> Result<CloudInstance, String> {
    if resource.provider.as_deref() != Some("aws") {
        return Err("cloud_instance requires `provider: aws`".into());
    }
    let region = field(&resource.cloud.region, "cloud_region")?;
    let instance_type = field(&resource.cloud.instance_type, "cloud_instance_type")?;
    let max_price = field(&resource.cloud.max_price, "cloud_max_price")?;
    let image = field(&resource.image, "image")?;
    require(
        is_word(&region, "-"),
        "cloud_region has characters outside [a-z0-9-]",
    )?;
    require(
        is_word(&instance_type, ".-"),
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
    let max_lifetime_min = resource.cloud.max_lifetime_min.filter(|m| *m > 0);
    let max_lifetime_min = max_lifetime_min.ok_or("cloud_max_lifetime_min must be set and > 0")?;
    for t in &resource.tags {
        require(
            is_tag(t),
            &format!("tag '{t}' must be key=value of plain characters"),
        )?;
    }
    Ok(CloudInstance {
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

fn is_positive_decimal(s: &str) -> bool {
    let plain = !s.is_empty() && s.chars().all(|c| c.is_ascii_digit() || c == '.');
    plain && s.matches('.').count() <= 1 && s.parse::<f64>().is_ok_and(|v| v > 0.0)
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
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.:/".contains(c))
    };
    t.split_once('=').is_some_and(|(k, v)| plain(k) && plain(v))
}

/// Idempotency key: region plus the first 12 hex digits of the image digest.
fn key_of(s: &CloudInstance) -> String {
    let digest = s.image.rsplit_once("@sha256:").map_or("", |(_, d)| d);
    format!("{}-{}", s.region, &digest[..12])
}

/// The three apply gates a declaration must carry: (parity gate, budget cap, kill switch).
fn gates_of(r: &Resource) -> Result<(String, String, String), Vec<&'static str>> {
    let abs = |v: &Option<String>| v.clone().filter(|p| p.starts_with('/'));
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

fn reject(msg: &str) -> String {
    format!("echo {} >&2; exit 1", sh_squote(&format!("REFUSED: {msg}")))
}

/// Read-only provider query printing the ids of live instances with our key.
fn describe(s: &CloudInstance) -> String {
    format!(
        "aws ec2 describe-instances --region {region} \
         --filters {tag} 'Name=instance-state-name,Values=pending,running' \
         --query 'Reservations[].Instances[].InstanceId' --output text",
        region = sh_squote(&s.region),
        tag = sh_squote(&format!("Name=tag:forjar-key,Values={}", key_of(s))),
    )
}

/// Script asking the provider whether the instance exists (read-only).
pub fn check_script(resource: &Resource) -> String {
    let s = match spec_of(resource) {
        Ok(s) => s,
        Err(e) => return reject(&e),
    };
    let key = key_of(&s);
    crate::resources::verdict::check_script_from(&[crate::resources::verdict::assert_that(
        &format!("[ -n \"$({})\" ]", describe(&s)),
        &format!("instance-present:{key}"),
        &format!("instance-absent:{key}"),
    )])
}

/// Script reporting observed state (read-only provider query).
pub fn state_query_script(resource: &Resource) -> String {
    let s = match spec_of(resource) {
        Ok(s) => s,
        Err(e) => return reject(&e),
    };
    format!(
        "ids=\"$({})\"\nif [ -n \"$ids\" ]; then echo \"present $ids\"; else echo absent; fi\n",
        describe(&s)
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
    let (gate, budget, kill) = match gates_of(resource) {
        Ok(g) => g,
        Err(missing) => {
            return reject(&format!(
                "cloud_instance apply needs declared: {}",
                missing.join(", ")
            ))
        }
    };
    let user_data = format!(
        "#!/bin/sh\nexport FORJAR_IMAGE={}\nshutdown -h +{}\n",
        s.image, s.max_lifetime_min
    );
    let mut tags = vec![format!("{{Key=forjar-key,Value={}}}", key_of(&s))];
    tags.extend(
        s.tags
            .iter()
            .filter_map(|t| t.split_once('='))
            .map(|(k, v)| format!("{{Key={k},Value={v}}}")),
    );
    format!(
        "set -eu\n\
         if [ -e {kill} ]; then echo 'REFUSED: kill switch is thrown' >&2; exit 1; fi\n\
         if ! {{ [ -f {gate} ] && grep -qx 'parity-gate: pass' {gate}; }}; then\n\
         \x20 echo 'REFUSED: parity gate has not passed' >&2; exit 1\n\
         fi\n\
         # budget cap declared: {budget_note}\n\
         ids=\"$({describe})\"\n\
         if [ -n \"$ids\" ]; then echo \"present $ids\"; exit 0; fi\n\
         aws ec2 run-instances --region {region} --count 1 --instance-type {itype} \
         --image-id {ami} \
         --instance-market-options {market} \
         --instance-initiated-shutdown-behavior terminate \
         --user-data {ud} \
         --tag-specifications {tagspec}\n",
        kill = sh_squote(&kill),
        gate = sh_squote(&gate),
        budget_note = budget.replace(|c: char| !(c.is_ascii_digit() || c == '.'), ""),
        describe = describe(&s),
        region = sh_squote(&s.region),
        itype = sh_squote(&s.instance_type),
        ami = sh_squote("resolve:ssm:/aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-x86_64"),
        market = sh_squote(&format!(
            "MarketType=spot,SpotOptions={{MaxPrice={},SpotInstanceType=one-time,InstanceInterruptionBehavior=terminate}}",
            s.max_price
        )),
        ud = sh_squote(&user_data),
        tagspec = sh_squote(&format!("ResourceType=instance,Tags=[{}]", tags.join(","))),
    )
}
