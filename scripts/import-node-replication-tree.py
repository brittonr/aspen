#!/usr/bin/env python3
"""Import the frozen verifier pilot's exact, hash-pinned upstream tree artifact."""
import argparse
import base64
import hashlib
import json
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path, help="Previously exported upstream Git tree JSON")
    args = parser.parse_args()
    directory = Path(__file__).resolve().parent.parent / "verification/verified-node-replication-pilot"
    profile = json.loads((directory / "generated/profile.json").read_text())
    data = args.input.read_bytes()
    # SHA-256 is the existing Nix fixed-output interoperability identity.
    actual = "sha256-" + base64.b64encode(hashlib.sha256(data).digest()).decode("ascii")
    expected = profile["upstream"]["tree_api_hash"]
    if actual != expected:
        parser.error(f"upstream tree hash mismatch: expected {expected}, got {actual}")
    (directory / "upstream-tree.json").write_bytes(data)
    print(f"Imported exact upstream tree {profile['upstream']['revision']} ({expected})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
