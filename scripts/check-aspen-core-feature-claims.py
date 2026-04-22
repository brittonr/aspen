#!/usr/bin/env python3
"""Check no-std feature claims for `aspen-core`.

This checker validates the documented feature topology and the downstream proof
artifacts used by the OpenSpec no-std core change.
"""

from __future__ import annotations

import argparse
import json
import sys
import tomllib
from pathlib import Path
from typing import Any

EMPTY_LIST: list[str] = []
ROOT_FEATURE_MARKER = 'aspen-core feature "'
STD_FEATURE_MARKER = 'aspen-core feature "std"'
LAYER_FEATURE_MARKER = 'aspen-core feature "layer"'
NO_STD_MARKER = '#![no_std]'
ALLOC_MARKER = 'extern crate alloc;'
ASPEN_CORE_MARKER = 'aspen_core::'


class FeatureClaimError(RuntimeError):
    pass


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--default-features", required=True, type=Path)
    parser.add_argument("--smoke-manifest", required=True, type=Path)
    parser.add_argument("--smoke-source", required=True, type=Path)
    parser.add_argument("--cluster-features", required=True, type=Path)
    parser.add_argument("--cli-features", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    return parser.parse_args(argv)


def normalized_path(path: Path) -> Path:
    return path.expanduser().resolve()


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def strip_evidence_metadata(text: str) -> str:
    lines = text.splitlines()
    metadata_prefixes = (
        "Evidence-ID:",
        "Task-ID:",
        "Artifact-Type:",
        "Covers:",
    )
    line_index = 0
    while line_index < len(lines) and any(lines[line_index].startswith(prefix) for prefix in metadata_prefixes):
        line_index += 1
    while line_index < len(lines) and lines[line_index] == "":
        line_index += 1
    return "\n".join(lines[line_index:]) + ("\n" if lines[line_index:] else "")


def read_evidence_text(path: Path) -> str:
    return strip_evidence_metadata(path.read_text())


def load_toml(path: Path) -> dict[str, Any]:
    return tomllib.loads(read_evidence_text(path))


def default_feature_result(path: Path) -> dict[str, Any]:
    text = read_evidence_text(path)
    offending_lines = [line for line in text.splitlines() if ROOT_FEATURE_MARKER in line]
    return {
        "ok": not offending_lines,
        "path": str(path),
        "offending_lines": offending_lines,
    }


def manifest_feature_result(path: Path) -> dict[str, Any]:
    data = load_toml(path)
    features = data.get("features", {})
    default_features = features.get("default", None)
    layer_features = set(features.get("layer", []))
    global_discovery_features = set(features.get("global-discovery", []))
    sql_features = set(features.get("sql", []))
    messages: list[str] = []
    if default_features != EMPTY_LIST:
        messages.append(f"default features expected [], found {default_features!r}")
    if {"std", "dep:aspen-layer"} - layer_features:
        messages.append(f"layer missing required entries: {sorted({'std', 'dep:aspen-layer'} - layer_features)}")
    if {"std", "dep:iroh-blobs"} - global_discovery_features:
        messages.append(
            f"global-discovery missing required entries: {sorted({'std', 'dep:iroh-blobs'} - global_discovery_features)}"
        )
    if "std" in sql_features:
        messages.append("sql must remain alloc-safe and not require std")
    return {"ok": not messages, "path": str(path), "messages": messages}


def smoke_manifest_result(path: Path) -> dict[str, Any]:
    data = load_toml(path)
    dependency = data.get("dependencies", {}).get("aspen-core")
    messages: list[str] = []
    if dependency is None:
        messages.append("missing aspen-core dependency")
    elif isinstance(dependency, dict):
        if "features" in dependency:
            messages.append("aspen-core dependency must not override features")
        if "default-features" in dependency:
            messages.append("aspen-core dependency must use bare default resolution")
    else:
        messages.append("aspen-core dependency should be an explicit table entry")
    return {"ok": not messages, "path": str(path), "messages": messages}


def smoke_source_result(path: Path) -> dict[str, Any]:
    text = read_evidence_text(path)
    messages: list[str] = []
    if NO_STD_MARKER not in text:
        messages.append("missing #![no_std]")
    if ALLOC_MARKER not in text:
        messages.append("missing extern crate alloc")
    if ASPEN_CORE_MARKER not in text:
        messages.append("missing explicit aspen_core usage")
    return {"ok": not messages, "path": str(path), "messages": messages}


def consumer_feature_result(path: Path, required_markers: list[str]) -> dict[str, Any]:
    text = read_evidence_text(path)
    missing_markers = [marker for marker in required_markers if marker not in text]
    return {"ok": not missing_markers, "path": str(path), "missing_markers": missing_markers}


def write_output(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    root = repo_root()
    core_manifest_path = root / "crates/aspen-core/Cargo.toml"
    results = {
        "default_features": default_feature_result(normalized_path(args.default_features)),
        "core_manifest": manifest_feature_result(core_manifest_path),
        "smoke_manifest": smoke_manifest_result(normalized_path(args.smoke_manifest)),
        "smoke_source": smoke_source_result(normalized_path(args.smoke_source)),
        "cluster_features": consumer_feature_result(
            normalized_path(args.cluster_features),
            [STD_FEATURE_MARKER],
        ),
        "cli_features": consumer_feature_result(
            normalized_path(args.cli_features),
            [LAYER_FEATURE_MARKER, STD_FEATURE_MARKER],
        ),
    }
    failures = [name for name, result in results.items() if not result["ok"]]
    payload = {
        "ok": not failures,
        "failures": failures,
        "results": results,
    }
    output_path = normalized_path(args.output)
    write_output(output_path, payload)
    if failures:
        print("feature claims check failed", file=sys.stderr)
        for name in failures:
            print(f"- {name}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
