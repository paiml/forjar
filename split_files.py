#!/usr/bin/env python3
"""Split oversized Rust source files at blank-line boundaries.

For each file over 500 lines, creates _b.rs (and _c.rs, _d.rs) siblings
with duplicated import headers. Prints new module declarations for mod.rs.
"""

import os
import re
import sys
from pathlib import Path

MAX_LINES = 498  # Target max per file


def brace_depth_change(line):
    """Net brace depth change, skipping strings and line comments."""
    d = 0
    in_str = False
    i = 0
    while i < len(line):
        c = line[i]
        if in_str:
            if c == '\\':
                i += 1
            elif c == '"':
                in_str = False
        else:
            if c == '"':
                in_str = True
            elif c == '{':
                d += 1
            elif c == '}':
                d -= 1
            elif c == '/' and i + 1 < len(line) and line[i + 1] == '/':
                break
        i += 1
    return d


def detect_wrapper(lines):
    """Detect #[cfg(test)] mod tests { ... } wrapper in PURE test files.
    Only returns True if the test wrapper is near the start (within first 30 lines),
    indicating a pure test file. Mixed impl+test files are handled as impl files.
    Returns (has_wrapper, cfg_idx, mod_idx, use_super_idx, closing_idx).
    """
    for i, line in enumerate(lines):
        if '#[cfg(test)]' in line:
            # Only treat as wrapper if near the start (pure test file)
            if i > 30:
                return False, -1, -1, -1, -1
            for j in range(i + 1, min(i + 3, len(lines))):
                if re.search(r'mod\s+tests\s*\{', lines[j]):
                    use_idx = -1
                    for k in range(j + 1, min(j + 5, len(lines))):
                        if 'use super::*' in lines[k]:
                            use_idx = k
                            break
                    # Find matching close brace
                    depth = 0
                    close = len(lines) - 1
                    for m in range(j, len(lines)):
                        depth += brace_depth_change(lines[m])
                        if depth == 0 and m > j:
                            close = m
                            break
                    return True, i, j, use_idx, close
    return False, -1, -1, -1, -1


def find_split_candidates(lines, body_start, body_end, base_depth):
    """Return line indices that are safe split points between items.
    A split at candidate C means: original keeps lines[:C], new file gets lines[C:].
    Safe split points are:
    1. After a blank line at base depth (split AFTER the blank)
    2. After a closing brace that returns to base depth (between items)
    """
    candidates = []
    depth = base_depth

    for i in range(body_start, body_end):
        cur_depth = depth
        depth += brace_depth_change(lines[i])

        if cur_depth != base_depth:
            # Check if this line ENDS at base depth (closing brace)
            if depth == base_depth and lines[i].strip().endswith('}'):
                # The next line starts a new item — mark it as candidate
                if i + 1 < body_end:
                    candidates.append(i + 1)
            continue

        # At base depth: blank lines are candidates
        if lines[i].strip() == '':
            if i + 1 < body_end:
                candidates.append(i + 1)

    return sorted(set(candidates))


def find_header(lines, has_wrapper, cfg_idx, mod_idx, use_super_idx):
    """Identify header section to duplicate in new files.
    Returns (header_lines_list, body_start_idx, base_depth, footer_str).
    """
    if has_wrapper:
        inner_start = use_super_idx + 1 if use_super_idx >= 0 else mod_idx + 1
        outer = lines[:cfg_idx]
        inner = [lines[cfg_idx], lines[mod_idx]]
        if use_super_idx >= 0:
            inner.append(lines[use_super_idx])

        # Extend header to include all non-#[test] items (helpers, constants)
        # inside mod tests {} up to the first #[test] attribute.
        # This ensures helper functions are duplicated to all split files.
        first_test_idx = inner_start
        for k in range(inner_start, len(lines)):
            stripped = lines[k].strip()
            if stripped == '#[test]':
                first_test_idx = k
                break
        # Back up past any blank lines before #[test]
        while first_test_idx > inner_start and lines[first_test_idx - 1].strip() == '':
            first_test_idx -= 1

        # Include helper items in the header
        helpers = lines[inner_start:first_test_idx]
        header = outer + inner + helpers
        return header, first_test_idx, 1, '}\n'
    else:
        # Find start of first item
        for i, line in enumerate(lines):
            s = line.strip()
            if (re.match(r'(pub(\(crate\))?\s+)?fn\s+', s) or
                    re.match(r'(pub(\(crate\))?\s+)?struct\s+', s) or
                    re.match(r'(pub(\(crate\))?\s+)?const\s+[A-Z]', s) or
                    re.match(r'(pub(\(crate\))?\s+)?enum\s+', s) or
                    re.match(r'impl\b', s)):
                # Back up past attributes and doc comments
                start = i
                while start > 0:
                    prev = lines[start - 1].strip()
                    if prev.startswith('#[') or prev.startswith('///'):
                        start -= 1
                    else:
                        break
                return lines[:start], start, 0, ''
        return lines[:], len(lines), 0, ''


