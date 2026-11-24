//! Allow-list checks for declared values (every value reaches a boot script
//! or an `aws` command line).

/// Hard cap on instances per fleet (`cloud_count`, #1265).
use crate::core::types::Resource;

pub const MAX_FLEET_COUNT: u32 = 6;

pub(super) fn field(v: &Option<String>, name: &str) -> Result<String, String> {
    match v.as_deref() {
        Some(s) if !s.is_empty() => Ok(s.to_string()),
        _ => Err(format!("cloud_instance requires `{name}`")),
    }
}

pub(super) fn require(ok: bool, msg: &str) -> Result<(), String> {
    if ok {
        Ok(())
    } else {
        Err(msg.to_string())
    }
}

pub(super) fn is_instance_type(t: &str) -> bool {
    is_word(t, ".-") && t.starts_with(|c: char| c.is_ascii_alphanumeric())
}

pub(super) fn resolve_instance_types(
    single: &Option<String>,
    list: &Option<Vec<String>>,
) -> Result<Vec<String>, String> {
    match (single, list) {
        (Some(_), Some(_)) => {
            Err("cannot set both cloud_instance_type and cloud_instance_types".into())
        }
        (None, None) => {
            Err("cloud_instance requires `cloud_instance_type` or `cloud_instance_types`".into())
        }
        (Some(t), None) => {
            require(
                !t.is_empty(),
                "cloud_instance requires `cloud_instance_type`",
            )?;
            require(
                is_instance_type(t),
                "cloud_instance_type has unexpected characters",
            )?;
            Ok(vec![t.clone()])
        }
        (None, Some(ts)) => {
            require(!ts.is_empty(), "cloud_instance_types must not be empty")?;
            let uniq: std::collections::BTreeSet<&str> = ts.iter().map(String::as_str).collect();
            require(
                uniq.len() == ts.len(),
                "cloud_instance_types entries must be distinct",
            )?;
            for t in ts {
                require(
                    is_instance_type(t),
                    "cloud_instance_types has unexpected characters",
                )?;
            }
            Ok(ts.clone())
        }
    }
}

pub(super) fn check_fleet_caps(count: Option<u32>) -> Result<u32, crate::core::error::ForjarError> {
    let n = count.unwrap_or(1);
    if n == 0 || n > MAX_FLEET_COUNT {
        return Err(crate::core::error::ForjarError::from_untyped(format!(
            "cloud_count {n} must be 1 to {MAX_FLEET_COUNT} (hard cap {MAX_FLEET_COUNT})"
        )));
    }
    Ok(n)
}

pub(super) fn is_word(s: &str, extra: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || extra.contains(c))
}

pub(super) fn is_ident(s: &str, max: usize) -> bool {
    s.len() <= max
        && s.starts_with(|c: char| c.is_ascii_alphanumeric())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
}

pub(super) fn image_name(image: &str) -> Option<&str> {
    image.rsplit_once("@sha256:").map(|(n, _)| n)
}

pub(super) fn is_image_name(n: &str) -> bool {
    n.len() <= 255
        && n.starts_with(|c: char| c.is_ascii_alphanumeric())
        && n.chars()
            .all(|c| c.is_ascii_alphanumeric() || "._/:-".contains(c))
}

pub(super) fn is_positive_decimal(s: &str) -> bool {
    // digits, optionally `digits.digits`; short enough that it can never
    // overflow to infinity (inf > inf is false and would defeat the cap check).
    let (int, frac) = s.split_once('.').unwrap_or((s, "0"));
    let digits = |p: &str| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit());
    digits(int)
        && digits(frac)
        && s.len() <= 12
        && s.parse::<f64>().is_ok_and(|v| v.is_finite() && v > 0.0)
}

pub(super) fn has_digest(image: &str) -> bool {
    image.rsplit_once("@sha256:").is_some_and(|(name, hex)| {
        !name.is_empty()
            && hex.len() == 64
            && hex.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'))
    })
}

pub(super) fn is_tag(t: &str) -> bool {
    let plain = |s: &str| {
        !s.is_empty()
            && s.len() <= 128
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.:/".contains(c))
    };
    t.split_once('=').is_some_and(|(k, v)| plain(k) && plain(v))
}

/// The three apply gates a declaration must carry: (parity gate, budget cap, kill switch).
pub(super) fn gates_of(r: &Resource) -> Result<(String, String, String), Vec<&'static str>> {
    let abs = |v: &Option<String>| {
        v.clone()
            .filter(|p| p.starts_with('/') && !p.chars().any(char::is_control))
            // an unresolved template is not a path: a kill switch at
            // `/run/{{x}}` could never be thrown, so refuse it (fail closed)
            .filter(|p| !p.contains("{{") && !p.contains("}}"))
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
