#!/usr/bin/env python3
"""Validate OpenSpec drain verification matrix evidence."""
from __future__ import annotations

import argparse
import pathlib
import re
import subprocess
import sys

IMPLEMENTATION_TASK_RE = re.compile(r"^I\d+\b")
FINAL_VERIFICATION_RE = re.compile(r"^V\d+\b")
SOURCE_SUFFIXES = {".rs", ".py", ".sh", ".toml", ".nix", ".lock", ".js", ".ts", ".tsx", ".jsx", ".go"}
DOC_SUFFIXES = {".md", ".txt", ".json"}
REQUIRED_RAILS = {"build", "test", "format"}
GOOD_STATUS = {"pass", "passed", "full", "scoped", "blocked", "doc-only", "skipped-doc-only"}


def fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def section(text: str, heading: str) -> str | None:
    pattern = re.compile(rf"^##\s+{re.escape(heading)}\s*$\n(.*?)(?=^##\s+|\Z)", re.MULTILINE | re.DOTALL)
    match = pattern.search(text)
    return match.group(1).strip() if match else None


def checked_tasks(tasks_text: str) -> list[str]:
    return [m.group(1).strip() for m in re.finditer(r"^\s*-\s*\[x\]\s+(.*\S)\s*$", tasks_text, re.MULTILINE)]


def changed_files(verification_text: str) -> list[str]:
    return [m.group(1).strip() for m in re.finditer(r"^\s*-\s*Changed file:\s*`([^`]+)`\s*$", verification_text, re.MULTILINE)]


def is_source_path(path: str) -> bool:
    p = pathlib.PurePosixPath(path)
    if path.startswith("openspec/changes/"):
        return False
    if path.startswith("docs/") or path.startswith("openspec/templates/") or path.startswith("openspec/specs/"):
        return False
    return p.suffix in SOURCE_SUFFIXES


def parse_matrix(matrix_text: str) -> dict[str, dict[str, str]]:
    rows: dict[str, dict[str, str]] = {}
    for raw in matrix_text.splitlines():
        line = raw.strip()
        if not line.startswith("|") or "---" in line.lower():
            continue
        cells = [cell.strip().lower() for cell in line.strip("|").split("|")]
        if not cells or cells[0] in {"rail", ""}:
            continue
        if len(cells) < 6:
            fail("Drain Verification Matrix rows must include rail, command, status, artifact, scope rationale, and next best check")
        rail = cells[0]
        rows[rail] = {
            "command": cells[1],
            "status": cells[2],
            "artifact": cells[3],
            "rationale": cells[4],
            "next": cells[5],
        }
    return rows


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("change", help="OpenSpec change directory or name")
    parser.add_argument("--repo-root", default=".")
    args = parser.parse_args()

    repo_root = pathlib.Path(args.repo_root).resolve()
    candidate = pathlib.Path(args.change)
    if not candidate.is_absolute():
        if candidate.exists():
            change_dir = candidate.resolve()
        elif (repo_root / "openspec" / "changes" / args.change).exists():
            change_dir = (repo_root / "openspec" / "changes" / args.change).resolve()
        else:
            change_dir = (repo_root / args.change).resolve()
    else:
        change_dir = candidate.resolve()

    tasks_path = change_dir / "tasks.md"
    verification_path = change_dir / "verification.md"
    if not tasks_path.exists() or not verification_path.exists():
        print("OK: no checked-task verification matrix required yet")
        return 0

    tasks = checked_tasks(tasks_path.read_text())
    if not tasks:
        print("OK: no checked tasks require drain verification matrix")
        return 0

    has_impl = any(IMPLEMENTATION_TASK_RE.match(task) for task in tasks)
    has_final_v = any(FINAL_VERIFICATION_RE.match(task) for task in tasks)
    if not has_impl and not has_final_v:
        print("OK: checked tasks do not require drain verification matrix")
        return 0

    verification_text = verification_path.read_text()
    files = changed_files(verification_text)
    source_changed = any(is_source_path(path) for path in files)
    matrix = section(verification_text, "Drain Verification Matrix")
    if not matrix:
        fail("verification.md must include ## Drain Verification Matrix before checked implementation/final verification tasks")

    rows = parse_matrix(matrix)
    missing = sorted(REQUIRED_RAILS - set(rows))
    if missing:
        fail("Drain Verification Matrix missing required rails: " + ", ".join(missing))

    for rail in sorted(REQUIRED_RAILS):
        row = rows[rail]
        status = row["status"]
        if status not in GOOD_STATUS:
            fail(f"Drain Verification Matrix rail {rail} has invalid or empty status: {status!r}")
        if status in {"pass", "passed", "full", "scoped"} and (not row["command"] or row["command"] in {"-", "n/a"}):
            fail(f"Drain Verification Matrix rail {rail} must name a command for status {status}")
        if status in {"pass", "passed", "full", "scoped"} and (not row["artifact"] or row["artifact"] in {"-", "n/a"}):
            fail(f"Drain Verification Matrix rail {rail} must name an artifact for status {status}")
        if status in {"scoped", "blocked"} and (not row["rationale"] or row["rationale"] in {"-", "n/a"} or not row["next"] or row["next"] in {"-", "n/a"}):
            fail(f"Drain Verification Matrix rail {rail} requires scope rationale and next best check for status {status}")
        if status in {"doc-only", "skipped-doc-only"}:
            if source_changed:
                fail(f"Drain Verification Matrix rail {rail} cannot use doc-only status while source/tooling files changed")
            if not row["rationale"] or row["rationale"] in {"-", "n/a"}:
                fail(f"Drain Verification Matrix rail {rail} doc-only status requires rationale")

    print("OK: drain verification matrix is complete")
    print("  source changes:", "yes" if source_changed else "no")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
