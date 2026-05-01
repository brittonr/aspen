#!/usr/bin/env python3
"""Validate OpenSpec verification index freshness."""
from __future__ import annotations

import argparse
import pathlib
import re
import sys

PATH_RE = re.compile(r"`([^`]+)`")
ACTIVE_PATH_RE = re.compile(r"openspec/changes/(?!archive/|_done/)([A-Za-z0-9_.-]+)/")
PLACEHOLDER_RE = re.compile(r"\b(pending|placeholder|will be replaced|todo)\b", re.IGNORECASE)
FINAL_ARTIFACT_RE = re.compile(r"(preflight|final-diff|implementation-diff|final)", re.IGNORECASE)
DIFF_ARTIFACT_RE = re.compile(r"(diff|patch)\.(txt|md|patch)$|diff|patch", re.IGNORECASE)


def fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


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


def is_archived(change_dir: pathlib.Path) -> bool:
    parts = change_dir.parts
    return "archive" in parts and "changes" in parts


def active_name_from_archive(change_dir: pathlib.Path) -> str | None:
    name = change_dir.name
    match = re.match(r"\d{4}-\d{2}-\d{2}-(.+)", name)
    return match.group(1) if match else None


def paths_in_index(text: str) -> list[str]:
    return [m.group(1) for m in PATH_RE.finditer(text) if "/evidence/" in m.group(1)]


def check_stale_active_paths(change_dir: pathlib.Path, files: list[pathlib.Path]) -> list[str]:
    issues: list[str] = []
    archived_name = active_name_from_archive(change_dir) if is_archived(change_dir) else None
    for path in files:
        if not path.exists() or DIFF_ARTIFACT_RE.search(path.name):
            continue
        text = path.read_text(errors="replace")
        for match in ACTIVE_PATH_RE.finditer(text):
            active_name = match.group(1)
            if archived_name and active_name == archived_name:
                issues.append(f"{path}: stale active path {match.group(0)}")
    return issues


def check_preflight_artifacts(repo_root: pathlib.Path, index_text: str) -> list[str]:
    issues: list[str] = []
    for rel in paths_in_index(index_text):
        if "preflight" not in pathlib.Path(rel).name.lower():
            continue
        path = repo_root / rel
        if not path.exists():
            continue
        text = path.read_text(errors="replace").strip()
        first = text.splitlines()[0].strip() if text else ""
        if not first or PLACEHOLDER_RE.search(text) or not (first.startswith("OK:") or first.startswith("FAIL:")):
            issues.append(f"{rel}: preflight artifact is placeholder or not a final transcript")
    return issues


def check_final_artifact_mtime(repo_root: pathlib.Path, index_text: str) -> list[str]:
    issues: list[str] = []
    changed = [repo_root / p for p in re.findall(r"^\s*-\s*Changed file:\s*`([^`]+)`\s*$", index_text, re.MULTILINE)]
    changed = [p for p in changed if p.exists()]
    source_changed = [p for p in changed if not FINAL_ARTIFACT_RE.search(p.name)]
    if not source_changed:
        return issues
    newest_source = max(p.stat().st_mtime for p in source_changed)
    for rel in paths_in_index(index_text):
        if not FINAL_ARTIFACT_RE.search(pathlib.Path(rel).name):
            continue
        path = repo_root / rel
        if not path.exists():
            continue
        if path.stat().st_mtime + 0.0001 < newest_source:
            issues.append(f"{rel}: final evidence appears older than changed files; regenerate after final edits are staged")
    return issues


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("change", help="OpenSpec change directory or name")
    parser.add_argument("--repo-root", default=".")
    args = parser.parse_args()

    repo_root = pathlib.Path(args.repo_root).resolve()
    change_dir = resolve_change(repo_root, args.change)
    if not change_dir.exists():
        fail(f"change directory not found: {change_dir}")

    index_files = [change_dir / "verification.md", change_dir / "tasks.md"]
    existing = [p for p in index_files if p.exists()]
    if not existing:
        print(f"OK: no verification index files in {change_dir}")
        return 0

    index_text = "\n".join(p.read_text(errors="replace") for p in existing)
    issues: list[str] = []
    issues.extend(check_stale_active_paths(change_dir, existing))
    issues.extend(check_preflight_artifacts(repo_root, index_text))
    issues.extend(check_final_artifact_mtime(repo_root, index_text))

    if issues:
        fail("verification index freshness checks failed:\n  - " + "\n  - ".join(issues))

    print("OK: verification index is fresh")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