def split_file(filepath):
    """Split a single file. Returns list of new_filepath strings."""
    with open(filepath) as f:
        content = f.read()
    lines = content.split('\n')
    # Preserve trailing newline handling
    if content.endswith('\n') and lines and lines[-1] == '':
        lines = lines[:-1]

    total = len(lines)
    if total <= 500:
        return []

    has_wrapper, cfg_idx, mod_idx, use_super_idx, close_idx = detect_wrapper(lines)
    header, body_start, base_depth, footer = find_header(
        lines, has_wrapper, cfg_idx, mod_idx, use_super_idx
    )

    if has_wrapper:
        body_end = close_idx  # Exclude closing brace
    else:
        body_end = total

    header_overhead = len(header) + (1 if footer else 0)
    max_body_per_part = MAX_LINES - header_overhead

    if max_body_per_part <= 50:
        print(f"  WARNING: header too large ({header_overhead} lines), max_body={max_body_per_part}")
        max_body_per_part = MAX_LINES - 30

    candidates = find_split_candidates(lines, body_start, body_end, base_depth)
    if not candidates:
        print(f"  WARNING: no split candidates found for {filepath}")
        return []

    # Greedy forward splitting:
    # - Original file: lines[0 : sp1] + footer <= MAX_LINES
    # - File _b: header + lines[sp1 : sp2] + footer <= MAX_LINES
    # - File _c: header + lines[sp2 : sp3] + footer <= MAX_LINES
    # etc.

    footer_lines = 1 if footer else 0
    first_max_end = MAX_LINES - footer_lines  # First file includes original header

    split_points = []

    # Find split point for first file
    best = None
    for c in candidates:
        if c <= first_max_end:
            best = c
    if best is None:
        best = candidates[0]
    split_points.append(best)

    # Keep splitting until remaining chunk fits in one file
    cursor = best
    while True:
        remaining = body_end - cursor
        remaining_file_size = header_overhead + remaining
        if remaining_file_size <= MAX_LINES:
            break  # Last chunk fits

        max_end = cursor + max_body_per_part
        best = None
        for c in candidates:
            if c > cursor and c <= max_end:
                best = c
        if best is None:
            # No candidate in range — force split at nearest candidate after cursor
            for c in candidates:
                if c > cursor:
                    best = c
                    break
            if best is None:
                break  # No more candidates
        split_points.append(best)
        cursor = best

    if not split_points:
        print(f"  WARNING: could not find valid split points for {filepath}")
        return []

    # Now create the files
    new_files = []
    base = filepath.replace('.rs', '')
    all_suffixes = ['_b', '_c', '_d', '_e', '_f', '_g', '_h', '_i', '_j']

    # Find first available suffix (don't overwrite existing files)
    available_suffixes = []
    for s in all_suffixes:
        candidate = f'{base}{s}.rs'
        if not os.path.exists(candidate):
            available_suffixes.append(s)
    if not available_suffixes:
        print(f"  ERROR: no available suffix for {filepath}")
        return []

    # Write first part (modify original)
    first_end = split_points[0]
    first_lines = lines[:first_end]
    if footer:
        first_lines.append(footer.rstrip('\n'))
    with open(filepath, 'w') as f:
        f.write('\n'.join(first_lines) + '\n')

    # Write subsequent parts
    suffix_idx = 0
    for i in range(len(split_points)):
        chunk_start = split_points[i]
        chunk_end = split_points[i + 1] if i + 1 < len(split_points) else body_end

        if chunk_start >= body_end:
            break

        if suffix_idx >= len(available_suffixes):
            print(f"  ERROR: ran out of suffixes for {filepath}")
            break

        suffix = available_suffixes[suffix_idx]
        suffix_idx += 1
        new_path = f'{base}{suffix}.rs'

        content_lines = [h.rstrip('\n') for h in header]
        for j in range(chunk_start, min(chunk_end, body_end)):
            content_lines.append(lines[j])
        if footer:
            content_lines.append(footer.rstrip('\n'))

        with open(new_path, 'w') as f:
            f.write('\n'.join(content_lines) + '\n')
        new_files.append(new_path)

    return new_files


