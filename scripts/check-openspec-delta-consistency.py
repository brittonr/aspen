#!/usr/bin/env python3
"""Check OpenSpec delta specs for structural consistency.

This is intentionally dependency-light so preflight and fixture tests can run in
minimal repos. It validates the currently relevant invariants:

* every added requirement and added scenario has an ID line;
* every modified/removed requirement targets an existing main-spec requirement
  by heading or ID;
* a modified target may be missing only when the modified requirement block has
  an explicit `Migration note:` rationale.
"""

from __future__ import annotations

import argparse
import dataclasses
import pathlib
import re
import sys
from typing import Iterable


SECTION_RE = re.compile(r"^##\s+(ADDED|MODIFIED|REMOVED)\s+Requirements\s*$")
REQ_RE = re.compile(r"^###\s+Requirement:\s+(.+?)\s*$")
SCENARIO_RE = re.compile(r"^####\s+Scenario:\s+(.+?)\s*$")
ID_RE = re.compile(r"^ID:\s*(\S+)\s*$")
FEATURE_PHRASES = (
    "default feature",
    "default features",
    "no-default-features",
    "optional feature",
    "feature gate",
    "feature-gated",
    "feature contract",
)


@dataclasses.dataclass(frozen=True)
class Issue:
    severity: str
    path: pathlib.Path
    message: str

    def format(self, root: pathlib.Path) -> str:
        try:
            rel = self.path.relative_to(root)
        except ValueError:
            rel = self.path
        return f"{self.severity}: {rel}: {self.message}"


@dataclasses.dataclass(frozen=True)
class Block:
    kind: str
    heading: str
    heading_line: int
    text: str
    id: str | None


