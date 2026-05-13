#!/usr/bin/env python3
"""Run Aspen's bounded quick confidence rail.

This rail intentionally composes cheap/local checks only. It is not a
replacement for full dogfood, nix flake check, or gated runtime-host proofs.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from dataclasses import asdict, dataclass
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parent.parent
SUMMARY_PATH = REPO_ROOT / "target" / "quick-confidence" / "summary.json"


@dataclass(frozen=True)
class Check:
    name: str
    command: list[str]
    diagnostic: str


@dataclass
class CheckResult:
    name: str
    command: str
    status: str
    exit_status: int | None
    elapsed_ms: int
    diagnostic: str


CHECKS: tuple[Check, ...] = (
    Check(
        name="harness-inventory",
        command=["scripts/test-harness.sh", "check"],
        diagnostic="Run scripts/test-harness.sh export, inspect test-harness/suites, and rerun scripts/test-harness.sh check.",
    ),
    Check(
        name="runtime-host-acceptance-bundle",
        command=["scripts/test-harness.sh", "runtime-host-acceptance-bundle"],
        diagnostic="Inspect docs/runtime-host-readiness.md, test-harness/generated/inventory.json, and scripts/check-runtime-host-acceptance-bundle.py.",
    ),
    Check(
        name="testing-public-api-boundary",
        command=["scripts/test-harness.sh", "public-api-boundary"],
        diagnostic="Inspect crates/aspen-testing/Cargo.toml and scripts/check-aspen-testing-public-api-boundary.py.",
    ),
    Check(
        name="verus-trusted-boundaries",
        command=["scripts/test-harness.sh", "verus-trusted-boundaries"],
        diagnostic="Inspect docs/verus-trusted-boundaries.md and scripts/check-verus-trusted-boundaries.py.",
    ),
    Check(
        name="operator-receipts-docs",
        command=["cargo", "test", "--test", "operator_receipts_docs", "--", "--nocapture"],
        diagnostic="Inspect docs/operator-receipts.md and tests/operator_receipts_docs.rs.",
    ),
    Check(
        name="runtime-host-readiness-docs",
        command=["cargo", "test", "--test", "runtime_host_readiness_docs", "--", "--nocapture"],
        diagnostic="Inspect docs/runtime-host-readiness.md and tests/runtime_host_readiness_docs.rs.",
    ),
    Check(
        name="openspec-all-strict",
        command=["openspec", "validate", "--all", "--strict", "--json"],
        diagnostic="Inspect the failing OpenSpec item, then rerun openspec validate <item> --strict --json.",
    ),
    Check(
        name="whitespace-diff",
        command=["git", "diff", "--check"],
        diagnostic="Fix reported trailing whitespace or EOF issues, then rerun git diff --check.",
    ),
)

SKIPPED_GATED_PROOFS: tuple[str, ...] = (
    "full dogfood/self-hosting acceptance (`nix run .#dogfood-local -- full`)",
    "KVM/NixOS VM runtime-host proofs",
    "Uhyve/Hermit runtime-host execution proofs",
    "Hyperlight runtime-host execution proofs",
    "network/ignored nextest profiles and full `nix flake check`",
)


def command_text(command: list[str]) -> str:
    return " ".join(command)


def run_check(check: Check, dry_run: bool) -> CheckResult:
    start = time.monotonic()
    if dry_run:
        return CheckResult(
            name=check.name,
            command=command_text(check.command),
            status="planned",
            exit_status=None,
            elapsed_ms=0,
            diagnostic=check.diagnostic,
        )

    completed = subprocess.run(check.command, cwd=REPO_ROOT)
    elapsed_ms = int((time.monotonic() - start) * 1000)
    return CheckResult(
        name=check.name,
        command=command_text(check.command),
        status="passed" if completed.returncode == 0 else "failed",
        exit_status=completed.returncode,
        elapsed_ms=elapsed_ms,
        diagnostic=check.diagnostic,
    )


def render_text(summary: dict[str, object]) -> str:
    lines = [
        "Aspen quick confidence rail",
        f"mode: {summary['mode']}",
        f"status: {summary['status']}",
        "",
        "Included checks:",
    ]
    for result in summary["checks"]:  # type: ignore[index]
        exit_status = result["exit_status"]  # type: ignore[index]
        exit_text = "n/a" if exit_status is None else str(exit_status)
        lines.append(
            f"- {result['status']}: {result['name']} "
            f"(exit={exit_text}, elapsed_ms={result['elapsed_ms']})"
        )
        lines.append(f"  command: `{result['command']}`")
        if result["status"] == "failed":
            lines.append(f"  next: {result['diagnostic']}")
    lines.extend(["", "Skipped gated proofs / non-proof boundaries:"])
    for skipped in summary["skipped_gated_proofs"]:  # type: ignore[index]
        lines.append(f"- skipped: {skipped}")
    lines.extend(
        [
            "",
            "Boundary: this quick rail does not claim runtime-host acceptance, full dogfood acceptance, or production readiness.",
            f"JSON summary: {summary['summary_path']}",
        ]
    )
    return "\n".join(lines) + "\n"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run Aspen's bounded quick confidence rail")
    parser.add_argument("--dry-run", action="store_true", help="render the selected checks without executing them")
    parser.add_argument("--json", action="store_true", help="print JSON summary instead of text")
    parser.add_argument("--summary", default=str(SUMMARY_PATH), help="summary JSON output path")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    summary_path = Path(args.summary)
    if not summary_path.is_absolute():
        summary_path = REPO_ROOT / summary_path

    results = [run_check(check, args.dry_run) for check in CHECKS]
    failed = [result for result in results if result.status == "failed"]
    status = "planned" if args.dry_run else ("failed" if failed else "passed")
    summary = {
        "schema": "aspen.quick-confidence.v1",
        "mode": "dry-run" if args.dry_run else "run",
        "status": status,
        "checks": [asdict(result) for result in results],
        "skipped_gated_proofs": list(SKIPPED_GATED_PROOFS),
        "non_proof_boundary": "Quick confidence does not prove runtime-host acceptance, full dogfood acceptance, or production readiness.",
        "summary_path": str(summary_path),
    }

    summary_path.parent.mkdir(parents=True, exist_ok=True)
    summary_path.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")

    if args.json:
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        print(render_text(summary))

    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
