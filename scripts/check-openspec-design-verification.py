#!/usr/bin/env python3
"""Validate OpenSpec design verification strategies for spec-changing changes."""
from __future__ import annotations

import argparse
import pathlib
import re
import sys

ID_RE = re.compile(r"\b[a-z0-9][a-z0-9_.-]*(?:\.[a-z0-9][a-z0-9_.-]*)+\b")
HEADING_RE = re.compile(r"^##\s+(.+?)\s*$", re.MULTILINE)
INLINE_ID_RE = re.compile(r"\[r\[([^\]]+)\]\]")
BODY_ID_RE = re.compile(r"^\s*ID:\s*([A-Za-z0-9_.-]+)\s*$", re.MULTILINE)
NEGATIVE_RE = re.compile(
    r"\b(reject(?:s|ion|ed)?|fail(?:s|ure|ed)?|timeout|malformed|unauthori[sz]ed|forbidden|invalid|missing|den(?:y|ied)|error)\b",
    re.IGNORECASE,
)
NEGATIVE_STRATEGY_RE = re.compile(
    r"\b(negative|reject(?:s|ion|ed)?|fail(?:s|ure|ed)?|timeout|malformed|unauthori[sz]ed|forbidden|invalid|missing|den(?:y|ied)|error|defer(?:red)?\b.*\brationale)\b",
    re.IGNORECASE | re.DOTALL,
)


def fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def section(text: str, heading: str) -> str | None:
    pattern = re.compile(rf"^##\s+{re.escape(heading)}\s*$\n(.*?)(?=^##\s+|\Z)", re.MULTILINE | re.DOTALL)
    match = pattern.search(text)
    return match.group(1).strip() if match else None


def collect_spec_ids(spec_text: str) -> set[str]:
    ids = set(INLINE_ID_RE.findall(spec_text))
    ids.update(BODY_ID_RE.findall(spec_text))
    # Also accept existing domain-like references in body text.
    ids.update(m.group(0) for m in ID_RE.finditer(spec_text) if "." in m.group(0))
    return ids


def has_specs(change_dir: pathlib.Path) -> bool:
    specs = change_dir / "specs"
    return specs.exists() and any(p.is_file() and p.suffix == ".md" for p in specs.glob("**/*.md"))


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

    if not change_dir.exists():
        fail(f"change directory not found: {change_dir}")
    if not has_specs(change_dir):
        print(f"OK: no delta specs in {change_dir}")
        return 0

    design_path = change_dir / "design.md"
    if not design_path.exists():
        fail(f"spec-changing change is missing design.md: {design_path}")
    design_text = design_path.read_text()
    strategy = section(design_text, "Verification Strategy")
    if not strategy:
        fail("design.md must include ## Verification Strategy for spec-changing changes")

    spec_ids: set[str] = set()
    negative_specs: list[pathlib.Path] = []
    for spec_path in sorted((change_dir / "specs").glob("**/*.md")):
        text = spec_path.read_text()
        spec_ids.update(collect_spec_ids(text))
        if NEGATIVE_RE.search(text):
            negative_specs.append(spec_path)

    cited_ids = {sid for sid in spec_ids if sid in strategy}
    if not cited_ids:
        fail("Verification Strategy must cite at least one changed requirement or scenario ID from specs/**/*.md")

    if negative_specs and not NEGATIVE_STRATEGY_RE.search(strategy):
        rel = ", ".join(str(p.relative_to(change_dir)) for p in negative_specs)
        fail(
            "Verification Strategy must name a negative/failure check or explicit defer-with-rationale entry "
            f"because changed specs define negative behavior: {rel}"
        )

    print("OK: design verification strategy is mapped")
    print(f"  cited ids: {', '.join(sorted(cited_ids))}")
    if negative_specs:
        print("  negative behavior: covered")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
