//! PMAT-073: Char-boundary-safe string truncation helpers.
//!
//! Slicing a `&str` at a fixed byte index (`&s[..n]`) panics when `n`
//! falls inside a multibyte UTF-8 sequence. These helpers back off to
//! the nearest char boundary, so they are safe on arbitrary input.

/// Longest prefix of `s` that is at most `max` bytes, ending on a char boundary.
pub fn truncate_at_boundary(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Truncate `s` to at most `max` bytes, appending `...` when truncated.
///
/// For `max < 4` there is no room for the ellipsis, so the first `max`
/// chars are returned instead.
pub fn truncate_with_ellipsis(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    if max < 4 {
        return s.chars().take(max).collect();
    }
    format!("{}...", truncate_at_boundary(s, max - 3))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boundary_ascii_short_passthrough() {
        assert_eq!(truncate_at_boundary("abc", 10), "abc");
        assert_eq!(truncate_at_boundary("abc", 3), "abc");
    }

    #[test]
    fn boundary_ascii_truncates() {
        assert_eq!(truncate_at_boundary("abcdef", 4), "abcd");
    }

    #[test]
    fn boundary_empty_and_zero() {
        assert_eq!(truncate_at_boundary("", 5), "");
        assert_eq!(truncate_at_boundary("abc", 0), "");
    }

    #[test]
    fn boundary_cyrillic_backs_off() {
        // "sshpass -p " is 11 bytes; 'п' occupies bytes 11-12, so a cut at
        // 12 lands mid-char and must back off to 11.
        let s = "sshpass -p пароль";
        assert_eq!(truncate_at_boundary(s, 12), "sshpass -p ");
    }

    #[test]
    fn boundary_emoji_backs_off() {
        // 🎉 is 4 bytes at offsets 2..6
        let s = "ab🎉cd";
        assert_eq!(truncate_at_boundary(s, 3), "ab");
        assert_eq!(truncate_at_boundary(s, 5), "ab");
        assert_eq!(truncate_at_boundary(s, 6), "ab🎉");
    }

    #[test]
    fn boundary_cjk_backs_off() {
        // Each CJK char is 3 bytes
        let s = "你好世界";
        assert_eq!(truncate_at_boundary(s, 4), "你");
        assert_eq!(truncate_at_boundary(s, 6), "你好");
        assert_eq!(truncate_at_boundary(s, 12), "你好世界");
    }

    #[test]
    fn ellipsis_short_passthrough() {
        assert_eq!(truncate_with_ellipsis("abc", 10), "abc");
        assert_eq!(truncate_with_ellipsis("abcd", 4), "abcd");
    }

    #[test]
    fn ellipsis_ascii_truncates() {
        assert_eq!(truncate_with_ellipsis("abcdefgh", 7), "abcd...");
    }

    #[test]
    fn ellipsis_tiny_max_takes_chars() {
        assert_eq!(truncate_with_ellipsis("abcdef", 3), "abc");
        assert_eq!(truncate_with_ellipsis("你好世界", 2), "你好");
    }

    #[test]
    fn ellipsis_multibyte_backs_off() {
        // max 7 → 4-byte budget; 'д' spans bytes 3..5 so cut backs off to 3
        assert_eq!(truncate_with_ellipsis("abcдефг", 7), "abc...");
        assert_eq!(truncate_with_ellipsis("ab🎉cdef", 8), "ab...");
    }
}