def slug_heading(heading: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", heading.lower()).strip("-")


def parse_main_requirements(path: pathlib.Path) -> tuple[set[str], set[str]]:
    if not path.exists():
        return set(), set()
    headings: set[str] = set()
    ids: set[str] = set()
    current_start = 0
    current_heading: str | None = None
    current_lines: list[str] = []

    def flush() -> None:
        nonlocal current_heading, current_lines
        if current_heading is None:
            return
        headings.add(current_heading)
        headings.add(slug_heading(current_heading))
        for line in current_lines:
            m = ID_RE.match(line.strip())
            if m:
                ids.add(m.group(1))
                break

    for line_no, raw in enumerate(path.read_text(errors="replace").splitlines(), start=1):
        m = REQ_RE.match(raw)
        if m:
            flush()
            current_start = line_no
            current_heading = m.group(1).strip()
            current_lines = []
        elif current_heading is not None:
            current_lines.append(raw)
    _ = current_start
    flush()
    return headings, ids


def iter_delta_blocks(path: pathlib.Path) -> Iterable[Block]:
    lines = path.read_text(errors="replace").splitlines()
    section: str | None = None
    i = 0
    while i < len(lines):
        section_match = SECTION_RE.match(lines[i])
        if section_match:
            section = section_match.group(1)
            i += 1
            continue
        req_match = REQ_RE.match(lines[i])
        if section and req_match:
            start = i
            i += 1
            body: list[str] = []
            while i < len(lines):
                if SECTION_RE.match(lines[i]) or REQ_RE.match(lines[i]):
                    break
                body.append(lines[i])
                i += 1
            block_text = "\n".join(body)
            block_id = None
            for body_line in body:
                id_match = ID_RE.match(body_line.strip())
                if id_match:
                    block_id = id_match.group(1)
                    break
            yield Block(section, req_match.group(1).strip(), start + 1, block_text, block_id)
            continue
        i += 1


def has_id_before_next_heading(text: str) -> bool:
    for line in text.splitlines():
        if line.startswith("### ") or line.startswith("#### "):
            return False
        if ID_RE.match(line.strip()):
            return True
    return False


def check_added_ids(path: pathlib.Path) -> list[Issue]:
    issues: list[Issue] = []
    lines = path.read_text(errors="replace").splitlines()
    section: str | None = None
    for idx, line in enumerate(lines):
        section_match = SECTION_RE.match(line)
        if section_match:
            section = section_match.group(1)
            continue
        if section != "ADDED":
            continue
        if REQ_RE.match(line) or SCENARIO_RE.match(line):
            next_lines: list[str] = []
            for following in lines[idx + 1 :]:
                if following.startswith("### ") or following.startswith("#### ") or SECTION_RE.match(following):
                    break
                next_lines.append(following)
            if not has_id_before_next_heading("\n".join(next_lines)):
                issues.append(Issue("ERROR", path, f"missing ID after line {idx + 1}: {line.strip()}"))
    return issues


def check_target_existence(change_spec: pathlib.Path, main_spec: pathlib.Path) -> list[Issue]:
    headings, ids = parse_main_requirements(main_spec)
    issues: list[Issue] = []
    for block in iter_delta_blocks(change_spec):
        if block.kind not in {"MODIFIED", "REMOVED"}:
            continue
        target_found = False
        if block.id and block.id in ids:
            target_found = True
        if block.heading in headings or slug_heading(block.heading) in headings:
            target_found = True
        if target_found:
            continue
        if block.kind == "MODIFIED" and "Migration note:" in block.text:
            issues.append(
                Issue(
                    "WARNING",
                    change_spec,
                    f"modified requirement at line {block.heading_line} uses Migration note for missing target: {block.id or block.heading}",
                )
            )
            continue
        issues.append(
            Issue(
                "ERROR",
                change_spec,
                f"{block.kind.lower()} requirement at line {block.heading_line} has no target in {main_spec}: {block.id or block.heading}",
            )
        )
    return issues


def check_feature_phrase_conflicts(change_dir: pathlib.Path) -> list[Issue]:
    artifacts = [change_dir / "proposal.md", change_dir / "design.md"] + sorted((change_dir / "specs").glob("**/*.md"))
    seen: dict[str, list[pathlib.Path]] = {}
    for artifact in artifacts:
        if not artifact.exists():
            continue
        lower = artifact.read_text(errors="replace").lower()
        for phrase in FEATURE_PHRASES:
            if phrase in lower:
                seen.setdefault(phrase, []).append(artifact)
    warnings: list[Issue] = []
    positive_default = any(p in seen for p in ("default feature", "default features"))
    negative_default = "no-default-features" in seen
    if positive_default and negative_default:
        paths = sorted({p for ps in seen.values() for p in ps})
        warnings.append(
            Issue(
                "WARNING",
                change_dir,
                "potentially conflicting feature-contract phrases across artifacts: "
                + ", ".join(str(p) for p in paths),
            )
        )
    return warnings


def check_change(repo_root: pathlib.Path, change_dir: pathlib.Path) -> list[Issue]:
    issues: list[Issue] = []
    specs_dir = change_dir / "specs"
    for change_spec in sorted(specs_dir.glob("**/*.md")):
        relative = change_spec.relative_to(specs_dir)
        main_spec = repo_root / "openspec" / "specs" / relative
        issues.extend(check_added_ids(change_spec))
        issues.extend(check_target_existence(change_spec, main_spec))
    issues.extend(check_feature_phrase_conflicts(change_dir))
    return issues


def resolve_change(repo_root: pathlib.Path, value: str) -> pathlib.Path:
    candidate = pathlib.Path(value)
    if candidate.is_dir():
        return candidate.resolve()
    for base in (repo_root / "openspec" / "changes", repo_root / "openspec" / "changes" / "active"):
        candidate = base / value
        if candidate.is_dir():
            return candidate.resolve()
    raise SystemExit(f"error: could not resolve change {value!r}")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("change", help="OpenSpec change name or directory")
    parser.add_argument("--repo-root", default=".", help="repository root")
    parser.add_argument("--warnings-as-errors", action="store_true")
    args = parser.parse_args(argv)

    repo_root = pathlib.Path(args.repo_root).resolve()
    change_dir = resolve_change(repo_root, args.change)
    issues = check_change(repo_root, change_dir)
    errors = [issue for issue in issues if issue.severity == "ERROR"]
    warnings = [issue for issue in issues if issue.severity == "WARNING"]
    for issue in issues:
        print(issue.format(repo_root))
    if errors or (args.warnings_as_errors and warnings):
        return 1
    print(f"OK: {change_dir.relative_to(repo_root).as_posix()} delta specs are consistent")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
