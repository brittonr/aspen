#!/usr/bin/env python3
"""Guard canonical OpenSpec main specs against archive/delta-format drift.

This checker is intentionally lightweight and dependency-free so it can run in
Nix sandbox checks before heavier OpenSpec validation. It validates only live
canonical specs under openspec/specs/**/spec.md, not active change deltas.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass
from pathlib import Path

DELTA_HEADING_RE = re.compile(r"^##\s+(ADDED|MODIFIED|REMOVED)\s+Requirements\s*$")
TITLE_RE = re.compile(r"^#\s+(.+?)\s+Specification\s*$")
REQUIREMENT_RE = re.compile(r"^###\s+(.+)$")
SCENARIO_RE = re.compile(r"^####\s+Scenario:\s+(.+)$")
PLACEHOLDER_SNIPPETS = (
    "TBD - created by archiving change",
    "Update Purpose after archive",
)


@dataclass(frozen=True)
class Finding:
    path: str
    line: int
    code: str
    message: str


def line_number(lines: list[str], prefix: str) -> int:
    for index, line in enumerate(lines, start=1):
        if line.startswith(prefix):
            return index
    return 1


def non_empty_block(lines: list[str], start: int, end: int) -> bool:
    return any(line.strip() for line in lines[start:end])


def check_spec(path: Path, root: Path) -> list[Finding]:
    rel = str(path.relative_to(root))
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()
    findings: list[Finding] = []

    if not lines:
        return [Finding(rel, 1, "empty-spec", "canonical spec is empty")]

    title_match = TITLE_RE.match(lines[0].strip())
    if not title_match:
        findings.append(
            Finding(
                rel,
                1,
                "missing-title",
                "first line must be '# <id> Specification'",
            )
        )
    elif title_match.group(1) != path.parent.name:
        findings.append(
            Finding(
                rel,
                1,
                "title-id-mismatch",
                f"title id must match spec directory '{path.parent.name}'",
            )
        )

    purpose_indices = [index for index, line in enumerate(lines) if line.strip() == "## Purpose"]
    requirements_indices = [index for index, line in enumerate(lines) if line.strip() == "## Requirements"]

    if len(purpose_indices) != 1:
        findings.append(
            Finding(
                rel,
                line_number(lines, "## Purpose"),
                "purpose-count",
                "canonical spec must contain exactly one '## Purpose' heading",
            )
        )
    if len(requirements_indices) != 1:
        findings.append(
            Finding(
                rel,
                line_number(lines, "## Requirements"),
                "requirements-count",
                "canonical spec must contain exactly one '## Requirements' heading",
            )
        )

    if purpose_indices and requirements_indices:
        purpose = purpose_indices[0]
        requirements = requirements_indices[0]
        if purpose > requirements:
            findings.append(
                Finding(
                    rel,
                    purpose + 1,
                    "purpose-after-requirements",
                    "'## Purpose' must appear before '## Requirements'",
                )
            )
        elif not non_empty_block(lines, purpose + 1, requirements):
            findings.append(
                Finding(
                    rel,
                    purpose + 1,
                    "empty-purpose",
                    "purpose section must contain concrete text",
                )
            )

    for index, line in enumerate(lines, start=1):
        stripped = line.strip()
        if DELTA_HEADING_RE.match(stripped):
            findings.append(
                Finding(
                    rel,
                    index,
                    "delta-heading-in-canonical-spec",
                    "live canonical specs must use '## Requirements', not delta headings",
                )
            )
        if any(snippet in line for snippet in PLACEHOLDER_SNIPPETS):
            findings.append(
                Finding(
                    rel,
                    index,
                    "archive-purpose-placeholder",
                    "archive-generated Purpose placeholder must be replaced",
                )
            )

    return findings


def check_specs(root: Path) -> tuple[int, list[Finding]]:
    specs_dir = root / "openspec" / "specs"
    if not specs_dir.is_dir():
        return 0, [Finding("openspec/specs", 1, "missing-specs-dir", "openspec/specs is missing")]

    spec_paths = sorted(specs_dir.glob("*/spec.md"))
    findings: list[Finding] = []
    for path in spec_paths:
        findings.extend(check_spec(path, root))
    return len(spec_paths), findings


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default=".", help="repository root (default: current directory)")
    parser.add_argument("--json", action="store_true", help="emit JSON report")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    checked, findings = check_specs(root)
    report = {
        "checked": checked,
        "passed": not findings,
        "findings": [asdict(finding) for finding in findings],
    }

    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        if findings:
            print(f"OpenSpec canonical hygiene failed: {len(findings)} finding(s)")
            for finding in findings:
                print(f"{finding.path}:{finding.line}: {finding.code}: {finding.message}")
        else:
            print(f"OpenSpec canonical hygiene passed: {checked} spec(s) checked")

    return 1 if findings else 0


if __name__ == "__main__":
    sys.exit(main())
