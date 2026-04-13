#!/usr/bin/env python3
"""Tiger Style audit scanner for Aspen Rust sources.

Scans Rust source files for a bounded set of review hotspots:
- overlong function bodies
- zero-assert overlong functions
- recursive functions
- verified code that reads ambient time
- public or verified APIs that expose `usize`

The parser is intentionally small. It is good enough for the repo's audit
workflow, but `audit-report.md` should still describe its limitations.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

DEFAULT_ROOTS = [Path("crates")]
DEFAULT_LINE_LIMIT = 70
FUNCTION_START_RE = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:(?:async|const|unsafe|extern\s+\"[^\"]+\"|default)\s+)*fn\s+([A-Za-z_][A-Za-z0-9_]*)"
)
ASSERT_RE = re.compile(r"\b(?:debug_assert|assert(?:_(?:eq|ne|matches))?)!\b")
NOW_PATTERNS = ("SystemTime::now()", "Instant::now()")


def is_char_literal_start(text: str, index: int) -> bool:
    next_index = index + 1
    if next_index >= len(text):
        return False
    next_char = text[next_index]
    if next_char == "\\":
        escape_end = index + 3
        return escape_end < len(text) and text[escape_end] == "'"
    char_end = index + 2
    return char_end < len(text) and text[char_end] == "'"


@dataclass
class FunctionInfo:
    file: str
    function: str
    line: int
    end_line: int
    line_count: int
    signature: str
    body: str
    is_public: bool
    is_verified: bool
    assert_count: int


class ParseError(RuntimeError):
    pass


def strip_line_comments(line: str) -> str:
    chars: list[str] = []
    i = 0
    in_string = False
    in_char = False
    escape = False
    while i < len(line):
        ch = line[i]
        nxt = line[i + 1] if i + 1 < len(line) else ""
        if not in_string and not in_char and ch == "/" and nxt == "/":
            break
        chars.append(ch)
        if escape:
            escape = False
        elif ch == "\\":
            escape = True
        elif ch == '"' and not in_char:
            in_string = not in_string
        elif ch == "'" and not in_string:
            if is_char_literal_start(line, i):
                in_char = not in_char
        i += 1
    return "".join(chars)


def signature_result(lines: list[str], start_index: int, match_column: int) -> tuple[bool, int, int, str]:
    """Return (has_body, brace_line, brace_col, signature_text)."""
    paren_depth = 0
    bracket_depth = 0
    angle_depth = 0
    signature_parts: list[str] = []

    for line_index in range(start_index, len(lines)):
        raw_line = lines[line_index]
        line = strip_line_comments(raw_line)
        start_col = match_column if line_index == start_index else 0
        signature_parts.append(line[start_col:])
        col = start_col
        while col < len(line):
            ch = line[col]
            if ch == "(":
                paren_depth += 1
            elif ch == ")":
                paren_depth = max(0, paren_depth - 1)
            elif ch == "[":
                bracket_depth += 1
            elif ch == "]":
                bracket_depth = max(0, bracket_depth - 1)
            elif ch == "<":
                angle_depth += 1
            elif ch == ">" and angle_depth > 0:
                angle_depth -= 1
            elif paren_depth == 0 and bracket_depth == 0 and angle_depth == 0:
                if ch == ";":
                    return False, line_index, col, "".join(signature_parts)
                if ch == "{":
                    return True, line_index, col, "".join(signature_parts)
            col += 1
        signature_parts.append("\n")

    raise ParseError(f"unterminated signature starting on line {start_index + 1}")


class BodyScanner:
    def __init__(self) -> None:
        self.in_block_comment = False
        self.in_string = False
        self.in_char = False
        self.escape = False

    def feed(self, text: str, brace_depth: int) -> int:
        i = 0
        while i < len(text):
            ch = text[i]
            nxt = text[i + 1] if i + 1 < len(text) else ""
            if self.in_block_comment:
                if ch == "*" and nxt == "/":
                    self.in_block_comment = False
                    i += 2
                    continue
                i += 1
                continue
            if self.in_string:
                if self.escape:
                    self.escape = False
                elif ch == "\\":
                    self.escape = True
                elif ch == '"':
                    self.in_string = False
                i += 1
                continue
            if self.in_char:
                if self.escape:
                    self.escape = False
                elif ch == "\\":
                    self.escape = True
                elif ch == "'":
                    self.in_char = False
                i += 1
                continue
            if ch == "/" and nxt == "*":
                self.in_block_comment = True
                i += 2
                continue
            if ch == "/" and nxt == "/":
                break
            if ch == '"':
                self.in_string = True
                i += 1
                continue
            if ch == "'":
                if is_char_literal_start(text, i):
                    self.in_char = True
                i += 1
                continue
            if ch == "{":
                brace_depth += 1
            elif ch == "}":
                brace_depth -= 1
            i += 1
        return brace_depth


def parse_function(lines: list[str], path: Path, start_index: int, match: re.Match[str]) -> tuple[FunctionInfo | None, int]:
    has_body, brace_line, brace_col, signature = signature_result(lines, start_index, match.start())
    if not has_body:
        return None, brace_line + 1

    scanner = BodyScanner()
    brace_depth = 0
    body_chunks: list[str] = []
    end_line = brace_line
    for line_index in range(brace_line, len(lines)):
        line = lines[line_index]
        start_col = brace_col if line_index == brace_line else 0
        segment = line[start_col:]
        body_chunks.append(segment)
        brace_depth = scanner.feed(segment, brace_depth)
        if brace_depth == 0:
            end_line = line_index
            break
    else:
        raise ParseError(f"unterminated body for {match.group(1)} in {path}")

    body = "".join(body_chunks)
    is_verified = f"{os.sep}src{os.sep}verified{os.sep}" in str(path)
    is_public = match.group(0).lstrip().startswith("pub")
    info = FunctionInfo(
        file=str(path),
        function=match.group(1),
        line=start_index + 1,
        end_line=end_line + 1,
        line_count=end_line - start_index + 1,
        signature=signature,
        body=body,
        is_public=is_public,
        is_verified=is_verified,
        assert_count=len(ASSERT_RE.findall(body)),
    )
    return info, end_line + 1


def find_functions(path: Path) -> list[FunctionInfo]:
    lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    functions: list[FunctionInfo] = []
    index = 0
    while index < len(lines):
        match = FUNCTION_START_RE.match(lines[index])
        if not match:
            index += 1
            continue
        function, next_index = parse_function(lines, path, index, match)
        if function is not None:
            functions.append(function)
        index = max(next_index, index + 1)
    return functions


def should_scan_child(root: Path, child: Path) -> bool:
    if "target" in child.parts:
        return False
    root_parts = set(root.parts)
    if "crates" in root_parts:
        return "src" in child.parts
    return True


def iter_rust_files(paths: list[Path]) -> Iterable[Path]:
    seen: set[Path] = set()
    for path in paths:
        if path.is_file() and path.suffix == ".rs":
            resolved = path.resolve()
            if resolved not in seen:
                seen.add(resolved)
                yield path
            continue
        if not path.exists():
            continue
        for child in sorted(path.rglob("*.rs")):
            if not child.is_file() or not should_scan_child(path, child):
                continue
            resolved = child.resolve()
            if resolved not in seen:
                seen.add(resolved)
                yield child


def usize_hotspots(functions: list[FunctionInfo]) -> list[dict]:
    hotspots: list[dict] = []
    for function in functions:
        if "usize" not in function.signature:
            continue
        if not (function.is_public or function.is_verified):
            continue
        hotspots.append(
            {
                "rule": "usize_api",
                "file": function.file,
                "function": function.function,
                "line": function.line,
                "line_count": function.line_count,
                "assert_count": function.assert_count,
                "detail": "public or verified signature exposes usize",
            }
        )
    return hotspots


def recursive_hotspots(functions: list[FunctionInfo]) -> list[dict]:
    hotspots: list[dict] = []
    for function in functions:
        recursive_call = re.search(rf"\b{re.escape(function.function)}\s*\(", function.body)
        if not recursive_call:
            continue
        hotspots.append(
            {
                "rule": "recursion",
                "file": function.file,
                "function": function.function,
                "line": function.line,
                "line_count": function.line_count,
                "assert_count": function.assert_count,
                "detail": "function body calls itself",
            }
        )
    return hotspots


def strip_comments_preserve_layout(text: str) -> str:
    chars: list[str] = []
    index = 0
    in_block_comment = False
    in_line_comment = False
    in_string = False
    in_char = False
    escape = False
    while index < len(text):
        ch = text[index]
        nxt = text[index + 1] if index + 1 < len(text) else ""
        if in_line_comment:
            if ch == "\n":
                in_line_comment = False
                chars.append(ch)
            else:
                chars.append(" ")
            index += 1
            continue
        if in_block_comment:
            if ch == "*" and nxt == "/":
                chars.extend("  ")
                in_block_comment = False
                index += 2
                continue
            chars.append("\n" if ch == "\n" else " ")
            index += 1
            continue
        if in_string:
            chars.append(ch)
            if escape:
                escape = False
            elif ch == "\\":
                escape = True
            elif ch == '"':
                in_string = False
            index += 1
            continue
        if in_char:
            chars.append(ch)
            if escape:
                escape = False
            elif ch == "\\":
                escape = True
            elif ch == "'":
                in_char = False
            index += 1
            continue
        if ch == "/" and nxt == "/":
            chars.extend("  ")
            in_line_comment = True
            index += 2
            continue
        if ch == "/" and nxt == "*":
            chars.extend("  ")
            in_block_comment = True
            index += 2
            continue
        if ch == '"':
            in_string = True
            chars.append(ch)
            index += 1
            continue
        if ch == "'":
            chars.append(ch)
            if is_char_literal_start(text, index):
                in_char = True
            index += 1
            continue
        chars.append(ch)
        index += 1
    return "".join(chars)


def verified_time_hotspots(files: list[Path]) -> list[dict]:
    hotspots: list[dict] = []
    for path in files:
        path_str = str(path)
        if f"{os.sep}src{os.sep}verified{os.sep}" not in path_str:
            continue
        text = strip_comments_preserve_layout(path.read_text(encoding="utf-8"))
        for pattern in NOW_PATTERNS:
            if pattern not in text:
                continue
            line = text[: text.index(pattern)].count("\n") + 1
            hotspots.append(
                {
                    "rule": "verified_ambient_time",
                    "file": path_str,
                    "function": None,
                    "line": line,
                    "line_count": 1,
                    "assert_count": 0,
                    "detail": f"verified code reads ambient time with {pattern}",
                }
            )
    return hotspots


def length_hotspots(functions: list[FunctionInfo], line_limit: int) -> list[dict]:
    hotspots: list[dict] = []
    for function in functions:
        if function.line_count <= line_limit:
            continue
        hotspots.append(
            {
                "rule": "function_length",
                "file": function.file,
                "function": function.function,
                "line": function.line,
                "line_count": function.line_count,
                "assert_count": function.assert_count,
                "detail": f"function exceeds {line_limit} lines",
            }
        )
        if function.assert_count == 0:
            hotspots.append(
                {
                    "rule": "zero_assert_hotspot",
                    "file": function.file,
                    "function": function.function,
                    "line": function.line,
                    "line_count": function.line_count,
                    "assert_count": function.assert_count,
                    "detail": f"overlong function exceeds {line_limit} lines with no assertions",
                }
            )
    return hotspots


def sort_hotspots(hotspots: list[dict]) -> list[dict]:
    rule_rank = {
        "zero_assert_hotspot": 0,
        "function_length": 1,
        "recursion": 2,
        "verified_ambient_time": 3,
        "usize_api": 4,
    }
    return sorted(
        hotspots,
        key=lambda item: (
            rule_rank.get(item["rule"], 99),
            -int(item.get("line_count") or 0),
            item["file"],
            item.get("function") or "",
            int(item.get("line") or 0),
        ),
    )


def scan(paths: list[Path], line_limit: int) -> dict:
    files = list(iter_rust_files(paths))
    functions: list[FunctionInfo] = []
    parse_errors: list[str] = []
    for path in files:
        try:
            functions.extend(find_functions(path))
        except (ParseError, UnicodeDecodeError) as error:
            parse_errors.append(f"{path}: {error}")

    hotspots = []
    hotspots.extend(length_hotspots(functions, line_limit))
    hotspots.extend(recursive_hotspots(functions))
    hotspots.extend(verified_time_hotspots(files))
    hotspots.extend(usize_hotspots(functions))
    sorted_hotspots = sort_hotspots(hotspots)

    counts = Counter(item["rule"] for item in sorted_hotspots)
    return {
        "scanner": "scripts/tigerstyle-audit.py",
        "line_limit": line_limit,
        "roots": [str(path) for path in paths],
        "files_scanned": len(files),
        "functions_scanned": len(functions),
        "hotspot_counts": dict(sorted(counts.items())),
        "hotspots": sorted_hotspots,
        "parse_errors": parse_errors,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="*", type=Path, help="files or directories to scan")
    parser.add_argument("--line-limit", type=int, default=DEFAULT_LINE_LIMIT)
    parser.add_argument("--json", action="store_true", help="emit machine-readable JSON")
    parser.add_argument("--pretty", action="store_true", help="pretty-print JSON")
    return parser.parse_args(argv)


def print_human_report(result: dict) -> None:
    print(f"scanner: {result['scanner']}")
    print(f"roots: {', '.join(result['roots'])}")
    print(f"files scanned: {result['files_scanned']}")
    print(f"functions scanned: {result['functions_scanned']}")
    print("hotspot counts:")
    for rule, count in sorted(result["hotspot_counts"].items()):
        print(f"  {rule}: {count}")
    print("top hotspots:")
    for hotspot in result["hotspots"][:20]:
        function = hotspot.get("function") or "<file>"
        print(
            f"  {hotspot['rule']}: {hotspot['file']}:{hotspot['line']} {function} "
            f"({hotspot['line_count']} lines, {hotspot['assert_count']} asserts)"
        )
    if result["parse_errors"]:
        print("parse errors:")
        for error in result["parse_errors"]:
            print(f"  {error}")


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    roots = args.paths or DEFAULT_ROOTS
    result = scan(roots, args.line_limit)
    if args.json or args.pretty:
        indent = 2 if args.pretty else None
        json.dump(result, sys.stdout, indent=indent, sort_keys=bool(indent))
        sys.stdout.write("\n")
    else:
        print_human_report(result)
    return 1 if result["parse_errors"] else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
