#!/usr/bin/env python3
"""Deterministic dependency checks for foundation leaf and wire crates."""

from __future__ import annotations

import argparse
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Final

COMMENT_PREFIX: Final[str] = "#"
ASCII_CHARSET: Final[str] = "ascii"
NORMAL_EDGE_KIND: Final[str] = "normal"
NO_DEFAULT_FEATURES_FLAG: Final[str] = "--no-default-features"
PREFIX_KIND: Final[str] = "none"

ROOT: Final[Path] = Path(__file__).resolve().parents[1]

LEAF_DENYLIST: Final[tuple[str, ...]] = (
    "iroh",
    "iroh-base",
    "libc",
    "redb",
)
WIRE_DENYLIST: Final[tuple[str, ...]] = (
    "anyhow",
    "aspen-disk",
    "aspen-layer",
    "aspen-time",
    "chrono",
    "iroh",
    "iroh-base",
    "iroh-blobs",
    "libc",
    "n0-future",
    "rand",
    "redb",
    "serde_json",
    "tokio",
    "tokio-util",
    "tracing",
)


class CheckError(RuntimeError):
    pass


class ResultRecorder:
    def __init__(self) -> None:
        self.failures: list[str] = []

    def pass_line(self, message: str) -> None:
        print(f"PASS {message}")

    def fail_line(self, message: str) -> None:
        self.failures.append(message)
        print(f"FAIL {message}")

    def require(self, condition: bool, success_message: str, failure_message: str) -> None:
        if condition:
            self.pass_line(success_message)
        else:
            self.fail_line(failure_message)



def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mode", choices=("leaf", "wire"), required=True)
    return parser.parse_args(argv)



def load_toml(path: Path) -> dict[str, object]:
    with path.open("rb") as handle:
        return tomllib.load(handle)



def dependency_spec(manifest: dict[str, object], package_name: str) -> dict[str, object] | None:
    dependencies = manifest.get("dependencies", {})
    if not isinstance(dependencies, dict):
        raise CheckError("dependencies table missing")
    spec = dependencies.get(package_name)
    return spec if isinstance(spec, dict) else None



def has_normal_dependency(manifest: dict[str, object], package_name: str) -> bool:
    dependencies = manifest.get("dependencies", {})
    if not isinstance(dependencies, dict):
        raise CheckError("dependencies table missing")
    return package_name in dependencies



def cargo_tree_packages(package_name: str) -> list[str]:
    command = [
        "cargo",
        "tree",
        "-p",
        package_name,
        NO_DEFAULT_FEATURES_FLAG,
        "-e",
        NORMAL_EDGE_KIND,
        "--prefix",
        PREFIX_KIND,
        "--charset",
        ASCII_CHARSET,
    ]
    completed = subprocess.run(command, cwd=ROOT, text=True, capture_output=True)
    if completed.returncode != 0:
        raise CheckError(completed.stderr.strip())
    packages: list[str] = []
    for raw_line in completed.stdout.splitlines():
        line = raw_line.split(COMMENT_PREFIX, maxsplit=1)[0].strip()
        if not line:
            continue
        package_name_field = line.split(" ", maxsplit=1)[0]
        if package_name_field not in packages:
            packages.append(package_name_field)
    return packages



def check_no_denied_packages(recorder: ResultRecorder, crate_name: str, denied_packages: tuple[str, ...]) -> None:
    packages = cargo_tree_packages(crate_name)
    denied_hits = sorted(package for package in packages if package in denied_packages)
    recorder.require(
        not denied_hits,
        success_message=f"{crate_name} no-default-features graph excludes {', '.join(denied_packages)}",
        failure_message=f"{crate_name} no-default-features graph leaked {', '.join(denied_hits)}",
    )



def check_leaf_mode(recorder: ResultRecorder) -> None:
    storage_manifest_path = ROOT / "crates/aspen-storage-types/Cargo.toml"
    traits_manifest_path = ROOT / "crates/aspen-traits/Cargo.toml"
    storage_manifest = load_toml(storage_manifest_path)
    traits_manifest = load_toml(traits_manifest_path)

    recorder.require(
        not has_normal_dependency(storage_manifest, "redb"),
        success_message="aspen-storage-types keeps redb out of normal dependencies",
        failure_message="aspen-storage-types still lists redb as a normal dependency",
    )
    recorder.require(
        dependency_spec(traits_manifest, "aspen-cluster-types") == {
            "path": "../aspen-cluster-types",
            "default-features": False,
        },
        success_message="aspen-traits keeps aspen-cluster-types on alloc-safe default-features = false",
        failure_message="aspen-traits must depend on aspen-cluster-types with default-features = false",
    )
    recorder.require(
        dependency_spec(traits_manifest, "aspen-kv-types") == {
            "path": "../aspen-kv-types",
            "default-features": False,
        },
        success_message="aspen-traits keeps aspen-kv-types on alloc-safe default-features = false",
        failure_message="aspen-traits must depend on aspen-kv-types with default-features = false",
    )
    check_no_denied_packages(recorder, "aspen-storage-types", LEAF_DENYLIST)
    check_no_denied_packages(recorder, "aspen-traits", LEAF_DENYLIST)



def check_wire_mode(recorder: ResultRecorder) -> None:
    manifest_paths = {
        "aspen-client-api": ROOT / "crates/aspen-client-api/Cargo.toml",
        "aspen-coordination-protocol": ROOT / "crates/aspen-coordination-protocol/Cargo.toml",
        "aspen-jobs-protocol": ROOT / "crates/aspen-jobs-protocol/Cargo.toml",
        "aspen-forge-protocol": ROOT / "crates/aspen-forge-protocol/Cargo.toml",
    }
    for crate_name, manifest_path in manifest_paths.items():
        manifest = load_toml(manifest_path)
        recorder.require(
            not has_normal_dependency(manifest, "serde_json"),
            success_message=f"{crate_name} keeps serde_json out of normal dependencies",
            failure_message=f"{crate_name} still lists serde_json as a normal dependency",
        )
        check_no_denied_packages(recorder, crate_name, WIRE_DENYLIST)



def main(argv: list[str]) -> int:
    args = parse_args(argv)
    recorder = ResultRecorder()
    try:
        if args.mode == "leaf":
            check_leaf_mode(recorder)
        else:
            check_wire_mode(recorder)
    except CheckError as error:
        recorder.fail_line(str(error))
    if recorder.failures:
        print("SUMMARY failed")
        return 1
    print("SUMMARY ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
