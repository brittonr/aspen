#!/usr/bin/env python3
"""Check final-response completion claims against cited command evidence."""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

CLAIM_FAMILIES = [
    (
        "clean-status",
        re.compile(r"\b(?:working tree|worktree|git status|repo(?:sitory)?)\b[^.\n]*(?:clean|nothing to commit)|\bclean working tree\b", re.I),
        re.compile(r"git status --short|git status --porcelain|nothing to commit|working tree clean", re.I),
    ),
    (
        "queue-empty",
        re.compile(r"\b(?:openspec\s+)?(?:queue|active changes?)\b[^.\n]*(?:empty|drained|none remain|no active)|\bdrain queue is empty\b", re.I),
        re.compile(r"openspec_helper\.py drain-plan|drain-plan|active_changes:\s*none|no active OpenSpec change", re.I),
    ),
    (
        "checks-pass",
        re.compile(r"\b(?:all|targeted|relevant)?\s*(?:checks|tests|preflight|validation)\b[^.\n]*(?:pass(?:ed|es)?|green|successful)", re.I),
        re.compile(r"exit(?:ed)?\s*0|\bOK:|passed|valid\"?:\s*true|summary", re.I),
    ),
    (
        "archived",
        re.compile(r"\b(?:archived|archive(?:d)?\s+as|moved\s+to\s+archive)\b", re.I),
        re.compile(r"openspec archive|archived as|changes/archive/\d{4}-\d{2}-\d{2}-", re.I),
    ),
    (
        "validated",
        re.compile(r"\b(?:openspec\s+)?validat(?:e|ed|ion)\b[^.\n]*(?:pass(?:ed|es)?|successful|valid)", re.I),
        re.compile(r"openspec validate|\"valid\"\s*:\s*true", re.I),
    ),
]

UNCERTAIN = re.compile(r"\b(?:not verified|not checked|not run|unable to verify|blocked|unknown|remaining risk|did not run)\b", re.I)


def read_path(path: str) -> str:
    return Path(path).read_text(errors="replace")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--response", required=True, help="final response text file")
    parser.add_argument("--evidence", action="append", default=[], help="command transcript/evidence file; repeatable")
    args = parser.parse_args()

    response = read_path(args.response)
    evidence = "\n".join(read_path(path) for path in args.evidence)

    findings = []
    uncertain = bool(UNCERTAIN.search(response))
    for family, claim_re, evidence_re in CLAIM_FAMILIES:
        if not claim_re.search(response):
            continue
        if evidence_re.search(evidence):
            print(f"PASS: {family} claim has matching evidence")
            continue
        if uncertain:
            print(f"WARN: {family} claim appears in an uncertainty-scoped response")
            continue
        findings.append(f"unsupported completion claim: {family}")

    if findings:
        for finding in findings:
            print(f"FAIL: {finding}", file=sys.stderr)
        return 1

    if not any(claim_re.search(response) for _, claim_re, _ in CLAIM_FAMILIES):
        print("OK: no high-risk completion claims detected")
    else:
        print("OK: completion claims are evidence-backed or uncertainty-scoped")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
