//! Reordering the entries of a block mapping without reformatting it.
//!
//! `lint --fix` sorted `resources:` by rebuilding the mapping in
//! `serde_yaml_ng` and re-emitting the whole document (paiml/forjar#359). The
//! sort was cosmetic; the cost was every comment in the file.
//!
//! A sort does not need a re-serialisation. Each entry of a block mapping owns
//! a contiguous run of source lines, so sorting is a PERMUTATION OF BYTE
//! RANGES: no byte is rewritten, only moved. A comment line immediately above
//! an entry travels with it, which is what the comment meant.

use super::{
    child_range, first_key_indent, is_comment, key_at, locate_key, scan_lines, value_of,
    AnchorError, Line, TRANSPARENT,
};

/// One entry of a block mapping and the source bytes that belong to it.
///
/// Blocks are contiguous and in source order: `blocks[i].end == blocks[i+1].start`.
/// That is what makes [`reorder`] a permutation rather than a rewrite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyBlock {
    /// The entry's key.
    pub key: String,
    /// Byte offset of the block's first byte.
    pub start: usize,
    /// Byte offset one past the block's last byte.
    pub end: usize,
}

/// Partition the block mapping at `path` into one [`KeyBlock`] per entry.
pub fn key_blocks(text: &str, path: &[&str]) -> Result<Vec<KeyBlock>, AnchorError> {
    let lines = scan_lines(text);
    let at = locate_key(&lines, path)?;
    let (_, val) = value_of(&lines[at.line], at.key_end);
    if !val.is_empty() {
        return Err(super::classify_inline(val));
    }
    let (lo, raw_hi) = child_range(&lines, at.line, at.indent, at.range_end);
    let hi = trim_trailing_transparent(&lines, lo, raw_hi);
    if hi <= lo {
        return Err(AnchorError::NotAMapping);
    }
    if !text[..lines[hi - 1].next].ends_with('\n') {
        return Err(AnchorError::Unterminated);
    }
    let indent = first_key_indent(&lines, lo, hi).ok_or(AnchorError::NotAMapping)?;
    partition(&lines, lo, hi, indent)
}

/// Drop trailing blank and comment lines from a mapping's range.
///
/// A comment sitting between the last entry and the NEXT top-level key is
/// about that next key, not about the entry above it, so it must not be
/// carried along when the entry moves.
fn trim_trailing_transparent(lines: &[Line<'_>], lo: usize, hi: usize) -> usize {
    let mut end = hi;
    while end > lo && lines[end - 1].indent == TRANSPARENT {
        end -= 1;
    }
    end
}

fn partition(
    lines: &[Line<'_>],
    lo: usize,
    hi: usize,
    indent: usize,
) -> Result<Vec<KeyBlock>, AnchorError> {
    let mut keys: Vec<(usize, String)> = Vec::new();
    for (j, line) in lines.iter().enumerate().take(hi).skip(lo) {
        if let Some((k, _)) = key_at(line, indent) {
            if keys.iter().any(|(_, seen)| seen == &k) {
                return Err(AnchorError::Duplicate);
            }
            keys.push((j, k));
        }
    }
    if keys.is_empty() {
        return Err(AnchorError::NotAMapping);
    }
    Ok(build_blocks(lines, lo, hi, &keys))
}

/// Turn key line indices into contiguous byte ranges covering the whole region.
fn build_blocks(
    lines: &[Line<'_>],
    lo: usize,
    hi: usize,
    keys: &[(usize, String)],
) -> Vec<KeyBlock> {
    let mut starts: Vec<usize> = Vec::with_capacity(keys.len());
    for (n, (idx, _)) in keys.iter().enumerate() {
        let floor = if n == 0 { lo } else { keys[n - 1].0 + 1 };
        starts.push(if n == 0 {
            lo
        } else {
            comment_run_start(lines, *idx, floor)
        });
    }
    keys.iter()
        .enumerate()
        .map(|(n, (_, key))| KeyBlock {
            key: key.clone(),
            start: lines[starts[n]].start,
            end: match starts.get(n + 1) {
                Some(&next) => lines[next].start,
                None => lines[hi - 1].next,
            },
        })
        .collect()
}

/// The first line of the contiguous comment run immediately above `idx`.
fn comment_run_start(lines: &[Line<'_>], idx: usize, floor: usize) -> usize {
    let mut start = idx;
    while start > floor && is_comment(&lines[start - 1]) {
        start -= 1;
    }
    start
}

/// Whether the blocks are already in ascending key order.
pub fn is_sorted(blocks: &[KeyBlock]) -> bool {
    blocks.windows(2).all(|w| w[0].key <= w[1].key)
}

/// Emit `text` with `blocks` written in `order`.
///
/// `order` is a permutation of `0..blocks.len()`; anything else is refused,
/// because a non-permutation would drop or duplicate source bytes.
pub fn reorder(text: &str, blocks: &[KeyBlock], order: &[usize]) -> Result<String, AnchorError> {
    if !is_permutation(blocks.len(), order) {
        return Err(AnchorError::NotAMapping);
    }
    let (Some(first), Some(last)) = (blocks.first(), blocks.last()) else {
        return Err(AnchorError::NotAMapping);
    };
    let mut out = String::with_capacity(text.len());
    out.push_str(&text[..first.start]);
    for &i in order {
        out.push_str(&text[blocks[i].start..blocks[i].end]);
    }
    out.push_str(&text[last.end..]);
    Ok(out)
}

fn is_permutation(len: usize, order: &[usize]) -> bool {
    if order.len() != len {
        return false;
    }
    let mut seen = vec![false; len];
    for &i in order {
        match seen.get_mut(i) {
            Some(slot) if !*slot => *slot = true,
            _ => return false,
        }
    }
    true
}

/// The permutation that puts `blocks` in ascending key order, stably.
pub fn sorted_order(blocks: &[KeyBlock]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..blocks.len()).collect();
    order.sort_by(|&a, &b| blocks[a].key.cmp(&blocks[b].key));
    order
}