def main():
    # Find all oversized files
    oversized = []
    for root, dirs, files in os.walk('src'):
        for fname in files:
            if not fname.endswith('.rs'):
                continue
            fpath = os.path.join(root, fname)
            with open(fpath) as f:
                count = sum(1 for _ in f)
            if count > 500:
                if fname == 'apply_args.rs':
                    continue
                # Skip files that need manual refactoring
                skip = {'mod.rs'}
                if fname in skip:
                    continue
                oversized.append((count, fpath))

    oversized.sort(reverse=True)
    print(f"Found {len(oversized)} files over 500 lines")

    if '--list' in sys.argv:
        for count, path in oversized:
            print(f"  {count:5d}  {path}")
        return

    if '--dry-run' in sys.argv:
        for count, path in oversized:
            print(f"  Would split: {path} ({count} lines)")
        return

    # Allow filtering by batch
    batch = None
    for arg in sys.argv[1:]:
        if arg.startswith('--batch='):
            batch = int(arg.split('=')[1])

    if batch is not None:
        if batch == 1:
            oversized = [(c, p) for c, p in oversized if c > 800]
        elif batch == 2:
            oversized = [(c, p) for c, p in oversized if 700 <= c <= 857]
        elif batch == 3:
            oversized = [(c, p) for c, p in oversized if 600 <= c < 700]
        elif batch == 4:
            oversized = [(c, p) for c, p in oversized if 560 <= c < 600]
        elif batch == 5:
            oversized = [(c, p) for c, p in oversized if 520 <= c < 560]
        elif batch == 6:
            oversized = [(c, p) for c, p in oversized if 501 <= c < 520]

    # Process files
    all_new_files = []
    errors = []
    for count, path in oversized:
        print(f"\nSplitting {path} ({count} lines)...")
        try:
            new_files = split_file(path)
            for nf in new_files:
                nf_count = sum(1 for _ in open(nf))
                status = "OK" if nf_count <= 500 else f"OVER ({nf_count})"
                print(f"  Created: {nf} ({nf_count} lines) [{status}]")
            orig_count = sum(1 for _ in open(path))
            status = "OK" if orig_count <= 500 else f"OVER ({orig_count})"
            print(f"  Original now: {path} ({orig_count} lines) [{status}]")
            all_new_files.extend(new_files)
        except Exception as e:
            print(f"  ERROR: {e}")
            import traceback
            traceback.print_exc()
            errors.append((path, str(e)))

    # Print module declarations needed
    print("\n\n=== NEW MODULE DECLARATIONS ===")
    by_dir = {}
    for nf in sorted(all_new_files):
        parent = str(Path(nf).parent)
        mod_name = Path(nf).stem
        by_dir.setdefault(parent, []).append(mod_name)

    for d, mods in sorted(by_dir.items()):
        print(f"\n  [{d}]")
        for mod_name in sorted(mods):
            is_test = mod_name.startswith('tests_') or mod_name.startswith('test_')
            if is_test:
                print(f"    #[cfg(test)]")
                print(f"    mod {mod_name};")
            else:
                print(f"    mod {mod_name};")

    # Verify no new file is over 500 lines
    print("\n\n=== VERIFICATION ===")
    still_over = []
    for nf in all_new_files:
        if os.path.exists(nf):
            c = sum(1 for _ in open(nf))
            if c > 500:
                still_over.append((c, nf))
    for _, path in oversized:
        if os.path.exists(path):
            c = sum(1 for _ in open(path))
            if c > 500:
                still_over.append((c, path))

    if still_over:
        print(f"WARNING: {len(still_over)} files still over 500 lines:")
        for c, p in sorted(still_over, reverse=True):
            print(f"  {c:5d}  {p}")
    else:
        print("All split files under 500 lines!")

    if errors:
        print(f"\n{len(errors)} ERRORS occurred:")
        for p, e in errors:
            print(f"  {p}: {e}")


if __name__ == '__main__':
    main()
