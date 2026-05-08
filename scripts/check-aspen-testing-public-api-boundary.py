#!/usr/bin/env python3
"""Check that aspen-testing's reusable default API stays adapter-light."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
FORBIDDEN_DEFAULT_PACKAGES = {
    "aspen-ci",
    "aspen-cluster",
    "aspen-forge",
    "aspen-jobs",
    "aspen-raft",
    "aspen-testing-madsim",
    "aspen-testing-network",
    "mad-turmoil",
    "madsim",
    "openraft",
}
REQUIRED_EXPLICIT_FEATURES = {
    "router",
    "simulation",
    "federation",
    "jobs",
    "ci",
    "forge",
    "network",
    "testing",
    "full",
}


def run_cargo_tree() -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            "cargo",
            "tree",
            "-p",
            "aspen-testing",
            "--no-default-features",
            "--prefix",
            "none",
        ],
        cwd=REPO_ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )


def load_manifest() -> str:
    return (REPO_ROOT / "crates/aspen-testing/Cargo.toml").read_text()


def main() -> int:
    manifest = load_manifest()
    missing_features = sorted(f for f in REQUIRED_EXPLICIT_FEATURES if f'{f} = [' not in manifest)

    cargo_tree = run_cargo_tree()
    if cargo_tree.returncode != 0:
        print(cargo_tree.stderr, file=sys.stderr)
        return cargo_tree.returncode

    package_names = {
        line.split()[0]
        for line in cargo_tree.stdout.splitlines()
        if line.strip() and not line.startswith("[")
    }
    leaked = sorted(package_names & FORBIDDEN_DEFAULT_PACKAGES)
    report = {
        "crate": "aspen-testing",
        "mode": "no-default-features",
        "forbidden_default_packages": sorted(FORBIDDEN_DEFAULT_PACKAGES),
        "leaked_packages": leaked,
        "required_explicit_features": sorted(REQUIRED_EXPLICIT_FEATURES),
        "missing_explicit_features": missing_features,
        "status": "passed" if not leaked and not missing_features else "failed",
    }
    print(json.dumps(report, indent=2, sort_keys=True))
    if leaked or missing_features:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
