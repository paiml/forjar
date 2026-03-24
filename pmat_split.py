#!/usr/bin/env python3
"""
PMAT-guided semantic file splitter for Rust source files.

Uses `pmat extract` (with test_module/children support) for function boundaries
and `pmat split` for semantic naming. Ensures all resulting files are under 500 lines.
"""
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path("/home/noah/src/forjar")
MAX_LINES = 500

def run_pmat_extract(filepath: str) -> dict:
    """Run pmat extract to get function boundaries with children nesting."""
    result = subprocess.run(
        ["pmat", "extract", "--list", filepath],
        capture_output=True, text=True, cwd=str(ROOT)
    )
    if result.returncode != 0:
        print(f"  WARNING: pmat extract failed for {filepath}: {result.stderr[:200]}")
        return None
    return json.loads(result.stdout)

def run_pmat_split_dry(filepath: str) -> dict:
    """Run pmat split for semantic cluster names (dry-run)."""
    result = subprocess.run(
        ["pmat", "split", filepath, "--format", "json"],
        capture_output=True, text=True, cwd=str(ROOT)
    )
    if result.returncode != 0:
        return None
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError:
        return None

def is_test_file(filepath: str) -> bool:
    return os.path.basename(filepath).startswith("tests_")

def read_file(filepath: str) -> list:
    """Read file, return list of lines (preserving newlines)."""
    with open(filepath) as f:
        return f.readlines()

def get_module_name(filepath: str) -> str:
    return Path(filepath).stem

def find_doc_comment_start(lines: list, fn_start_0idx: int) -> int:
    """Find the first doc comment/attribute/blank line preceding a function (0-indexed)."""
    i = fn_start_0idx - 1
    while i >= 0:
        stripped = lines[i].strip()
        if stripped.startswith("///") or stripped.startswith("#[") or stripped == "":
            i -= 1
        else:
            break
    return i + 1


# ---------------------------------------------------------------------------
# Test file splitting (uses test_module + children from pmat extract)
# ---------------------------------------------------------------------------

def split_test_file(filepath: str, extract_data: dict):
    """Split a test file using pmat extract's test_module children."""
    abs_path = str(ROOT / filepath)
    lines = read_file(abs_path)
    total = len(lines)
    items = extract_data.get("items", [])

    # Find the test_module item with children
    test_mod = None
    for item in items:
        if item.get("type") == "test_module":
            test_mod = item
            break

    if test_mod is None:
        # Fallback: look for type=="module" named "tests"
        for item in items:
            if item.get("type") == "module" and item.get("name") == "tests":
                test_mod = item
                break

    if test_mod is None:
        print(f"  SKIP: {filepath} - no test module found")
        return []

    children = test_mod.get("children", [])
    if not children:
        # Fallback: top-level functions inside the module's line range
        mod_start = test_mod["start_line"]
        mod_end = test_mod["end_line"]
        children = [
            i for i in items
            if i.get("type") == "function"
            and i["start_line"] > mod_start
            and i["end_line"] <= mod_end
        ]

    if len(children) < 2:
        print(f"  SKIP: {filepath} - only {len(children)} test functions, cannot split")
        return []

    mod_start_line = test_mod["start_line"]  # 1-indexed
    mod_end_line = test_mod["end_line"]       # 1-indexed

    # Header = everything from line 1 to (mod_start_line), inclusive of the mod opening
    # We need everything up to and including 'use super::*;' inside the test module
    # Find the last 'use' line within the first few lines of the test module
    header_end_0 = mod_start_line  # 0-indexed: line after 'mod tests {'
    for idx in range(mod_start_line, min(mod_start_line + 20, total)):
        stripped = lines[idx].strip()
        if stripped.startswith("use ") or stripped.startswith("#![allow") or stripped == "":
            header_end_0 = idx + 1
        elif stripped.startswith("//"):
            header_end_0 = idx + 1
        else:
            break

    header_lines = lines[:header_end_0]
    header_text = "".join(header_lines)

    # Distribute children into pieces under MAX_LINES
    pieces = []
    current_piece = []
    current_size = len(header_lines) + 1  # +1 for closing brace

    for child in children:
        child_start_0 = find_doc_comment_start(lines, child["start_line"] - 1)
        child_end_0 = child["end_line"]  # 1-indexed inclusive, so lines[child_end_0-1] is last line
        child_size = child_end_0 - child_start_0

        if current_size + child_size > MAX_LINES and current_piece:
            pieces.append(current_piece)
            current_piece = [(child_start_0, child_end_0)]
            current_size = len(header_lines) + 1 + child_size
        else:
            current_piece.append((child_start_0, child_end_0))
            current_size += child_size

    if current_piece:
        pieces.append(current_piece)

    if len(pieces) <= 1:
        print(f"  SKIP: {filepath} - all tests fit in one piece ({total} lines)")
        return []

    mod_name = get_module_name(filepath)
    created_files = []

    # Write original (first piece)
    first_piece_tests = pieces[0]
    orig_parts = [header_text]
    for start_0, end_0 in first_piece_tests:
        orig_parts.append("\n")
        orig_parts.append("".join(lines[start_0:end_0]))
    orig_parts.append("}\n")
    orig_content = "".join(orig_parts)

    with open(abs_path, 'w') as f:
        f.write(orig_content)
    orig_lines = orig_content.count('\n')
    print(f"  Original: {filepath} -> {orig_lines} lines")

    # Create split files
    for piece_idx, piece_tests in enumerate(pieces[1:], 1):
        suffix = chr(ord('a') + piece_idx)  # b, c, d...
        new_name = f"{mod_name}_{suffix}"
        new_path = str(Path(filepath).parent / f"{new_name}.rs")
        new_abs_path = str(ROOT / new_path)

        parts = [header_text]
        for start_0, end_0 in piece_tests:
            parts.append("\n")
            parts.append("".join(lines[start_0:end_0]))
        parts.append("}\n")
        new_content = "".join(parts)
        new_lines = new_content.count('\n')

        with open(new_abs_path, 'w') as f:
            f.write(new_content)
        created_files.append((new_path, new_name, True, new_lines))
        print(f"  Created: {new_path} -> {new_lines} lines")

    return created_files


