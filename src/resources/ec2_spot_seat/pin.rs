//! The image pin: `image<TAB>key<TAB>value` rows of the single fleet-image file.

use crate::core::error::ForjarError;

/// The three values the resource takes from the pin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pin {
    pub id: String,
    pub s3_key: String,
    pub sha256: String,
}

fn row<'a>(text: &'a str, key: &str) -> Result<&'a str, ForjarError> {
    let mut hits = text.lines().filter_map(|l| {
        let mut f = l.split('\t');
        (f.next() == Some("image") && f.next() == Some(key)).then(|| f.next().unwrap_or(""))
    });
    let v = hits.next().filter(|v| !v.is_empty());
    match (v, hits.next()) {
        (Some(_), Some(_)) => Err(ForjarError::validation(format!(
            "pin key `{key}` appears twice"
        ))),
        (Some(v), None) => Ok(v),
        (None, _) => Err(ForjarError::validation(format!(
            "pin key `{key}` is missing"
        ))),
    }
}

fn shell_safe(v: &str) -> bool {
    !v.is_empty()
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '/' | '.' | '-' | '_'))
}

/// Parse the pin text with the declared key names.
///
/// # Errors
///
/// Returns `Err` when a key is missing, duplicated, or its value is not a plain
/// token (id `sha256:` + 64 hex, sha256 64 hex).
pub fn parse_pin(
    text: &str,
    id_key: &str,
    s3_key_key: &str,
    sha_key: &str,
) -> Result<Pin, ForjarError> {
    let id = row(text, id_key)?;
    let s3_key = row(text, s3_key_key)?;
    let sha256 = row(text, sha_key)?;
    let hex = |h: &str| h.len() == 64 && h.chars().all(|c| c.is_ascii_hexdigit());
    if !id.strip_prefix("sha256:").is_some_and(hex) {
        return Err(ForjarError::validation(format!(
            "pin `{id_key}` is not a sha256:<64 hex> image id"
        )));
    }
    if !hex(sha256) {
        return Err(ForjarError::validation(format!(
            "pin `{sha_key}` is not 64 hex digits"
        )));
    }
    if !shell_safe(s3_key) {
        return Err(ForjarError::validation(format!(
            "pin `{s3_key_key}` holds an unsafe value"
        )));
    }
    Ok(Pin {
        id: id.into(),
        s3_key: s3_key.into(),
        sha256: sha256.into(),
    })
}
