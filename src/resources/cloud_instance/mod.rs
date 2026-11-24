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
//! - Ownership is `forjar-owner = <name>@<region>`. Two config files (or
//!   stacks) in one account that declare the same `name` in the same region
//!   share it: one sees the other's box as its own, and its `state: absent`
//!   terminates it. Uniqueness is enforced within one config only. Keep names
//!   distinct across configs.
//! - The instance key includes the image digest and, when declared, a hash of the
//!   placement (subnet, security groups, zone). Changing `image:` while the
//!   old box is alive is refused (never a second launch); the old box lives
//!   until its lifetime ends or `state: absent`. The spend cap is per launch,
//!   not summed over a day.

use crate::core::shell_escape::sh_squote;
use crate::core::types::Resource;

mod destroy;
use destroy::destroy_script;
mod ami;
mod key;
mod launch_io;
mod query;
pub use query::state_query_script;
use query::{describe, describe_by, lookup_ids, reject, squote_keep_newlines};
use validate::gates_of;
mod placement;
mod profile;
mod storage;
mod userdata;
pub use profile::misplaced as profile_misplaced;
mod validate;
use key::{key_of, owner_of};
pub use launch_io::apply_script;
pub use validate::MAX_FLEET_COUNT;
use validate::{
    check_fleet_caps, field, has_digest, image_name, is_ident, is_image_name, is_positive_decimal,
    is_tag, is_word, require, resolve_instance_types,
};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_b1;
#[cfg(test)]
mod tests_b4;
#[cfg(test)]
mod tests_chain;
#[cfg(test)]
mod tests_fleet;
#[cfg(test)]
mod tests_launch;
#[cfg(test)]
mod tests_lint;
#[cfg(test)]
mod tests_net;
#[cfg(test)]
mod tests_resolve;
#[cfg(test)]
mod tests_storage;

/// A validated `cloud_instance` declaration (single spot instance or spot fleet).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudInstance {
    /// The resource's own name (`name:`), unique per declared resource.
    pub name: String,
    pub region: String,
    /// Candidate instance types in fallback preference order (`len >= 1`).
    pub instance_types: Vec<String>,
    /// Fleet instance count (`1..=MAX_FLEET_COUNT`).
    pub count: u32,
    pub max_price: String,
    pub image: String,
    /// Pinned AMI id (`cloud_ami`); `None` = the default image.
    pub ami: Option<String>,
    pub max_lifetime_min: u32,
    pub tags: Vec<String>,
    pub placement: placement::Placement,
    pub storage: storage::Storage,
    /// `cloud_user_data_file`: local file passed by path as user-data; `None` = built-in script.
    pub user_data_file: Option<String>,
    /// IAM instance profile name (`cloud_instance_profile`); `None` = none attached.
    pub profile: Option<String>,
}

impl CloudInstance {
    /// Worst-case hourly ceiling summed over the fleet (`max_price * count`).
    #[must_use]
    pub fn total_max_price_usd_hr(&self) -> f64 {
        self.max_price.parse::<f64>().unwrap_or(f64::MAX) * f64::from(self.count)
    }

    /// Ordered `(instance_type, availability_zone)` launch candidates.
    pub(super) fn candidates(&self) -> Vec<(&str, Option<&str>)> {
        let zones = self.placement.zones();
        let mut out = Vec::new();
        for itype in &self.instance_types {
            if zones.is_empty() {
                out.push((itype.as_str(), None));
            } else {
                for z in zones {
                    out.push((itype.as_str(), Some(z.as_str())));
                }
            }
        }
        out
    }
}

/// Resolve a validated [`CloudInstance`] from a resource declaration.
///
/// # Errors
///
/// Returns `Err` when the declaration is not aws, exceeds the fleet count hard
/// cap, lacks a field, names an image without
/// an `@sha256:<64 hex>` digest, has a non-positive price or lifetime, or
/// carries a value outside the strict allow-list.
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
    require(
        is_word(&region, "-") && region.starts_with(|c: char| c.is_ascii_alphanumeric()),
        "cloud_region has characters outside [a-z0-9-]",
    )?;
    let instance_types = resolve_instance_types(
        &resource.cloud.instance_type,
        &resource.cloud.instance_types,
    )?;
    let max_price = field(&resource.cloud.max_price, "cloud_max_price")?;
    require(
        is_positive_decimal(&max_price),
        "cloud_max_price must be a positive decimal",
    )?;
    let count = check_fleet_caps(resource.cloud.count)
        .map_err(crate::core::error::ForjarError::into_message)?;
    let image = field(&resource.image, "image")?;
    require(
        has_digest(&image),
        "image must be pinned by @sha256:<64 hex digits>",
    )?;
    require(
        image_name(&image).is_some_and(is_image_name),
        "image name must be 1-255 characters of [A-Za-z0-9._/:-], starting alphanumeric",
    )?;
    let ami = ami::checked(&resource.cloud.ami)?;
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
    let placement = placement::Placement::of(&resource.cloud, &region)?;
    let storage = storage::Storage::of(&resource.cloud, &resource.tags)?;
    let user_data_file = userdata::checked(&resource.cloud.user_data_file)?;
    let profile = profile::checked(&resource.cloud.instance_profile)?;
    Ok(CloudInstance {
        name,
        region,
        instance_types,
        count,
        max_price,
        image,
        ami,
        max_lifetime_min,
        tags: resource.tags.clone(),
        placement,
        storage,
        user_data_file,
        profile,
    })
}

