#!/usr/bin/env python3
"""Validate OpenSpec task size, fan-out, and dependency-order guidance."""
from __future__ import annotations

import argparse
import dataclasses
import pathlib
import re
import sys

TASK_RE = re.compile(r"^\s*-\s*\[[ x~]\]\s+(?P<body>.*\S)\s*$")
LABEL_RE = re.compile(r"\b(?P<label>[IV]\d+)\b")
COVERS_RE = re.compile(r"\[covers=([^\]]+)\]")
ID_SPLIT_RE = re.compile(r"[,\s]+")
DEPENDENCY_RE = re.compile(r"\b(depends? on|requires?|after|blocked by|foundation)\b", re.IGNORECASE)
PREREQ_RE = re.compile(r"\b(prerequisite|depends on earlier|after earlier|ordered after|foundation first)\b", re.IGNORECASE)
INTEGRATION_RE = re.compile(r"\b(integration proof|integration verification|end-to-end|e2e)\b", re.IGNORECASE)
EVIDENCE_RE = re.compile(r"\b(evidence|artifact|transcript|fixture|report|matrix)\b", re.IGNORECASE)


@dataclasses.dataclass(frozen=True)
class Task:
    line_no: int
    body: str
    label: str | None
    covers: tuple[str, ...]


def fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def warn(message: str) -> None:
    print(f"WARN: {message}")


def resolve_change(repo_root: pathlib.Path, value: str) -> pathlib.Path:
    candidate = pathlib.Path(value)
    if candidate.is_absolute() and candidate.is_dir():
        return candidate.resolve()
    if candidate.is_dir():
        return candidate.resolve()
    for base in (repo_root / "openspec" / "changes", repo_root / "openspec" / "changes" / "active"):
        candidate = base / value
        if candidate.is_dir():
            return candidate.resolve()
    return (repo_root / value).resolve()


def parse_tasks(tasks_path: pathlib.Path) -> list[Task]:
    tasks: list[Task] = []
    for line_no, line in enumerate(tasks_path.read_text(errors="replace").splitlines(), start=1):
        match = TASK_RE.match(line)
        if not match:
            continue
        body = match.group("body")
        label_match = LABEL_RE.search(body)
        covers_match = COVERS_RE.search(body)
        covers: tuple[str, ...] = ()
        if covers_match:
            covers = tuple(part.strip() for part in ID_SPLIT_RE.split(covers_match.group(1)) if part.strip())
        tasks.append(Task(line_no, body, label_match.group("label") if label_match else None, covers))
    return tasks


def is_verification(task: Task) -> bool:
    return bool(task.label and task.label.startswith("V"))


def task_number(task: Task) -> int | None:
    if not task.label:
        return None
    try:
        return int(task.label[1:])
    except ValueError:
        return None


def check_fanout(task: Task, threshold: int) -> list[str]:
    issues: list[str] = []
    fanout = len(set(task.covers))
    if fanout <= threshold:
        return issues
    if is_verification(task) and INTEGRATION_RE.search(task.body) and EVIDENCE_RE.search(task.body):
        return issues
    if is_verification(task):
        issues.append(
            f"line {task.line_no}: broad verification task covers {fanout} IDs but is not labeled as an integration proof with evidence expectations; "
            "label it as integration proof or split it"
        )
    else:
        issues.append(
            f"line {task.line_no}: oversized implementation task covers {fanout} IDs; split by requirement/scenario or mark a separate integration verification task"
        )
    return issues


def check_dependencies(task: Task) -> list[str]:
    issues: list[str] = []
    if not DEPENDENCY_RE.search(task.body):
        return issues
    current = task_number(task)
    later_refs = []
    if current is not None:
        for ref in re.findall(r"\bI(\d+)\b", task.body):
            try:
                ref_num = int(ref)
            except ValueError:
                continue
            if ref_num > current:
                later_refs.append(f"I{ref_num}")
    if (later_refs or "foundation" in task.body.lower()) and not PREREQ_RE.search(task.body):
        detail = f" later refs {', '.join(later_refs)}" if later_refs else " ambiguous foundation dependency"
        issues.append(
            f"line {task.line_no}: dependency ordering is ambiguous ({detail}); move prerequisite tasks earlier or add an explicit Prerequisite note"
        )
    return issues


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("change", help="OpenSpec change directory or name")
    parser.add_argument("--repo-root", default=".")
    parser.add_argument("--max-implementation-fanout", type=int, default=3)
    args = parser.parse_args()

    repo_root = pathlib.Path(args.repo_root).resolve()
    change_dir = resolve_change(repo_root, args.change)
    tasks_path = change_dir / "tasks.md"
    if not tasks_path.exists():
        print(f"OK: no tasks.md in {change_dir}")
        return 0

    issues: list[str] = []
    for task in parse_tasks(tasks_path):
        issues.extend(check_fanout(task, args.max_implementation_fanout))
        issues.extend(check_dependencies(task))

    if issues:
        fail("task size/order checks failed:\n  - " + "\n  - ".join(issues))

    print("OK: OpenSpec tasks are bounded and dependency-aware")
    print(f"  max implementation fan-out: {args.max_implementation_fanout}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
