#!/usr/bin/env python3
"""Validate the static runtime-host acceptance bundle.

This check is intentionally static: it verifies the docs, harness manifests,
generated inventory, source tests, and non-proof wording that support the
operator-facing runtime-host readiness claim without executing gated VM/Uhyve or
Hyperlight jobs.
"""

from __future__ import annotations

import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


REPO_ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class RuntimeHostAnchor:
    suite_id: str
    manifest_path: str
    proof_marker: str
    guard_marker: str | None
    source_paths: tuple[str, ...]
    required_doc_anchors: tuple[str, ...]
    required_manifest_anchors: tuple[str, ...]
    required_source_anchors: tuple[str, ...]
    target_kind: str
    target_package: str | None
    target_test: str | None
    target_feature: str | None
    run_ignored: str | None


RUNTIME_HOST_ANCHORS: tuple[RuntimeHostAnchor, ...] = (
    RuntimeHostAnchor(
        suite_id="runtime-host-microvm-ci-vm",
        manifest_path="test-harness/suites/vm/runtime-host-microvm-ci.ncl",
        proof_marker="CI job completed via snapshot-restored VM",
        guard_marker=None,
        source_paths=("nix/tests/vm-snapshot-e2e.nix",),
        required_doc_anchors=(
            "runtime-host-microvm-ci-vm",
            "ASPEN_CI_NET_CONFIG",
            "worker registered with cluster",
            "CI job completed via snapshot-restored VM",
            "All stress test jobs completed",
            "package build of `aspen-node-vm-test` without the gated VM check",
        ),
        required_manifest_anchors=("runtime-host-microvm-ci-vm", "aspen-spawned-execution", "e2e-registered"),
        required_source_anchors=(
            "ASPEN_CI_NET_CONFIG",
            "worker registered with cluster",
            "CI job completed via snapshot-restored VM",
            "All stress test jobs completed",
            "timeout_secs",
        ),
        target_kind="nix-build",
        target_package=None,
        target_test=None,
        target_feature=None,
        run_ignored=None,
    ),
    RuntimeHostAnchor(
        suite_id="runtime-host-wasm-product-path",
        manifest_path="test-harness/suites/vm/runtime-host-wasm-product-path.ncl",
        proof_marker="ASPEN_WASM_RUNTIME_HOST_EXECUTED",
        guard_marker="ASPEN_WASM_RUNTIME_HOST_PRODUCT_PATH_GUARD",
        source_paths=("crates/aspen-jobs/tests/wasm_product_path_test.rs",),
        required_doc_anchors=(
            "runtime-host-wasm-product-path",
            "cargo test -p aspen-jobs --test wasm_product_path_test --features plugins-wasm",
            "ASPEN_WASM_RUNTIME_HOST_EXECUTED",
            "ASPEN_WASM_RUNTIME_HOST_PRODUCT_PATH_GUARD",
            "aspen:runtime-host/wasm-v1",
            "runtime-core-only WASM admission checks",
        ),
        required_manifest_anchors=(
            "runtime-host-wasm-product-path",
            "aspen-spawned-execution",
            "e2e-registered",
            "wasm_product_path_test",
            "plugins-wasm",
        ),
        required_source_anchors=(
            "ASPEN_WASM_RUNTIME_HOST_EXECUTED",
            "ASPEN_WASM_RUNTIME_HOST_PRODUCT_PATH_GUARD",
            "JobManager",
            "WorkerPool",
        ),
        target_kind="cargo-nextest",
        target_package="aspen-jobs",
        target_test="wasm_product_path_test",
        target_feature="plugins-wasm",
        run_ignored="default",
    ),
    RuntimeHostAnchor(
        suite_id="runtime-host-hyperlight-product-path",
        manifest_path="test-harness/suites/vm/runtime-host-hyperlight-product-path.ncl",
        proof_marker="ASPEN_HYPERLIGHT_RUNTIME_HOST_EXECUTED",
        guard_marker="ASPEN_HYPERLIGHT_RUNTIME_HOST_PRODUCT_PATH_GUARD",
        source_paths=("crates/aspen-jobs/tests/hyperlight_product_path_test.rs",),
        required_doc_anchors=(
            "runtime-host-hyperlight-product-path",
            "cargo test -p aspen-jobs --test hyperlight_product_path_test --features plugins-vm",
            "ASPEN_HYPERLIGHT_RUNTIME_HOST_EXECUTED",
            "ASPEN_HYPERLIGHT_RUNTIME_HOST_PRODUCT_PATH_GUARD",
            "aspen:runtime-host/hyperlight-v1",
            "direct worker-only calls",
        ),
        required_manifest_anchors=(
            "runtime-host-hyperlight-product-path",
            "aspen-spawned-execution",
            "e2e-registered",
            "hyperlight_product_path_test",
            "plugins-vm",
            "ignored-only",
        ),
        required_source_anchors=(
            "ASPEN_HYPERLIGHT_RUNTIME_HOST_EXECUTED",
            "ASPEN_HYPERLIGHT_RUNTIME_HOST_PRODUCT_PATH_GUARD",
            "JobManager",
            "WorkerPool",
            "HyperlightWorker",
        ),
        target_kind="cargo-nextest",
        target_package="aspen-jobs",
        target_test="hyperlight_product_path_test",
        target_feature="plugins-vm",
        run_ignored="ignored-only",
    ),
    RuntimeHostAnchor(
        suite_id="runtime-host-oci-lowering-product-path",
        manifest_path="test-harness/suites/vm/runtime-host-oci-lowering-product-path.ncl",
        proof_marker="ASPEN_OCI_LOWERING_RUNTIME_HOST_EXECUTED",
        guard_marker="ASPEN_OCI_LOWERING_RUNTIME_HOST_PRODUCT_PATH_GUARD",
        source_paths=("crates/aspen-jobs/tests/oci_lowering_product_path_test.rs",),
        required_doc_anchors=(
            "runtime-host-oci-lowering-product-path",
            "cargo test -p aspen-jobs --test oci_lowering_product_path_test --features plugins-wasm",
            "ASPEN_OCI_LOWERING_RUNTIME_HOST_EXECUTED",
            "ASPEN_OCI_LOWERING_RUNTIME_HOST_PRODUCT_PATH_GUARD",
            "ASPEN_WASM_RUNTIME_HOST_EXECUTED",
            "sha256:",
            "mutable OCI tags alone",
        ),
        required_manifest_anchors=(
            "runtime-host-oci-lowering-product-path",
            "aspen-spawned-execution",
            "e2e-registered",
            "oci_lowering_product_path_test",
            "plugins-wasm",
            "immutable-oci-source-identity",
            "derived-isolated-target-artifact",
        ),
        required_source_anchors=(
            "ASPEN_OCI_LOWERING_RUNTIME_HOST_EXECUTED",
            "ASPEN_OCI_LOWERING_RUNTIME_HOST_PRODUCT_PATH_GUARD",
            "JobManager",
            "WorkerPool",
            "OciLoweringPlan",
            "WasmComponentWorker",
        ),
        target_kind="cargo-nextest",
        target_package="aspen-jobs",
        target_test="oci_lowering_product_path_test",
        target_feature="plugins-wasm",
        run_ignored="default",
    ),
    RuntimeHostAnchor(
        suite_id="runtime-host-hermit-uhyve-product-path",
        manifest_path="test-harness/suites/vm/runtime-host-hermit-uhyve-product-path.ncl",
        proof_marker="ASPEN_HERMIT_UHYVE_RUNTIME_HOST_EXECUTED",
        guard_marker="ASPEN_HERMIT_UHYVE_RUNTIME_HOST_PRODUCT_PATH_GUARD",
        source_paths=(
            "crates/aspen-jobs/tests/hermit_uhyve_product_path_test.rs",
            "crates/aspen-jobs/src/vm_executor/hermit_uhyve.rs",
        ),
        required_doc_anchors=(
            "runtime-host-hermit-uhyve-product-path",
            "ASPEN_UHYVE",
            "ASPEN_HERMIT_UHYVE_IMAGE",
            "hermit-uhyve-marker",
            "hermit-uhyve-marker-contract",
            "fixture-build-is-not-runtime-host-proof",
            "ASPEN_HERMIT_UHYVE_RUNTIME_HOST_EXECUTED",
            "ASPEN_HERMIT_UHYVE_RUNTIME_HOST_PRODUCT_PATH_GUARD",
            "Hermit image package builds such as `.#hermit-uhyve-marker`",
        ),
        required_manifest_anchors=(
            "runtime-host-hermit-uhyve-product-path",
            "aspen-spawned-execution",
            "e2e-registered",
            "hermit_uhyve_product_path_test",
            "plugins-vm",
            "ignored-only",
            "real-uhyve-runner",
            "packages.x86_64-linux.hermit-uhyve-marker",
            "checks.x86_64-linux.hermit-uhyve-marker-contract",
        ),
        required_source_anchors=(
            "ASPEN_HERMIT_UHYVE_RUNTIME_HOST_EXECUTED",
            "ASPEN_HERMIT_UHYVE_RUNTIME_HOST_PRODUCT_PATH_GUARD",
            "JobManager",
            "WorkerPool",
            "HermitUhyveWorker",
            "proof marker missing",
        ),
        target_kind="cargo-nextest",
        target_package="aspen-jobs",
        target_test="hermit_uhyve_product_path_test",
        target_feature="plugins-vm",
        run_ignored="ignored-only",
    ),
)


