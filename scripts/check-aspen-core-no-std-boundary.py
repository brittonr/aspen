#!/usr/bin/env python3
"""Check alloc-only dependency boundaries for `aspen-core`.

This audit proves the no-std dependency boundary with deterministic Cargo data:
- exact direct normal dependency set
- manifest-side alloc-safe feature settings for constrained direct deps
- transitive allowlist membership
- denylist absence
- per-crate review-note presence and schema validation

The command emits both a machine-readable JSON summary and a human-readable diff.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Any

COMMENT_PREFIX = "#"
DEPTH_PATTERN = re.compile(r"^(\d+)(.+)$")
PACKAGE_PATTERN = re.compile(r"^([A-Za-z0-9_.-]+) v([^\s]+)")
REVIEW_FIELDS = (
    "Crate",
    "Introduced by",
    "Resolved features",
    "Filesystem",
    "Process/global state",
    "Thread/async-runtime",
    "Network",
    "Decision",
)
EXPECTED_DIRECT_PACKAGES = (
    "aspen-cluster-types",
    "aspen-constants",
    "aspen-hlc",
    "aspen-kv-types",
    "aspen-storage-types",
    "aspen-traits",
    "async-trait",
    "base64",
    "bincode",
    "hex",
    "serde",
    "snafu",
    "thiserror",
)
DENYLIST_PACKAGES = (
    "anyhow",
    "aspen-disk",
    "aspen-layer",
    "aspen-time",
    "chrono",
    "iroh",
    "iroh-base",
    "iroh-blobs",
    "n0-future",
    "rand",
    "serde_json",
    "tokio",
    "tokio-util",
    "tracing",
)
MANIFEST_RULES: dict[str, dict[str, Any]] = {
    "aspen-constants": {"default-features": False},
    "aspen-hlc": {"default-features": False},
    "aspen-cluster-types": {"default-features": False},
    "aspen-kv-types": {"default-features": False},
    "aspen-traits": {"default-features": False},
    "aspen-storage-types": {"default-features": False},
    "serde": {"default-features": False, "require-features": {"alloc", "derive"}, "forbid-features": {"std"}},
    "bincode": {"default-features": False},
    "base64": {"default-features": False, "require-features": {"alloc"}, "forbid-features": {"std"}},
    "hex": {"default-features": False, "require-features": {"alloc"}, "forbid-features": {"std"}},
    "snafu": {"default-features": False, "require-features": {"rust_1_65"}, "forbid-features": {"std"}},
}


class BoundaryError(RuntimeError):
    pass


def run_command(command: list[str], cwd: Path) -> str:
    completed = subprocess.run(command, cwd=cwd, text=True, capture_output=True)
    if completed.returncode != 0:
        raise BoundaryError(
            f"command failed ({completed.returncode}): {' '.join(command)}\n{completed.stderr.strip()}"
        )
    return completed.stdout


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest-path", required=True, type=Path)
    parser.add_argument("--allowlist", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--diff-output", required=True, type=Path)
    return parser.parse_args(argv)


def normalized_path(path: Path) -> Path:
    return path.expanduser().resolve()


def repo_root_for_manifest(manifest_path: Path) -> Path:
    return manifest_path.parents[2]


def load_manifest(manifest_path: Path) -> dict[str, Any]:
    with manifest_path.open("rb") as handle:
        return tomllib.load(handle)


def manifest_package_name(manifest_data: dict[str, Any]) -> str:
    return str(manifest_data["package"]["name"])


def dependency_spec(manifest_data: dict[str, Any], package_name: str) -> dict[str, Any] | None:
    value = manifest_data.get("dependencies", {}).get(package_name)
    if value is None or not isinstance(value, dict):
        return None
    return value


def feature_list(spec: dict[str, Any]) -> set[str]:
    features = spec.get("features", [])
    if not isinstance(features, list):
        return set()
    return {str(item) for item in features}


def manifest_rule_results(manifest_data: dict[str, Any]) -> list[dict[str, Any]]:
    results: list[dict[str, Any]] = []
    for package_name, rule in sorted(MANIFEST_RULES.items()):
        spec = dependency_spec(manifest_data, package_name)
        package_result = {"package": package_name, "ok": True, "messages": []}
        if spec is None:
            package_result["ok"] = False
            package_result["messages"].append("missing manifest entry")
            results.append(package_result)
            continue
        if "default-features" in rule:
            actual = spec.get("default-features")
            expected = rule["default-features"]
            if actual != expected:
                package_result["ok"] = False
                package_result["messages"].append(
                    f"default-features expected {expected!r}, found {actual!r}"
                )
        actual_features = feature_list(spec)
        required_features = set(rule.get("require-features", set()))
        missing_features = sorted(required_features - actual_features)
        if missing_features:
            package_result["ok"] = False
            package_result["messages"].append(f"missing required features: {', '.join(missing_features)}")
        forbidden_features = sorted(actual_features & set(rule.get("forbid-features", set())))
        if forbidden_features:
            package_result["ok"] = False
            package_result["messages"].append(f"forbidden features enabled: {', '.join(forbidden_features)}")
        results.append(package_result)
    return results


def cargo_tree_lines(manifest_path: Path, package_name: str, depth_one_only: bool) -> list[str]:
    command = [
        "cargo",
        "tree",
        "--manifest-path",
        str(manifest_path),
        "-p",
        package_name,
        "--no-default-features",
        "-e",
        "normal",
        "--prefix",
        "depth",
        "--charset",
        "ascii",
    ]
    if depth_one_only:
        command.extend(["--depth", "1"])
    stdout = run_command(command, manifest_path.parent)
    return [line.strip() for line in stdout.splitlines() if line.strip()]


def parse_tree_line(line: str) -> tuple[int, str, str]:
    depth_match = DEPTH_PATTERN.match(line)
    if depth_match is None:
        raise BoundaryError(f"unexpected cargo tree line: {line}")
    depth = int(depth_match.group(1))
    package_match = PACKAGE_PATTERN.match(depth_match.group(2).strip())
    if package_match is None:
        raise BoundaryError(f"unable to parse package from line: {line}")
    return depth, package_match.group(1), package_match.group(2)


def unique_package_refs(lines: list[str]) -> list[str]:
    refs: list[str] = []
    for line in lines:
        _, package_name, version = parse_tree_line(line)
        package_ref = f"{package_name}@{version}"
        if package_ref not in refs:
            refs.append(package_ref)
    return refs


def direct_packages(lines: list[str]) -> list[str]:
    packages: list[str] = []
    for line in lines:
        depth, package_name, _ = parse_tree_line(line)
        if depth == 1 and package_name not in packages:
            packages.append(package_name)
    return packages


def transitive_packages(lines: list[str], direct_package_names: set[str]) -> tuple[list[str], dict[str, str]]:
    package_refs: list[str] = []
    introduced_by: dict[str, str] = {}
    ancestors: dict[int, str] = {}
    for line in lines:
        depth, package_name, version = parse_tree_line(line)
        package_ref = f"{package_name}@{version}"
        ancestors[depth] = package_ref
        for stale_depth in [value for value in ancestors if value > depth]:
            del ancestors[stale_depth]
        if depth < 2 or package_name in direct_package_names:
            continue
        top_level_parent = ancestors[1]
        if package_ref not in package_refs:
            package_refs.append(package_ref)
            introduced_by[package_ref] = top_level_parent
    return package_refs, introduced_by


def load_allowlist(path: Path) -> list[str]:
    entries: list[str] = []
    for raw_line in path.read_text().splitlines():
        line = raw_line.split(COMMENT_PREFIX, maxsplit=1)[0].strip()
        if not line:
            continue
        entries.append(line)
    return entries


def review_note_path(repo_root: Path, package_name: str) -> Path:
    review_file_name = f"deps-transitive-review-{package_name}.md"
    candidate_paths = [
        repo_root / "openspec/changes/no-std-aspen-core/evidence" / review_file_name,
        repo_root / "openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence" / review_file_name,
    ]
    for candidate_path in candidate_paths:
        if candidate_path.is_file():
            return candidate_path
    return candidate_paths[0]


def parse_review_note(path: Path) -> dict[str, str]:
    fields: dict[str, str] = {}
    for raw_line in path.read_text().splitlines():
        line = raw_line.strip()
        if not line or ":" not in line:
            continue
        field_name, value = line.split(":", maxsplit=1)
        field_name = field_name.strip()
        if field_name in REVIEW_FIELDS:
            fields[field_name] = value.strip()
    return fields


def review_note_results(repo_root: Path, allowlist_entries: list[str], introduced_by: dict[str, str]) -> list[dict[str, Any]]:
    results: list[dict[str, Any]] = []
    for entry in allowlist_entries:
        package_name, _, version = entry.partition("@")
        note_path = review_note_path(repo_root, package_name)
        package_result = {
            "package": entry,
            "path": str(note_path),
            "ok": True,
            "messages": [],
        }
        if not note_path.is_file():
            package_result["ok"] = False
            package_result["messages"].append("missing review note")
            results.append(package_result)
            continue
        fields = parse_review_note(note_path)
        missing_fields = [field for field in REVIEW_FIELDS if field not in fields]
        if missing_fields:
            package_result["ok"] = False
            package_result["messages"].append(f"missing fields: {', '.join(missing_fields)}")
        crate_value = fields.get("Crate", "")
        if package_name not in crate_value or version not in crate_value:
            package_result["ok"] = False
            package_result["messages"].append(f"Crate field does not mention {entry}")
        introduced_value = fields.get("Introduced by", "")
        if not introduced_value:
            package_result["ok"] = False
            package_result["messages"].append("Introduced by field must not be empty")
        decision_value = fields.get("Decision", "")
        if decision_value.lower() != "allow":
            package_result["ok"] = False
            package_result["messages"].append("Decision field must be 'allow' for allowlisted crates")
        results.append(package_result)
    return results


def denylist_hits(package_refs: list[str]) -> list[str]:
    denied: list[str] = []
    for package_ref in package_refs:
        package_name = package_ref.split("@", maxsplit=1)[0]
        if package_name in DENYLIST_PACKAGES:
            denied.append(package_ref)
    return sorted(denied)


def write_diff(path: Path, sections: dict[str, list[str]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    lines: list[str] = []
    for title, values in sections.items():
        lines.append(f"## {title}")
        if values:
            lines.extend(f"- {value}" for value in values)
        else:
            lines.append("- none")
        lines.append("")
    path.write_text("\n".join(lines).rstrip() + "\n")


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    manifest_path = normalized_path(args.manifest_path)
    allowlist_path = normalized_path(args.allowlist)
    output_path = normalized_path(args.output)
    diff_output_path = normalized_path(args.diff_output)
    repo_root = repo_root_for_manifest(manifest_path)

    try:
        manifest_data = load_manifest(manifest_path)
        package_name = manifest_package_name(manifest_data)
        direct_tree = cargo_tree_lines(manifest_path, package_name, depth_one_only=True)
        full_tree = cargo_tree_lines(manifest_path, package_name, depth_one_only=False)
    except BoundaryError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    direct_package_list = direct_packages(direct_tree)
    expected_direct_list = list(EXPECTED_DIRECT_PACKAGES)
    missing_direct = sorted(set(expected_direct_list) - set(direct_package_list))
    unexpected_direct = sorted(set(direct_package_list) - set(expected_direct_list))

    manifest_results = manifest_rule_results(manifest_data)
    transitive_package_refs, introduced_by = transitive_packages(full_tree, set(direct_package_list))
    allowlist_entries = load_allowlist(allowlist_path)
    allowlist_set = set(allowlist_entries)
    transitive_set = set(transitive_package_refs)
    unexpected_transitives = sorted(transitive_set - allowlist_set)
    missing_from_graph = sorted(allowlist_set - transitive_set)
    denied_packages = denylist_hits(unique_package_refs(full_tree))
    review_results = review_note_results(repo_root, allowlist_entries, introduced_by)

    payload = {
        "ok": False,
        "manifest_path": str(manifest_path),
        "allowlist_path": str(allowlist_path),
        "direct": {
            "expected": expected_direct_list,
            "resolved": direct_package_list,
            "missing": missing_direct,
            "unexpected": unexpected_direct,
        },
        "manifest_rules": manifest_results,
        "transitives": {
            "resolved": sorted(transitive_package_refs),
            "introduced_by": introduced_by,
            "unexpected": unexpected_transitives,
            "missing_from_graph": missing_from_graph,
        },
        "denylist_hits": denied_packages,
        "review_notes": review_results,
    }

    failures = []
    if missing_direct or unexpected_direct:
        failures.append("direct dependency contract mismatch")
    if any(not result["ok"] for result in manifest_results):
        failures.append("manifest feature policy mismatch")
    if unexpected_transitives or missing_from_graph:
        failures.append("transitive allowlist mismatch")
    if denied_packages:
        failures.append("denylisted packages resolved")
    if any(not result["ok"] for result in review_results):
        failures.append("review notes missing or invalid")

    payload["ok"] = not failures
    payload["failures"] = failures
    write_json(output_path, payload)
    write_diff(
        diff_output_path,
        {
            "Direct missing": missing_direct,
            "Direct unexpected": unexpected_direct,
            "Transitives unexpected": unexpected_transitives,
            "Allowlist entries not in graph": missing_from_graph,
            "Denylist hits": denied_packages,
            "Invalid review notes": [
                f"{result['package']}: {'; '.join(result['messages'])}"
                for result in review_results
                if not result["ok"]
            ],
            "Manifest rule failures": [
                f"{result['package']}: {'; '.join(result['messages'])}"
                for result in manifest_results
                if not result["ok"]
            ],
        },
    )
    if failures:
        print("boundary check failed", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