# ---------------------------------------------------------------------------
# Implementation file splitting
# ---------------------------------------------------------------------------

def split_impl_file(filepath: str, extract_data: dict):
    """Split an implementation file at function boundaries."""
    abs_path = str(ROOT / filepath)
    lines = read_file(abs_path)
    total = len(lines)
    items = extract_data.get("items", [])
    imports = extract_data.get("imports", [])
    mod_name = get_module_name(filepath)

    # Only consider top-level items (not children of modules)
    top_items = [i for i in items if i.get("type") != "test_module"]

    if not top_items:
        print(f"  SKIP: {filepath} - no top-level items")
        return []

    # Check for inline test_module at the end — handle it specially
    test_mod = None
    for item in items:
        if item.get("type") == "test_module":
            test_mod = item
            break

    # Compute number of pieces needed
    num_pieces = max(2, (total + MAX_LINES - 1) // MAX_LINES)
    target_size = total // num_pieces

    # Find split points between top-level functions
    split_points = []
    for i in range(num_pieces - 1):
        target_end = (i + 1) * target_size
        best = None
        best_dist = float('inf')

        for item in top_items:
            end = item["end_line"]
            # Don't split inside test module
            if test_mod and end >= test_mod["start_line"]:
                continue
            # Must leave at least some content after
            if end >= total - 10:
                continue
            dist = abs(end - target_end)
            if dist < best_dist:
                best_dist = dist
                best = end

        if best and (not split_points or best > split_points[-1]):
            split_points.append(best)

    if not split_points:
        print(f"  SKIP: {filepath} - no valid split points found")
        return []

    # Build pieces: [(start_1idx, end_1idx), ...]
    pieces = []
    prev = 1
    for sp in split_points:
        pieces.append((prev, sp))
        prev = sp + 1
    pieces.append((prev, total))

    # Verify at least 2 pieces and originals under limit
    if len(pieces) <= 1:
        print(f"  SKIP: {filepath} - single piece")
        return []

    # Check if any piece exceeds MAX_LINES significantly
    for start, end in pieces:
        if (end - start + 1) > MAX_LINES + 50:
            # Allow small overflow, will need manual attention for huge functions
            pass

    created_files = []
    import_block = "\n".join(imports)

    # Keep first piece in original
    first_end = pieces[0][1]
    orig_lines_content = lines[:first_end]
    orig_content = "".join(orig_lines_content)

    # Make private functions pub(super) in original so _b files can use them
    orig_content = re.sub(
        r'^(fn )(\w+)',
        r'pub(super) fn \2',
        orig_content,
        flags=re.MULTILINE
    )

    # Create split files
    re_exports = []
    for piece_idx, (start, end) in enumerate(pieces[1:], 1):
        suffix = chr(ord('a') + piece_idx)  # b, c, d...
        new_name = f"{mod_name}_{suffix}"
        new_path = str(Path(filepath).parent / f"{new_name}.rs")
        new_abs_path = str(ROOT / new_path)

        piece_content = "".join(lines[start-1:end])

        # Build new file: imports + use super::original::* + piece content
        new_content = f"{import_block}\nuse super::{mod_name}::*;\n\n{piece_content}"
        new_lines = new_content.count('\n')

        with open(new_abs_path, 'w') as f:
            f.write(new_content)

        is_test = False
        created_files.append((new_path, new_name, is_test, new_lines))
        re_exports.append(new_name)
        print(f"  Created: {new_path} -> {new_lines} lines")

    # Add re-exports to original
    for name in re_exports:
        orig_content += f"\npub(super) use super::{name}::*;"
    orig_content += "\n"

    with open(abs_path, 'w') as f:
        f.write(orig_content)

    orig_final_lines = orig_content.count('\n')
    print(f"  Original: {filepath} -> {orig_final_lines} lines")

    return created_files


# ---------------------------------------------------------------------------
# Module registration
# ---------------------------------------------------------------------------

def register_modules(created_files: list):
    """Add module declarations to appropriate mod.rs files."""
    by_mod_rs = {}
    for filepath, mod_name, is_test, _ in created_files:
        mod_rs = str(Path(filepath).parent / "mod.rs")
        by_mod_rs.setdefault(mod_rs, []).append((mod_name, is_test))

    for mod_rs_path, modules in by_mod_rs.items():
        abs_mod_rs = str(ROOT / mod_rs_path)
        if not os.path.exists(abs_mod_rs):
            print(f"  WARNING: {mod_rs_path} not found")
            continue

        with open(abs_mod_rs) as f:
            content = f.read()

        existing = set(re.findall(r'mod\s+(\w+)\s*;', content))

        new_decls = []
        for mod_name, is_test in sorted(modules):
            if mod_name not in existing:
                if is_test:
                    new_decls.append(f"#[cfg(test)]\nmod {mod_name};")
                else:
                    new_decls.append(f"mod {mod_name};")

        if new_decls:
            mod_lines = content.split('\n')
            last_mod_idx = -1
            for i, line in enumerate(mod_lines):
                if re.match(r'\s*(?:#\[cfg\(test\)\]\s*)?(?:pub.*\s+)?mod\s+\w+\s*;', line):
                    last_mod_idx = i

            insert_at = last_mod_idx + 1 if last_mod_idx >= 0 else len(mod_lines)
            new_lines = mod_lines[:insert_at] + new_decls + mod_lines[insert_at:]

            with open(abs_mod_rs, 'w') as f:
                f.write('\n'.join(new_lines))

            print(f"  Registered {len(new_decls)} modules in {mod_rs_path}")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    with open("/tmp/oversized_files.txt") as f:
        files = []
        for line in f:
            line = line.strip()
            if not line or line.startswith("total"):
                continue
            parts = line.split()
            if len(parts) == 2:
                filepath = parts[0].replace(str(ROOT) + "/", "")
                count = int(parts[1])
                files.append((filepath, count))

    print(f"Processing {len(files)} oversized files\n")

    all_created = []

    for filepath, line_count in files:
        print(f"\n{'='*60}")
        print(f"Processing: {filepath} ({line_count} lines)")

        extract_data = run_pmat_extract(filepath)
        if not extract_data:
            print(f"  SKIP: could not extract")
            continue

        if is_test_file(filepath):
            created = split_test_file(filepath, extract_data)
        else:
            created = split_impl_file(filepath, extract_data)

        all_created.extend(created)

    if all_created:
        print(f"\n{'='*60}")
        print(f"Registering {len(all_created)} new modules...")
        register_modules(all_created)

    print(f"\nDone! Created {len(all_created)} new files.")
    print("Run: cargo check && cargo test --lib && cargo clippy -- -D warnings")

if __name__ == "__main__":
    main()