/// Plan-time: two `cloud_instance` resources may not share a name and region,
/// since the name is what keeps their instances and their teardown apart.
/// Values still holding a `{{` template are skipped (checked once resolved).
pub fn validate_unique_names(config: &crate::core::types::ForjarConfig) -> Vec<String> {
    unique_names(config.resources.iter())
}

/// Apply-time: the same check over the RESOLVED resources, so a `{{params.*}}`
/// name or region that collides once filled in is refused too.
pub fn validate_unique_names_resolved(config: &crate::core::types::ForjarConfig) -> Vec<String> {
    let resolved = crate::core::resolver::resolve_all(
        &config.resources,
        &config.params,
        &config.machines,
        &config.secrets,
    );
    unique_names(resolved.iter())
}

fn unique_names<'a>(resources: impl Iterator<Item = (&'a String, &'a Resource)>) -> Vec<String> {
    let mut seen: std::collections::HashMap<(String, String), &str> = Default::default();
    let mut errors = Vec::new();
    for (id, r) in resources {
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
    let zones = if s.placement.zones().is_empty() {
        String::new()
    } else {
        format!(
            "+   availability_zones: {}\n",
            s.placement.zones().join(",")
        )
    };
    format!(
        "+ cloud_instance aws/{region} spot {itype} x{count}\n\
         +   max_price: {price} USD/h\n\
         {zones}\
         +   image: {image}\n\
         +   max_lifetime_min: {life}\n\
         +   labels: {labels}\n\
         {ud}\
         - destroy: after {life} min (self-terminate) or when the kill switch exists\n\
         gates (apply refuses unless all declared):\n\
         \x20 parity_gate: {gate}\n\
         \x20 budget_cap_usd_day: {budget}\n\
         \x20 kill_switch: {kill}\n",
        region = s.region,
        itype = s.instance_types.join(","),
        count = s.count,
        price = s.max_price,
        image = s.image,
        life = s.max_lifetime_min,
        labels = s.tags.join(","),
        ud = s.user_data_file.as_deref().map_or_else(String::new, |p| {
            format!(
                "+   user_data_file: {p} (replaces the built-in script; checked: line 1 is #!, line 2 is '{}'; later commands could cancel it; image is not delivered)\n",
                userdata::required_line(s.max_lifetime_min)
            )
        }),
        gate = status(&c.parity_gate),
        budget = status(&c.budget_cap_usd_day),
        kill = status(&c.kill_switch),
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
    let (cond, count_prep) = if s.count == 1 {
        ("[ -n \"$ids\" ]".to_string(), String::new())
    } else {
        (
            format!("[ \"$have\" -ge {} ]", s.count),
            "have=0; for _i in $ids; do have=$((have+1)); done\n".to_string(),
        )
    };
    let mut checks = vec![crate::resources::verdict::assert_that(
        &cond,
        &format!("instance-present:{key}"),
        &format!("instance-absent:{key}"),
    )];
    checks.extend(s.storage.tag_check(&s.region, &s.tags, &key));
    let verdict = crate::resources::verdict::check_script_from(&checks);
    format!("{}{count_prep}{verdict}", lookup_ids("ids", &describe(&s)))
}

/// Script that creates the instance, or refuses. Refuses WITHOUT touching the
/// provider unless the parity gate says pass, the kill switch is absent, and a
/// budget cap is declared. The client token is for UTC day `day`.
pub(crate) fn apply_script_on(resource: &Resource, day: u64) -> String {
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
                "cloud_instance apply needs declared (and fully resolved): {}",
                missing.join(", ")
            ))
        }
    };
    // The lifetime bound (a declared user-data file must carry it too: see userdata.rs)
    // comes first: nothing derived from user input can run
    // before it, and every value is validated (`spec_of`) and quoted.
    let worst_case_usd = s.total_max_price_usd_hr() * f64::from(s.max_lifetime_min) / 60.0;
    if budget
        .parse::<f64>()
        .map_or(true, |cap| worst_case_usd > cap)
    {
        return reject(&format!(
            "worst-case spend {worst_case_usd:.4} USD (max_price x count x lifetime) exceeds cloud_budget_cap_usd_day {budget}"
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
    if !s.tags.iter().any(|t| t.starts_with("Owner=")) {
        tags.push(format!("{{Key=Owner,Value={}}}", owner_of(&s)));
    }
    tags.extend(
        s.tags
            .iter()
            .filter_map(|t| t.split_once('='))
            .map(|(k, v)| format!("{{Key={k},Value={v}}}")),
    );
    let cands = s.candidates();
    let is_fleet = s.count > 1 || cands.len() > 1;
    let (owned_block, itype_arg, net_args, run_tail) = if !is_fleet {
        (
            "if [ -z \"$owned\" ]; then :; else\n\
             \x20 set -- $keyed; keyed=\" $* \"; other=''\n\
             \x20 for i in $owned; do case \"$keyed\" in *\" $i \"*) ;; *) other=\"$other $i\";; esac; done\n\
             \x20 if [ -n \"$other\" ]; then echo \"REFUSED: image, placement or storage changed while instance$other is alive; set state: absent first or wait for its lifetime shutdown\" >&2; exit 1; fi\n\
             \x20 echo \"present $owned\"; exit 0\n\
             fi\n".to_string(),
            sh_squote(&s.instance_types[0]),
            s.placement.args(),
            launch_io::tail_on(&s, day),
        )
    } else {
        let zone_flag = if s.placement.zones().is_empty() {
            String::new()
        } else {
            "--placement \"AvailabilityZone=$az\" ".to_string()
        };
        (
            format!(
                "have=0\n\
                 if [ -z \"$owned\" ]; then :; else\n\
                 \x20 set -- $keyed; keyed=\" $* \"; other=''\n\
                 \x20 for i in $owned; do have=$((have+1)); case \"$keyed\" in *\" $i \"*) ;; *) other=\"$other $i\";; esac; done\n\
                 \x20 if [ -n \"$other\" ]; then echo \"REFUSED: image, placement or storage changed while instance$other is alive; set state: absent first or wait for its lifetime shutdown\" >&2; exit 1; fi\n\
                 \x20 if [ \"$have\" -ge {count} ]; then echo \"present $owned\"; exit 0; fi\n\
                 fi\n",
                count = s.count
            ),
            "\"$itype\"".to_string(),
            format!("{}{}", s.placement.net_args(), zone_flag),
            launch_io::fleet_tail_on(&s, &cands, day),
        )
    };
    format!(
        "set -eu\n\
         if [ -e {kill} ] || [ -L {kill} ]; then echo 'REFUSED: kill switch is thrown' >&2; exit 1; fi\n\
         if ! {{ [ -f {gate} ] && grep -qx 'parity-gate: pass' {gate} && [ \"$(grep -c '^parity-gate:' {gate})\" = 1 ]; }}; then\n\
         \x20 echo 'REFUSED: parity gate has not passed' >&2; exit 1\n\
         fi\n\
         # budget cap declared: {budget_note}\n\
         {ud_guard}\
         {lookup_owned}\
         {lookup_keyed}\
         {owned_block}\
         {rootdev}\
         {run_open}aws ec2 run-instances --region {region} --count 1 --instance-type {itype} \
         --image-id {ami} \
         --instance-market-options {market} \
         --instance-initiated-shutdown-behavior terminate \
         --metadata-options 'HttpTokens=required,HttpPutResponseHopLimit=1,HttpEndpoint=enabled' \
         {net}{store}{prof}\
         --user-data {ud} \
         --tag-specifications {tagspec}{voltags}{run_tail}",
        kill = sh_squote(&kill),
        gate = sh_squote(&gate),
        budget_note = budget.replace(|c: char| !(c.is_ascii_digit() || c == '.'), ""),
        lookup_owned = lookup_ids("owned", &describe_by(&s, "forjar-owner", &owner_of(&s), "Reservations[].Instances[].InstanceId")),
        lookup_keyed = lookup_ids("keyed", &describe(&s)),
        region = sh_squote(&s.region),
        itype = itype_arg,
        rootdev = s.storage.root_lookup(&sh_squote(&s.region), &s.ami),
        ami = s.storage.image_arg(&s.ami),
        market = sh_squote(&format!(
            "MarketType=spot,SpotOptions={{MaxPrice={},SpotInstanceType=one-time,InstanceInterruptionBehavior=terminate}}",
            s.max_price
        )),
        net = net_args,
        run_open = launch_io::OPEN,
        run_tail = run_tail,
        store = s.storage.args(),
        ud_guard = userdata::guard(s.user_data_file.as_deref(), s.max_lifetime_min),
        ud = s.user_data_file.as_deref().map_or_else(|| squote_keep_newlines(&user_data), userdata::file_arg),
        prof = profile::args(&s.profile),
        tagspec = sh_squote(&format!("ResourceType=instance,Tags=[{}]", tags.join(","))),
        voltags = s.storage.volume_tagspec(&s.tags),
    )
}
