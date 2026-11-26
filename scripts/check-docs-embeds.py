#!/usr/bin/env python3
"""Reject Rust `include_str!`/`include_bytes!` embeds that read from `docs/`.

Governing documentation is prose; data a binary embeds (generated Nickel exports, policy JSON,
fixtures) lives under `config/` or beside the code. Any string literal inside an
`include_str!(...)` or `include_bytes!(...)` invocation whose path has a `docs/` segment is a
finding, including paths assembled with `concat!(env!("CARGO_MANIFEST_DIR"), "/docs/...")`.

Scanned roots, relative to the repository root: `src/`, `crates/`, `tests/`, `tools/`,
`checks/`, `pilots/`, and `examples/`, `*.rs` only; `target/` and `vendor/` are never scanned.

Usage:
    check-docs-embeds.py [--root DIR]             # whole tree
    check-docs-embeds.py [--root DIR] PATH...     # explicit files

Exit status: 0 clean, 1 findings, 2 input error (fail closed).
"""
# r[impl molten.project.real_module_encapsulation]
import argparse
import re
import sys
from pathlib import Path

SCANNED_ROOTS = ("src", "crates", "tests", "tools", "checks", "pilots", "examples")
SKIPPED_DIRECTORIES = {"target", "vendor", ".git"}
EMBED = re.compile(r"\binclude_(?:str|bytes)!\s*\(")
STRING_LITERAL = re.compile(r'"((?:[^"\\]|\\.)*)"')
DOCS_SEGMENT = re.compile(r"(?:^|/)docs/")


def invocation_end(text: str, open_paren: int) -> int:
    """Index just past the parenthesis closing the one at `open_paren`; -1 if unbalanced."""
    depth = 0
    index = open_paren
    in_string = False
    while index < len(text):
        character = text[index]
        if in_string:
            if character == "\\":
                index += 2
                continue
            if character == '"':
                in_string = False
        elif character == '"':
            in_string = True
        elif character == "(":
            depth += 1
        elif character == ")":
            depth -= 1
            if depth == 0:
                return index + 1
        index += 1
    return -1


def findings(text: str) -> list[tuple[int, str]]:
    """(line, embedded path literal) for every embed reading under a `docs/` segment."""
    located = []
    for match in EMBED.finditer(text):
        end = invocation_end(text, match.end() - 1)
        if end < 0:
            located.append((text.count("\n", 0, match.start()) + 1, "<unbalanced include invocation>"))
            continue
        for literal in STRING_LITERAL.finditer(text, match.end(), end):
            if DOCS_SEGMENT.search(literal.group(1)):
                located.append((text.count("\n", 0, match.start()) + 1, literal.group(1)))
    return located


def rust_files(root: Path) -> list[Path]:
    files = []
    for name in SCANNED_ROOTS:
        base = root / name
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*.rs")):
            if SKIPPED_DIRECTORIES.isdisjoint(path.relative_to(root).parts):
                files.append(path)
    return files


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("paths", nargs="*", type=Path)
    arguments = parser.parse_args(argv)
    root = arguments.root.resolve()
    if not root.is_dir():
        print(f"check-docs-embeds: root {root} is not a directory", file=sys.stderr)
        return 2
    paths = [path if path.is_absolute() else root / path for path in arguments.paths] or rust_files(root)
    status = 0
    for path in paths:
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError) as error:
            print(f"check-docs-embeds: cannot read {path}: {error}", file=sys.stderr)
            return 2
        for line, literal in findings(text):
            shown = path.relative_to(root) if path.is_relative_to(root) else path
            print(f"{shown}:{line}: embeds documentation path {literal!r}; move the data under config/", file=sys.stderr)
            status = 1
    return status


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