def read_text(path: str) -> str:
    return (REPO_ROOT / path).read_text(encoding="utf-8")


def require_contains(errors: list[str], label: str, text: str, needles: Iterable[str]) -> None:
    for needle in needles:
        if needle not in text:
            errors.append(f"{label}: missing anchor {needle!r}")


def require_equal(errors: list[str], label: str, actual: object, expected: object) -> None:
    if actual != expected:
        errors.append(f"{label}: expected {expected!r}, got {actual!r}")


def validate_anchor(anchor: RuntimeHostAnchor, inventory: dict, docs: str) -> list[str]:
    errors: list[str] = []
    suite = next((item for item in inventory["suites"] if item["id"] == anchor.suite_id), None)
    if suite is None:
        return [f"inventory: missing suite {anchor.suite_id!r}"]

    label = anchor.suite_id
    require_equal(errors, f"{label}.manifest_path", suite.get("manifest_path"), anchor.manifest_path)
    runtime_host = suite.get("runtime_host") or {}
    require_equal(errors, f"{label}.proof_level", runtime_host.get("proof_level"), "aspen-spawned-execution")
    require_equal(errors, f"{label}.support_status", runtime_host.get("support_status"), "e2e-registered")
    require_equal(errors, f"{label}.gap_reason", runtime_host.get("gap_reason"), None)
    target = suite.get("target") or {}
    require_equal(errors, f"{label}.target.kind", target.get("kind"), anchor.target_kind)
    require_equal(errors, f"{label}.target.package", target.get("package"), anchor.target_package)
    require_equal(errors, f"{label}.target.test", target.get("test"), anchor.target_test)
    if anchor.target_feature is not None and anchor.target_feature not in target.get("features", []):
        errors.append(f"{label}.target.features: missing {anchor.target_feature!r}")
    require_equal(errors, f"{label}.target.run_ignored", target.get("run_ignored"), anchor.run_ignored)

    require_contains(errors, "docs/runtime-host-readiness.md", docs, anchor.required_doc_anchors)
    manifest = read_text(anchor.manifest_path)
    require_contains(errors, anchor.manifest_path, manifest, anchor.required_manifest_anchors)
    sources = "\n".join(read_text(path) for path in anchor.source_paths)
    require_contains(errors, ",".join(anchor.source_paths), sources, anchor.required_source_anchors)
    return errors


