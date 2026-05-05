#!/usr/bin/env python3
"""Audit Aspen supply-chain boundary invariants.

This checker is intentionally read-only. It verifies that supply-chain inputs are
pinned by lock/hash material, git dependencies are revision pinned, and public
unsafe Rust APIs do not appear in Aspen-owned source paths. It emits a compact
JSON receipt suitable for OpenSpec evidence.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
REV_RE = re.compile(r"^[0-9a-f]{40}$")
PUBLIC_UNSAFE_RE = re.compile(
    r'\bpub\s+(?:\([^)]*\)\s+)?(?:async\s+)?(?:extern\s+"[^"]+"\s+)?unsafe\s+fn\b'
)
NIX_FETCH_RE = re.compile(r"\bfetch(?:url|FromGitHub|git|FromGitLab|Tarball)\b")
HASH_RE = re.compile(r"\b(?:hash|sha256|sha512)\s*=")

EXCLUDED_PARTS = {
    ".git",
    "target",
    ".direnv",
    "result",
}
OWNED_SOURCE_PREFIXES = (
    "src/",
    "crates/",
    "examples/",
    "tests/",
    "nix/tests/fixtures/",
)
VENDORED_PREFIXES = (
    "vendor/",
    "openraft/",
)


@dataclass(frozen=True)
class Finding:
    check: str
    path: str
    detail: str

    def to_json(self) -> dict[str, str]:
        return {"check": self.check, "path": self.path, "detail": self.detail}


def load_toml(path: Path) -> dict[str, Any]:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def repo_files(suffixes: tuple[str, ...]) -> list[Path]:
    files: list[Path] = []
    for path in ROOT.rglob("*"):
        if any(part in EXCLUDED_PARTS for part in path.relative_to(ROOT).parts):
            continue
        if path.is_file() and path.suffix in suffixes:
            files.append(path)
    return sorted(files)


def is_vendored(path: Path) -> bool:
    rel = path.relative_to(ROOT).as_posix()
    return rel.startswith(VENDORED_PREFIXES)


def is_owned_rust(path: Path) -> bool:
    rel = path.relative_to(ROOT).as_posix()
    return rel.startswith(OWNED_SOURCE_PREFIXES) and not is_vendored(path)


def check_flake_lock() -> tuple[list[Finding], dict[str, int]]:
    lock_path = ROOT / "flake.lock"
    lock = json.loads(lock_path.read_text())
    findings: list[Finding] = []
    locked_nodes = 0
    hashed_nodes = 0
    revision_nodes = 0

    for name, node in sorted(lock.get("nodes", {}).items()):
        locked = node.get("locked")
        if not locked:
            continue
        locked_nodes += 1
        node_type = locked.get("type", "")
        nar_hash = locked.get("narHash")
        rev = locked.get("rev")
        if nar_hash:
            hashed_nodes += 1
        else:
            findings.append(Finding("flake-lock-hash", "flake.lock", f"{name} lacks narHash"))
        if node_type in {"github", "git", "tarball"}:
            if rev and REV_RE.fullmatch(rev):
                revision_nodes += 1
            else:
                findings.append(Finding("flake-lock-rev", "flake.lock", f"{name} lacks 40-hex rev"))

    return findings, {
        "locked_nodes": locked_nodes,
        "hashed_nodes": hashed_nodes,
        "revision_nodes": revision_nodes,
    }


def check_cargo_git_deps() -> tuple[list[Finding], dict[str, int]]:
    cargo = load_toml(ROOT / "Cargo.toml")
    lock = load_toml(ROOT / "Cargo.lock")
    findings: list[Finding] = []
    workspace_git_deps = 0
    locked_git_packages = 0

    deps = cargo.get("workspace", {}).get("dependencies", {})
    for name, dep in sorted(deps.items()):
        if isinstance(dep, dict) and "git" in dep:
            workspace_git_deps += 1
            rev = dep.get("rev")
            if not isinstance(rev, str) or not REV_RE.fullmatch(rev):
                findings.append(Finding("cargo-git-rev", "Cargo.toml", f"{name} lacks 40-hex rev"))
            if "branch" in dep or "tag" in dep:
                findings.append(Finding("cargo-git-floating", "Cargo.toml", f"{name} uses branch/tag"))

    for package in lock.get("package", []):
        source = package.get("source", "")
        if source.startswith("git+"):
            locked_git_packages += 1
            rev = source.rsplit("#", 1)[-1]
            if not REV_RE.fullmatch(rev):
                name = package.get("name", "<unknown>")
                findings.append(Finding("cargo-lock-git-rev", "Cargo.lock", f"{name} lacks locked rev"))

    return findings, {
        "workspace_git_deps": workspace_git_deps,
        "locked_git_packages": locked_git_packages,
    }


def check_nix_fetch_hashes() -> tuple[list[Finding], dict[str, int]]:
    findings: list[Finding] = []
    fetches = 0
    hashed_fetches = 0

    for path in repo_files((".nix",)):
        rel = path.relative_to(ROOT).as_posix()
        if rel.startswith(("vendor/", "nix/tests/")):
            continue
        lines = path.read_text(errors="replace").splitlines()
        code_lines = [line.split("#", 1)[0] for line in lines]
        for index, line in enumerate(code_lines):
            if not NIX_FETCH_RE.search(line):
                continue
            fetches += 1
            window = "\n".join(code_lines[index : min(index + 24, len(code_lines))])
            if HASH_RE.search(window):
                hashed_fetches += 1
            else:
                findings.append(Finding("nix-fetch-hash", rel, f"fetch at line {index + 1} lacks nearby hash"))

    return findings, {"nix_fetches": fetches, "hashed_nix_fetches": hashed_fetches}


def check_public_unsafe_apis() -> tuple[list[Finding], dict[str, int]]:
    findings: list[Finding] = []
    owned_rust_files = 0
    unsafe_blocks = 0
    public_unsafe_apis = 0
    vendored_public_unsafe_apis = 0

    for path in repo_files((".rs",)):
        text = path.read_text(errors="replace")
        if is_owned_rust(path):
            owned_rust_files += 1
            unsafe_blocks += len(re.findall(r"\bunsafe\s*\{", text))
            for match in PUBLIC_UNSAFE_RE.finditer(text):
                public_unsafe_apis += 1
                line = text.count("\n", 0, match.start()) + 1
                rel = path.relative_to(ROOT).as_posix()
                findings.append(Finding("public-unsafe-api", rel, f"public unsafe fn at line {line}"))
        elif is_vendored(path):
            vendored_public_unsafe_apis += len(PUBLIC_UNSAFE_RE.findall(text))

    return findings, {
        "owned_rust_files": owned_rust_files,
        "owned_unsafe_blocks": unsafe_blocks,
        "owned_public_unsafe_apis": public_unsafe_apis,
        "vendored_public_unsafe_apis": vendored_public_unsafe_apis,
    }


def check_build_script_inventory() -> tuple[list[Finding], dict[str, Any]]:
    scripts = [path.relative_to(ROOT).as_posix() for path in repo_files((".rs",)) if path.name == "build.rs"]
    vendored = [path for path in scripts if path.startswith(VENDORED_PREFIXES)]
    owned = [path for path in scripts if not path.startswith(VENDORED_PREFIXES)]
    return [], {
        "owned_build_scripts": owned,
        "vendored_build_scripts": vendored,
    }


def run_audit() -> dict[str, Any]:
    all_findings: list[Finding] = []
    summary: dict[str, Any] = {}

    for check in (
        check_flake_lock,
        check_cargo_git_deps,
        check_nix_fetch_hashes,
        check_public_unsafe_apis,
        check_build_script_inventory,
    ):
        findings, metrics = check()
        all_findings.extend(findings)
        summary.update(metrics)

    return {
        "status": "pass" if not all_findings else "fail",
        "summary": summary,
        "findings": [finding.to_json() for finding in all_findings],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true", help="emit JSON receipt")
    args = parser.parse_args()

    receipt = run_audit()
    if args.json:
        print(json.dumps(receipt, indent=2, sort_keys=True))
    else:
        print(f"status: {receipt['status']}")
        print(json.dumps(receipt["summary"], indent=2, sort_keys=True))
        for finding in receipt["findings"]:
            print(f"{finding['check']}: {finding['path']}: {finding['detail']}")

    return 0 if receipt["status"] == "pass" else 1


if __name__ == "__main__":
    sys.exit(main())
