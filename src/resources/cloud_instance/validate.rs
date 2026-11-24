//! Allow-list checks for declared values (every value reaches a boot script
//! or an `aws` command line).

/// Hard cap on instances per fleet (`cloud_count`, #1265).
pub const MAX_FLEET_COUNT: u32 = 6;

/// Plan-time hard cap on total USD/h summed over the fleet (`cloud_max_price * cloud_count`, #1265).
pub const MAX_FLEET_USD_PER_HOUR: f64 = 50.0;

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

pub(super) fn check_market_type(v: &Option<String>) -> Result<(), String> {
    match v.as_deref() {
        None | Some("spot") => Ok(()),
        Some(other) => Err(format!(
            "cloud_market_type '{other}' is refused; cloud_instance is spot-only (on-demand refused)"
        )),
    }
}

pub(super) fn check_fleet_caps(count: Option<u32>, max_price: &str) -> Result<u32, String> {
    let n = count.unwrap_or(1);
    if n == 0 || n > MAX_FLEET_COUNT {
        return Err(format!(
            "cloud_count {n} must be 1 to {MAX_FLEET_COUNT} (hard cap {MAX_FLEET_COUNT})"
        ));
    }
    let price = max_price.parse::<f64>().unwrap_or(f64::MAX);
    let total = price * f64::from(n);
    if !total.is_finite() || total > MAX_FLEET_USD_PER_HOUR {
        return Err(format!(
            "fleet max spend {total:.2} USD/h (cloud_max_price x cloud_count) exceeds hard cap {MAX_FLEET_USD_PER_HOUR:.2} USD/h"
        ));
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
