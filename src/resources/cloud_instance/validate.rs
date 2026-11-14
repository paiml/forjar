//! Allow-list checks for declared values (every value reaches a boot script
//! or an `aws` command line).

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
