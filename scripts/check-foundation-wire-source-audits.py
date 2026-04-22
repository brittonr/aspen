#!/usr/bin/env python3
"""Deterministic source audits for foundation leaf and wire crates."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import Final

ROOT: Final[Path] = Path(__file__).resolve().parents[1]
TEST_CFG_PREFIX: Final[str] = "#[cfg(test)]"
LINE_COMMENT_MARKER: Final[str] = "//"

LEAF_TARGETS: Final[dict[Path, tuple[str, ...]]] = {
    Path("crates/aspen-storage-types/src"): (
        "redb::TableDefinition",
        "TableDefinition::new",
        "redb::Database",
        "redb::ReadTransaction",
        "redb::WriteTransaction",
    ),
    Path("crates/aspen-traits/src"): (
        "std::sync::Arc",
    ),
}
WIRE_TARGETS: Final[dict[Path, tuple[str, ...]]] = {
    Path("crates/aspen-client-api/src"): (
        "std::collections",
        "postcard::to_stdvec",
        "serde_json",
    ),
    Path("crates/aspen-coordination-protocol/src"): (
        "std::collections",
        "postcard::to_stdvec",
        "serde_json",
    ),
    Path("crates/aspen-jobs-protocol/src"): (
        "std::collections",
        "postcard::to_stdvec",
        "serde_json",
    ),
    Path("crates/aspen-forge-protocol/src"): (
        "std::collections",
        "postcard::to_stdvec",
        "serde_json",
    ),
}


class AuditError(RuntimeError):
    pass


class ResultRecorder:
    def __init__(self) -> None:
        self.failures: list[str] = []

    def pass_line(self, message: str) -> None:
        print(f"PASS {message}")

    def fail_line(self, message: str) -> None:
        self.failures.append(message)
        print(f"FAIL {message}")



def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mode", choices=("leaf", "wire"), required=True)
    return parser.parse_args(argv)



def rust_files(directory: Path) -> list[Path]:
    if not directory.is_dir():
        raise AuditError(f"missing directory {directory}")
    return sorted(path for path in directory.rglob("*.rs") if "tests" not in path.parts)



def strip_line_comment(line: str) -> str:
    if line.lstrip().startswith("//"):
        return ""
    return line.split(LINE_COMMENT_MARKER, maxsplit=1)[0]



def brace_delta(text: str) -> int:
    return text.count("{") - text.count("}")



def production_text(path: Path) -> str:
    lines = path.read_text().splitlines()
    current_depth = 0
    pending_test_item = False
    skip_until_depth: int | None = None
    production_lines: list[str] = []
    for raw_line in lines:
        stripped_line = raw_line.strip()
        if stripped_line.startswith(TEST_CFG_PREFIX):
            pending_test_item = True
            continue
        uncommented = strip_line_comment(raw_line)
        stripped = uncommented.strip()
        if skip_until_depth is not None:
            current_depth += brace_delta(uncommented)
            if current_depth <= skip_until_depth:
                skip_until_depth = None
            continue
        if pending_test_item:
            pending_test_item = False
            if "{" in uncommented:
                skip_until_depth = current_depth
                current_depth += brace_delta(uncommented)
                if current_depth <= skip_until_depth:
                    skip_until_depth = None
            continue
        if stripped:
            production_lines.append(stripped)
        current_depth += brace_delta(uncommented)
    return "\n".join(production_lines)



def audit_targets(targets: dict[Path, tuple[str, ...]], recorder: ResultRecorder) -> None:
    for directory, forbidden_patterns in targets.items():
        for path in rust_files(ROOT / directory):
            text = production_text(path)
            for forbidden_pattern in forbidden_patterns:
                if forbidden_pattern in text:
                    recorder.fail_line(f"{path} still contains `{forbidden_pattern}` outside test-only code")
        recorder.pass_line(f"{directory} excludes forbidden helpers outside tests")



def main(argv: list[str]) -> int:
    args = parse_args(argv)
    recorder = ResultRecorder()
    try:
        if args.mode == "leaf":
            audit_targets(LEAF_TARGETS, recorder)
        else:
            audit_targets(WIRE_TARGETS, recorder)
    except AuditError as error:
        recorder.fail_line(str(error))
    if recorder.failures:
        print("SUMMARY failed")
        return 1
    print("SUMMARY ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