def main() -> int:
    inventory_path = REPO_ROOT / "test-harness/generated/inventory.json"
    docs = read_text("docs/runtime-host-readiness.md")
    inventory = json.loads(inventory_path.read_text(encoding="utf-8"))
    errors: list[str] = []

    manifest_paths = set(inventory.get("metadata", {}).get("manifest_paths", []))
    expected_paths = {anchor.manifest_path for anchor in RUNTIME_HOST_ANCHORS}
    missing_paths = sorted(expected_paths - manifest_paths)
    if missing_paths:
        errors.append(f"inventory.metadata.manifest_paths: missing {missing_paths!r}")

    for anchor in RUNTIME_HOST_ANCHORS:
        errors.extend(validate_anchor(anchor, inventory, docs))

    metadata_only_runtime_hosts = [
        item["id"]
        for item in inventory.get("suites", [])
        if item.get("runtime_host")
        and item.get("runtime_host", {}).get("support_status") == "metadata-only"
        and item["id"] in {anchor.suite_id for anchor in RUNTIME_HOST_ANCHORS}
    ]
    if metadata_only_runtime_hosts:
        errors.append(f"promoted runtime-host rows are metadata-only: {metadata_only_runtime_hosts!r}")

    require_contains(
        errors,
        "docs/runtime-host-readiness.md",
        docs,
        (
            "scripts/test-harness.sh runtime-host-acceptance-bundle",
            "not runtime-host proof by themselves",
            "Do not treat any of the following as sufficient by themselves",
            "metadata contract checks such as `hermit-uhyve-marker-contract`",
            "raw container execution, dev/unsafe container paths, or mutable OCI tags alone",
        ),
    )

    if errors:
        print("runtime-host acceptance bundle: FAIL", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print("runtime-host acceptance bundle: PASS")
    print("checked promoted rows:")
    for anchor in RUNTIME_HOST_ANCHORS:
        print(f"- {anchor.suite_id}: {anchor.proof_marker}")
    print("skipped gated execution: microVM, Hyperlight, Hermit/Uhyve live proof reruns")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
