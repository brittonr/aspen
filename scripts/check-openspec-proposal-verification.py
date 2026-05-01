#!/usr/bin/env python3
"""Validate OpenSpec proposal verification traceability."""
from __future__ import annotations

import argparse
import pathlib
import re
import sys

INLINE_ID_RE = re.compile(r"\[r\[([^\]]+)\]\]")
BODY_ID_RE = re.compile(r"^\s*ID:\s*([A-Za-z0-9_.-]+)\s*$", re.MULTILINE)
SECTION_RE = re.compile(r"^##\s+(.+?)\s*$", re.MULTILINE)
NEGATIVE_RE = re.compile(
    r"\b(negative|reject(?:s|ion|ed)?|fail(?:s|ure|ed)?|timeout|malformed|unauthori[sz]ed|forbidden|invalid|missing|den(?:y|ied)|error)\b",
    re.IGNORECASE,
)
RATIONALE_RE = re.compile(r"\b(because|rationale|reason|until|blocked|depends|pending|follow-up|followup)\b", re.IGNORECASE)
DEFER_RE = re.compile(r"\bdefer(?:red|s|ring)?\s+to\s+design\b", re.IGNORECASE)
VERIFICATION_HEADINGS = {
    "verification",
    "verification expectations",
    "proposal verification",
    "testing",
}


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


def has_specs(change_dir: pathlib.Path) -> bool:
    specs = change_dir / "specs"
    return specs.exists() and any(p.is_file() and p.suffix == ".md" for p in specs.glob("**/*.md"))


def collect_spec_ids(change_dir: pathlib.Path) -> set[str]:
    ids: set[str] = set()
    for path in sorted((change_dir / "specs").glob("**/*.md")):
        text = path.read_text(errors="replace")
        ids.update(INLINE_ID_RE.findall(text))
        ids.update(BODY_ID_RE.findall(text))
    return ids


def collect_section(text: str) -> str | None:
    matches = list(SECTION_RE.finditer(text))
    for idx, match in enumerate(matches):
        heading = match.group(1).strip().lower()
        if heading in VERIFICATION_HEADINGS or heading.startswith("verification "):
            start = match.end()
            end = matches[idx + 1].start() if idx + 1 < len(matches) else len(text)
            body = text[start:end].strip()
            return body or None
    return None


def invalid_deferrals(section: str, known_ids: set[str]) -> list[str]:
    bad: list[str] = []
    for line in section.splitlines():
        if not DEFER_RE.search(line):
            continue
        cited = [spec_id for spec_id in known_ids if spec_id in line]
        if not cited or not RATIONALE_RE.search(line):
            bad.append(line.strip())
    return bad


def valid_deferred_ids(section: str, known_ids: set[str]) -> set[str]:
    deferred: set[str] = set()
    for line in section.splitlines():
        if not DEFER_RE.search(line) or not RATIONALE_RE.search(line):
            continue
        deferred.update(spec_id for spec_id in known_ids if spec_id in line)
    return deferred


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("change", help="OpenSpec change directory or name")
    parser.add_argument("--repo-root", default=".")
    args = parser.parse_args()

    repo_root = pathlib.Path(args.repo_root).resolve()
    change_dir = resolve_change(repo_root, args.change)
    if not change_dir.exists():
        fail(f"change directory not found: {change_dir}")
    if not has_specs(change_dir):
        print(f"OK: no delta specs in {change_dir}")
        return 0

    proposal_path = change_dir / "proposal.md"
    if not proposal_path.exists():
        fail(f"spec-changing change is missing proposal.md: {proposal_path}")
    proposal_text = proposal_path.read_text(errors="replace")
    section = collect_section(proposal_text)
    if not section:
        fail("proposal.md must include a ## Verification Expectations section for spec-changing changes")

    spec_ids = collect_spec_ids(change_dir)
    if not spec_ids:
        print("OK: no requirement/scenario IDs found in delta specs")
        return 0

    bad_deferrals = invalid_deferrals(section, spec_ids)
    if bad_deferrals:
        fail(
            "defer to design entries must cite a changed requirement/scenario ID and rationale:\n  - "
            + "\n  - ".join(bad_deferrals)
        )

    deferred = valid_deferred_ids(section, spec_ids)
    cited = {spec_id for spec_id in spec_ids if spec_id in section}
    mapped = cited | deferred
    missing = sorted(spec_ids - mapped)
    if missing:
        fail("proposal verification is missing changed requirement/scenario IDs:\n  - " + "\n  - ".join(missing))

    spec_text = "\n".join(p.read_text(errors="replace") for p in sorted((change_dir / "specs").glob("**/*.md")))
    if NEGATIVE_RE.search(proposal_text) or NEGATIVE_RE.search(spec_text):
        has_negative_expectation = bool(NEGATIVE_RE.search(section)) or any(DEFER_RE.search(line) for line in section.splitlines())
        if not has_negative_expectation:
            fail("proposal verification must list a negative-path expectation or scoped defer-to-design entry")

    print("OK: proposal verification traceability is mapped")
    print(f"  mapped ids: {', '.join(sorted(mapped))}")
    if deferred:
        print(f"  deferred ids: {', '.join(sorted(deferred))}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
