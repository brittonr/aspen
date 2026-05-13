#!/usr/bin/env python3
"""Check Aspen's residual Verus trusted-boundary inventory.

This guard intentionally matches only source-level `#[verifier(external_body)]`
attributes in crate-local Verus models. The expected inventory mirrors
`docs/verus-trusted-boundaries.md` and should change only when the reviewer
boundary document is updated with new evidence.
"""

from __future__ import annotations

import re
import sys
from collections import Counter
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
PATTERN = re.compile(r"#\[verifier(?:::|\()external_body")

EXPECTED: dict[str, int] = {
    "crates/aspen-commit-dag/verus/commit_hash_spec.rs": 1,
    "crates/aspen-core/verus/tuple_spec.rs": 6,
    "crates/aspen-raft/verus/chain_verify_spec.rs": 2,
    "crates/aspen-secrets/verus/mac_spec.rs": 2,
}

DOC_PATH = REPO_ROOT / "docs" / "verus-trusted-boundaries.md"
REQUIRED_DOC_MARKERS = (
    "Total: 11 source-level residual `external_body` attributes across 4 Verus files.",
    "rg -n '#\\[verifier(?:::|\\()external_body' crates/*/verus/*.rs",
    "crates/aspen-core/verus/tuple_spec.rs",
    "crates/aspen-raft/verus/chain_verify_spec.rs",
    "crates/aspen-secrets/verus/mac_spec.rs",
    "crates/aspen-commit-dag/verus/commit_hash_spec.rs",
)


def relative(path: Path) -> str:
    return path.relative_to(REPO_ROOT).as_posix()


def scan_inventory() -> Counter[str]:
    counts: Counter[str] = Counter()
    crates_dir = REPO_ROOT / "crates"
    for path in crates_dir.glob("*/verus/*.rs"):
        text = path.read_text(errors="ignore")
        count = len(PATTERN.findall(text))
        if count:
            counts[relative(path)] = count
    return counts


def format_inventory(counts: dict[str, int]) -> str:
    return "\n".join(f"{count}  {path}" for path, count in sorted(counts.items()))


def check_docs() -> list[str]:
    errors: list[str] = []
    doc = DOC_PATH.read_text(errors="ignore")
    for marker in REQUIRED_DOC_MARKERS:
        if marker not in doc:
            errors.append(f"missing docs marker: {marker}")
    return errors


def main() -> int:
    actual = scan_inventory()
    expected = Counter(EXPECTED)
    errors = []

    if actual != expected:
        errors.append("trusted-boundary inventory drift")
        errors.append("expected:\n" + format_inventory(expected))
        errors.append("actual:\n" + format_inventory(actual))

    errors.extend(check_docs())

    if errors:
        print("Verus trusted-boundary check failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    total = sum(actual.values())
    print(
        f"Verus trusted-boundary inventory OK: {total} residual external_body attributes across {len(actual)} files"
    )
    print(format_inventory(actual))
    return 0


if __name__ == "__main__":
    sys.exit(main())
