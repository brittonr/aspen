#!/usr/bin/env python3
"""Reject ambient `std` in the production sources of a `no_std` kernel crate.

`#![cfg_attr(not(test), no_std)]` makes an unqualified `use std::...` fail to compile, but an
explicit `extern crate std;` would quietly link the standard library back in. This guard scans
every production source file of a crate (`src/**/*.rs` except `tests.rs`, `tests/` trees and
files declared only under `#[cfg(test)]`) and rejects, outside comments and string literals:

- `extern crate std` in any form;
- any `std::` path (including `::std::`).

Usage:
    check-no-std-sources.py CRATE_DIR...

Exit status: 0 clean, 1 findings, 2 input error (fail closed).
"""
# r[impl molten.project.tier1_no_std_kernel_crates]
import re
import sys
from pathlib import Path

EXTERN_STD = re.compile(r"\bextern\s+crate\s+std\b")
STD_PATH = re.compile(r"(?<![\w:])(?:::)?std::")
TEST_MODULE = re.compile(r"#\[cfg\(test\)\]\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*;")


def strip_comments_and_strings(text: str) -> str:
    """Blank comments and string/char literals while keeping line structure."""
    output = []
    index = 0
    length = len(text)
    while index < length:
        if text.startswith("//", index):
            end = text.find("\n", index)
            end = length if end < 0 else end
            output.append(" " * (end - index))
            index = end
        elif text.startswith("/*", index):
            end = text.find("*/", index + 2)
            end = length if end < 0 else end + 2
            output.append(re.sub(r"[^\n]", " ", text[index:end]))
            index = end
        elif text[index] == '"':
            end = index + 1
            while end < length and text[end] != '"':
                end += 2 if text[end] == "\\" else 1
            end = min(end + 1, length)
            output.append(re.sub(r"[^\n]", " ", text[index:end]))
            index = end
        else:
            output.append(text[index])
            index += 1
    return "".join(output)


def test_only_files(source_root: Path) -> set[Path]:
    """Files mounted by `#[cfg(test)] mod name;` (and everything below them)."""
    excluded = set()
    for path in source_root.rglob("*.rs"):
        for name in TEST_MODULE.findall(path.read_text(encoding="utf-8")):
            parent = path.parent if path.name in {"lib.rs", "mod.rs"} else path.with_suffix("")
            excluded.add(parent / f"{name}.rs")
            excluded.add(parent / name)
    return excluded


def is_excluded(path: Path, excluded: set[Path]) -> bool:
    return any(path == item or item in path.parents for item in excluded)


def findings(crate_dir: Path) -> list[str]:
    source_root = crate_dir / "src"
    if not source_root.is_dir():
        raise FileNotFoundError(f"{source_root} is not a directory")
    excluded = test_only_files(source_root)
    located = []
    for path in sorted(source_root.rglob("*.rs")):
        if is_excluded(path, excluded):
            continue
        code = strip_comments_and_strings(path.read_text(encoding="utf-8"))
        for number, line in enumerate(code.splitlines(), start=1):
            if EXTERN_STD.search(line):
                located.append(f"{path}:{number}: production source links std with `extern crate std`")
            elif STD_PATH.search(line):
                located.append(f"{path}:{number}: production source names a `std::` path")
    return located


def main(argv: list[str]) -> int:
    if not argv:
        print("usage: check-no-std-sources.py CRATE_DIR...", file=sys.stderr)
        return 2
    status = 0
    for crate in argv:
        try:
            located = findings(Path(crate))
        except (OSError, UnicodeDecodeError) as error:
            print(f"check-no-std-sources: {error}", file=sys.stderr)
            return 2
        for finding in located:
            print(finding, file=sys.stderr)
            status = 1
    return status


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
