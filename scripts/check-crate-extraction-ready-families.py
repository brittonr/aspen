#!/usr/bin/env python3
"""Run crate-extraction readiness checks for the repo's ready extraction families.

This is a thin repo check around ``check-crate-extraction-readiness.rs``.  The
per-family checker expects an OpenSpec-style evidence directory; this script
creates an isolated temporary evidence fixture so broad sweeps do not require
ad-hoc placeholder files in the working tree.
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


DEFAULT_READY_FAMILIES = [
    "foundational-types",
    "auth-ticket",
    "jobs-ci-core",
    "trust-crypto-secrets",
    "testing-harness",
    "protocol-wire",
    "blob-castore-cache",
    "coordination",
]

REQUIRED_EVIDENCE_BY_FAMILY = {
    "foundational-types": [
        "foundational-types-downstream-metadata.json",
        "foundational-types-forbidden-boundary.txt",
        "foundational-types-compatibility.txt",
    ],
    "auth-ticket": [
        "auth-ticket-downstream-metadata.json",
        "auth-ticket-forbidden-boundary.txt",
        "auth-ticket-compatibility.txt",
    ],
    "jobs-ci-core": [
        "jobs-ci-core-downstream-metadata.json",
        "jobs-ci-core-forbidden-boundary.txt",
        "jobs-ci-core-compatibility.txt",
    ],
    "trust-crypto-secrets": [
        "trust-crypto-secrets-downstream-metadata.json",
        "trust-crypto-secrets-forbidden-boundary.txt",
        "trust-crypto-secrets-compatibility.txt",
    ],
    "testing-harness": [
        "testing-harness-downstream-metadata.json",
        "testing-harness-forbidden-boundary.txt",
        "testing-harness-compatibility.txt",
    ],
    "protocol-wire": [
        "i5-downstream-protocol-wire-metadata.json",
        "i5-downstream-protocol-wire-forbidden-grep.txt",
        "i3-client-api-compatibility-tests.txt",
    ],
    "blob-castore-cache": [
        "i6-downstream-blob-metadata.json",
        "i6-downstream-cache-castore-metadata.json",
        "i6-downstream-blob-forbidden-grep.txt",
        "i6-downstream-cache-castore-forbidden-grep.txt",
    ],
}


def repo_root() -> Path:
    output = subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True)
    return Path(output.strip())


def write_fixture(change_dir: Path, family: str) -> Path:
    evidence_dir = change_dir / "evidence"
    evidence_dir.mkdir(parents=True, exist_ok=True)
    (change_dir / "verification.md").write_text(
        "# Temporary crate-extraction readiness sweep fixture\n\n"
        "## Task Coverage\n\n"
        f"- Evidence: generated temporary fixture for `{family}` readiness sweep.\n",
        encoding="utf-8",
    )
    for name in REQUIRED_EVIDENCE_BY_FAMILY.get(family, []):
        payload = (
            f"temporary fixture for scripts/check-crate-extraction-ready-families.py; "
            f"family={family}; artifact={name}\n"
        )
        (evidence_dir / name).write_text(payload, encoding="utf-8")
    return evidence_dir


def run_family(root: Path, family: str, work_dir: Path, args: argparse.Namespace) -> dict[str, object]:
    change_dir = work_dir / family
    evidence_dir = write_fixture(change_dir, family)
    output_json = evidence_dir / "readiness.json"
    output_markdown = evidence_dir / "readiness.md"
    command = [
        str(root / "scripts" / "check-crate-extraction-readiness.rs"),
        "--policy",
        str(args.policy),
        "--inventory",
        str(args.inventory),
        "--manifest-dir",
        str(args.manifest_dir),
        "--candidate-family",
        family,
        "--output-json",
        str(output_json),
        "--output-markdown",
        str(output_markdown),
    ]
    completed = subprocess.run(command, cwd=root, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    report: dict[str, object]
    if output_json.exists():
        report = json.loads(output_json.read_text(encoding="utf-8"))
    else:
        report = {
            "candidate_family": family,
            "passed": False,
            "failures": ["readiness checker did not write JSON report"],
            "warnings": [],
            "checked_candidates": [],
        }
    report["exit_code"] = completed.returncode
    report["stderr"] = completed.stderr.strip()
    if completed.returncode != 0 and completed.stderr.strip():
        failures = list(report.get("failures", []))
        failures.append(completed.stderr.strip())
        report["failures"] = failures
    return report


def parse_args() -> argparse.Namespace:
    root = repo_root()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--family",
        action="append",
        dest="families",
        help="family to check; may be repeated. Defaults to all ready extraction families.",
    )
    parser.add_argument("--policy", type=Path, default=root / "docs" / "crate-extraction" / "policy.ncl")
    parser.add_argument("--inventory", type=Path, default=root / "docs" / "crate-extraction.md")
    parser.add_argument("--manifest-dir", type=Path, default=root / "docs" / "crate-extraction")
    parser.add_argument(
        "--keep-temp",
        action="store_true",
        help="keep the temporary evidence fixture and print its path",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    root = repo_root()
    families = args.families or DEFAULT_READY_FAMILIES

    with tempfile.TemporaryDirectory(prefix="aspen-crate-extraction-sweep-") as tmp:
        work_dir = Path(tmp)
        reports = [run_family(root, family, work_dir, args) for family in families]
        failed = [report for report in reports if not report.get("passed") or report.get("exit_code") != 0]

        for report in reports:
            family = report["candidate_family"]
            checked = len(report.get("checked_candidates", []))
            warnings = len(report.get("warnings", []))
            status = "PASS" if report.get("passed") and report.get("exit_code") == 0 else "FAIL"
            print(f"{status} {family} checked={checked} warnings={warnings}")
            if status == "FAIL":
                for failure in report.get("failures", []):
                    print(f"  - {failure}")

        if args.keep_temp:
            keep_path = root / "target" / "crate-extraction-ready-family-sweep"
            if keep_path.exists():
                shutil.rmtree(keep_path)
            shutil.copytree(work_dir, keep_path)
            print(f"kept temporary evidence fixture at {keep_path}")

    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
