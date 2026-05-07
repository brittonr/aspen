#!/usr/bin/env python3
"""Validate Aspen's typed Nickel contract registry.

The Nickel contract validates schema shape. This script validates repository-local
invariants that Nickel cannot express conveniently: duplicate ids, source-kind
ownership rules, source path existence, Crunch classifications, and explicit
non-candidate coverage.
"""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
REGISTRY = REPO_ROOT / "schemas" / "typed-nickel-contract-registry.ncl"

REQUIRED_FAMILIES = {
    "ci-pipeline-config",
    "deploy-protocol-dtos",
    "dogfood-run-receipt",
    "native-ci-run-receipt",
    "node-cluster-profile-config",
    "test-harness-suite-manifests",
    "patchbay-fault-scenarios",
    "crate-extraction-readiness-policy",
    "feature-bundle-policy",
    "snix-build-executor-policy",
    "trust-bootstrap-policy",
    "operator-diagnostics-evidence",
    "sponsored-runtime-policy",
    "sponsored-runtime-grant",
    "sponsored-quota-ledger",
    "sponsored-usage-receipt",
}

REQUIRED_CRUNCH = {
    "crunch-contract-helpers",
    "crunch-project-contracts",
    "crunch-project-outputs",
    "crunch-derivation-builder",
    "crunch-inventory-topology",
    "crunch-system-module",
    "crunch-glue-rust-types",
    "crunch-project-manifest-validation",
    "crunch-build-report-evidence",
    "crunch-operator-diagnostics",
    "crunch-witness-rebuild",
    "crunch-attestation-schema",
}

REQUIRED_NON_CANDIDATES = {
    "raft-behavior",
    "protocol-discriminant-ownership",
    "cryptographic-internals",
    "raw-secret-material",
    "hot-path-runtime-constants",
    "crunch-runtime-build-semantics",
}


def run_json_export() -> dict:
    if shutil.which("nickel"):
        cmd = ["nickel", "export", "--format", "json", str(REGISTRY)]
    elif shutil.which("nix"):
        cmd = ["nix", "run", "nixpkgs#nickel", "--", "export", "--format", "json", str(REGISTRY)]
    else:
        raise SystemExit("neither `nickel` nor `nix` is available to export the registry")

    result = subprocess.run(cmd, cwd=REPO_ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if result.returncode != 0:
        sys.stderr.write(result.stdout)
        sys.stderr.write(result.stderr)
        raise SystemExit(f"registry export failed: {' '.join(cmd)}")
    return json.loads(result.stdout)


def require_unique(items: list[dict], section: str) -> list[str]:
    seen: set[str] = set()
    errors: list[str] = []
    for item in items:
        item_id = item.get("id")
        if item_id in seen:
            errors.append(f"duplicate id in {section}: {item_id}")
        seen.add(item_id)
    return errors


def resolve(path_text: str) -> Path:
    return (REPO_ROOT / path_text).resolve()


def validate_registry(data: dict) -> list[str]:
    errors: list[str] = []
    families = data.get("families", [])
    crunch = data.get("crunch_prior_art", [])
    non_candidates = data.get("non_candidates", [])

    errors.extend(require_unique(families, "families"))
    errors.extend(require_unique(crunch, "crunch_prior_art"))
    errors.extend(require_unique(non_candidates, "non_candidates"))

    family_ids = {item["id"] for item in families}
    crunch_ids = {item["id"] for item in crunch}
    non_candidate_ids = {item["id"] for item in non_candidates}

    for missing in sorted(REQUIRED_FAMILIES - family_ids):
        errors.append(f"missing required contract family: {missing}")
    for missing in sorted(REQUIRED_CRUNCH - crunch_ids):
        errors.append(f"missing required Crunch classification: {missing}")
    for missing in sorted(REQUIRED_NON_CANDIDATES - non_candidate_ids):
        errors.append(f"missing required non-candidate: {missing}")

    for family in families:
        source_path = resolve(family["source_path"])
        if not source_path.exists():
            errors.append(f"family {family['id']} source_path does not exist: {family['source_path']}")

        source_kind = family["source_kind"]
        generated = family["generated"]
        if source_kind == "rust-derived" and not generated:
            errors.append(f"family {family['id']} is rust-derived but generated=false")
        if source_kind == "nickel-authored" and generated:
            errors.append(f"family {family['id']} is nickel-authored but generated=true")
        if source_kind == "rust-derived" and not family.get("generation_command"):
            errors.append(f"family {family['id']} is rust-derived but lacks generation_command")
        if not family.get("validation_commands"):
            errors.append(f"family {family['id']} has no validation_commands")
        if family["status"] == "existing" and family.get("artifact_path"):
            artifact_path = resolve(family["artifact_path"])
            if not artifact_path.exists():
                errors.append(f"family {family['id']} existing artifact_path does not exist: {family['artifact_path']}")

    rejected = {item["id"] for item in crunch if item["classification"] == "reject"}
    for expected_reject in ("crunch-derivation-builder", "crunch-witness-rebuild"):
        if expected_reject not in rejected:
            errors.append(f"{expected_reject} must remain classified as reject")

    for item in crunch:
        crunch_path = resolve(item["source_path"])
        # Crunch is a sibling reference repo. If it is checked out, require the
        # recorded path to resolve; if not, keep the registry portable.
        if (REPO_ROOT / "../crunch/crunch").resolve().exists() and not crunch_path.exists():
            errors.append(f"Crunch prior-art path does not exist: {item['source_path']}")
        for family_id in item.get("applies_to", []):
            if family_id not in family_ids:
                errors.append(f"Crunch item {item['id']} applies_to unknown family {family_id}")

    return errors


def main() -> int:
    data = run_json_export()
    errors = validate_registry(data)
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print(
        "typed Nickel registry OK: "
        f"{len(data['families'])} families, "
        f"{len(data['crunch_prior_art'])} Crunch classifications, "
        f"{len(data['non_candidates'])} non-candidates"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
