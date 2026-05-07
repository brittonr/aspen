#!/usr/bin/env python3
"""Run focused typed-Nickel contract validation fixtures.

This is intentionally narrower than the full test harness: it typechecks every
registry-owned Nickel contract touched by the typed-Nickel OpenSpec and evaluates
small positive/negative fixtures so contract failures are observable.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]

TYPECHECK_FILES = [
    "crates/aspen-ci/src/config/schema/ci_schema.ncl",
    "crates/aspen-nickel/src/schema/node_config.ncl",
    "test-harness/schema.ncl",
    "docs/crate-extraction/policy.ncl",
    "schemas/feature-bundles.ncl",
    "schemas/snix-build-executor-policy.ncl",
    "schemas/trust-bootstrap-policy.ncl",
    "schemas/operator-diagnostics-evidence.ncl",
    "schemas/sponsored-runtime-policy.ncl",
    "schemas/typed-nickel-contract-registry.ncl",
    "schemas/dogfood-run-receipt.ncl",
    "schemas/ci-run-receipt.ncl",
    "schemas/deploy-protocol.ncl",
]

FIXTURES: dict[str, tuple[str, bool]] = {
    "ci-positive.ncl": (
        '''let PipelineConfig = import "@REPO@/crates/aspen-ci/src/config/schema/ci_schema.ncl" in
{
  name = "typed-ci",
  stages = [{
    name = "build",
    jobs = [{
      name = "cargo-test",
      type = 'shell,
      command = "cargo test",
      cache = { enabled = true, ttl_secs = 60 },
      artifact_specs = [{ name = "junit", path = "target/nextest/default/junit.xml" }],
      deploy = { artifact_from = "build", statefulness = 'stateless },
    }],
  }],
} | PipelineConfig
''',
        True,
    ),
    "node-positive.ncl": (
        '''let schema = import "@REPO@/crates/aspen-nickel/src/schema/node_config.ncl" in
{
  node_id = 1,
  cookie = "cluster-cookie-ref",
  bootstrap_peers = [{ node_id = 2, endpoint = "iroh://peer" }],
  feature_bundles = [{ name = 'dogfood }],
  metrics = { prometheus = true, scrape_interval_secs = 10 },
  trust = { policy_id = "trust-main", threshold = 2, secret_ref = "sops://cluster/share-1" },
} | schema.NodeConfig
''',
        True,
    ),
    "test-harness-positive.ncl": (
        '''let schema = import "@REPO@/test-harness/schema.ncl" in
{
  id = "patchbay-convergence",
  layer = "patchbay",
  owner = "Aspen test maintainers",
  runtime_class = "linux-namespaces",
  capabilities = ["network-emulation"],
  isolation_assumptions = ["namespace-per-suite"],
  timeout_class = "extended",
  expected_artifacts = [{ path = "target/patchbay/report.json" }],
  faults = [{ dimension = "latency", bound = "p95<=250ms", expected_convergence = "cluster recovers" }],
  convergence_assertions = ["raft commits resume"],
  target = { kind = "cargo-nextest", package = "aspen-testing-patchbay" },
} | schema.SuiteManifest
''',
        True,
    ),
    "feature-positive.ncl": (
        '''let schema = import "@REPO@/schemas/feature-bundles.ncl" in
{
  profiles = [{
    name = "dogfood",
    description = "dogfood fixture",
    features = ["forge", "ci", "jobs", "blob", "snix", "trust"],
  }],
} | schema.BundlePolicy
''',
        True,
    ),
    "snix-positive.ncl": (
        '''let schema = import "@REPO@/schemas/snix-build-executor-policy.ncl" in
{
  sandbox = { backend = "bubblewrap", network = false, max_seconds = 120 },
  upstream_caches = [{ name = "cache.nixos.org", url = "https://cache.nixos.org", mode = "read-only" }],
  fallback = "explicit-only",
  allow_nix_cli_fallback = false,
} | schema.ExecutorPolicy
''',
        True,
    ),
    "trust-positive.ncl": (
        '''let schema = import "@REPO@/schemas/trust-bootstrap-policy.ncl" in
{
  cluster_id = "aspen-prod",
  initial_quorum = {
    policy_id = "aspen-prod-trust",
    threshold = 2,
    participants = [
      { id = "node-1", public_identity_ref = "iroh://node-1", share_ref = "sops://shares/node-1" },
      { id = "node-2", public_identity_ref = "iroh://node-2", share_ref = "sops://shares/node-2" },
    ],
  },
} | schema.BootstrapPolicy
''',
        True,
    ),
    "diagnostics-positive.ncl": (
        '''let schema = import "@REPO@/schemas/operator-diagnostics-evidence.ncl" in
{
  generated_by = "aspen-cli diagnose",
  subject = "cluster/aspen-prod",
  findings = [{ id = "receipt-present", severity = "info", summary = "receipt attached" }],
} | schema.DiagnosticEnvelope
''',
        True,
    ),
    "sponsored-policy-positive.ncl": (
        '''let schema = import "@REPO@/schemas/sponsored-runtime-policy.ncl" in
{
  catalogs = [{ id = "small", resources = [{ class = "cpu", limit = 2, unit = "vcpu" }] }],
  provider_offers = [{
    id = "provider-small",
    provider = { id = "provider/node-a", kind = "provider" },
    catalog_ref = "small",
    isolation_classes = ["oci"],
    settlement_kinds = ["invoice"],
    max_concurrent = 2,
  }],
  sponsor_policies = [{
    id = "sponsor-team-a",
    sponsor = { id = "org/team-a", kind = "sponsor" },
    beneficiaries = [{ id = "user/alice", kind = "beneficiary" }],
    settlement_refs = [{ kind = "invoice", reference = "invoice://team-a/1" }],
    max_grant_seconds = 600,
    revocation_ref = "raft://sponsorship/revocations/team-a",
  }],
  admission_profiles = [{
    id = "admit-ci",
    provider_offer_ref = "provider-small",
    sponsor_policy_ref = "sponsor-team-a",
    workload = { id = "workload/ci", kind = "workload" },
    service = { id = "service/forge-ci", kind = "service" },
    requested = [{ class = "cpu", limit = 1, unit = "vcpu" }],
  }],
} | schema.SponsorshipPolicyBundle
''',
        True,
    ),
    "sponsored-policy-negative-secret.ncl": (
        '''let schema = import "@REPO@/schemas/sponsored-runtime-policy.ncl" in
{
  id = "bad-sponsor",
  sponsor = { id = "org/team-a", kind = "sponsor" },
  beneficiaries = [{ id = "user/alice", kind = "beneficiary" }],
  settlement_refs = [{ kind = "invoice", reference = "token=cleartext" }],
  max_grant_seconds = 600,
  revocation_ref = "raft://sponsorship/revocations/team-a",
} | schema.SponsorPolicy
''',
        False,
    ),
    "sponsored-policy-negative-provider-kind.ncl": (
        '''let schema = import "@REPO@/schemas/sponsored-runtime-policy.ncl" in
{
  id = "bad-provider",
  provider = { id = "user/alice", kind = "beneficiary" },
  catalog_ref = "small",
  isolation_classes = ["oci"],
  settlement_kinds = ["invoice"],
} | schema.ProviderOffer
''',
        False,
    ),
    "sponsored-policy-positive-defaults.ncl": (
        '''let schema = import "@REPO@/schemas/sponsored-runtime-policy.ncl" in
{
  id = "provider-defaults",
  provider = { id = "provider/node-a", kind = "provider" },
  catalog_ref = "small",
  isolation_classes = ["wasm"],
  settlement_kinds = ["voucher"],
} | schema.ProviderOffer
''',
        True,
    ),
    "sponsored-policy-negative-zero-limit.ncl": (
        '''let schema = import "@REPO@/schemas/sponsored-runtime-policy.ncl" in
{
  class = "cpu",
  limit = 0,
  unit = "vcpu",
} | schema.ResourceLimit
''',
        False,
    ),
    "sponsored-policy-negative-sponsor-kind.ncl": (
        '''let schema = import "@REPO@/schemas/sponsored-runtime-policy.ncl" in
{
  id = "bad-sponsor-kind",
  sponsor = { id = "provider/node-a", kind = "provider" },
  beneficiaries = [{ id = "user/alice", kind = "beneficiary" }],
  max_grant_seconds = 600,
  revocation_ref = "raft://sponsorship/revocations/team-a",
} | schema.SponsorPolicy
''',
        False,
    ),
    "sponsored-policy-negative-negative-cost.ncl": (
        '''let schema = import "@REPO@/schemas/sponsored-runtime-policy.ncl" in
{
  id = "bad-cost",
  provider_offer_ref = "provider-small",
  sponsor_policy_ref = "sponsor-team-a",
  workload = { id = "workload/ci", kind = "workload" },
  service = { id = "service/forge-ci", kind = "service" },
  requested = [{ class = "cpu", limit = 1, unit = "vcpu" }],
  estimated_cost = -1,
} | schema.AdmissionProfile
''',
        False,
    ),
    "trust-negative-inline-secret.ncl": (
        '''let schema = import "@REPO@/schemas/trust-bootstrap-policy.ncl" in
{
  cluster_id = "aspen-prod",
  initial_quorum = {
    policy_id = "bad",
    threshold = 1,
    participants = [{ id = "node-1", public_identity_ref = "iroh://node-1", share_ref = "password=cleartext" }],
  },
} | schema.BootstrapPolicy
''',
        False,
    ),
    "feature-negative-unknown.ncl": (
        '''let schema = import "@REPO@/schemas/feature-bundles.ncl" in
{
  profiles = [{ name = "dogfood", description = "bad", features = ["unknown-feature"] }],
} | schema.BundlePolicy
''',
        False,
    ),
}


def nickel_cmd() -> list[str]:
    if shutil.which("nickel"):
        return ["nickel"]
    if shutil.which("nix"):
        return ["nix", "run", "nixpkgs#nickel", "--"]
    raise SystemExit("neither nickel nor nix is available")


def run(cmd: list[str], *, expect_ok: bool = True) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(cmd, cwd=REPO_ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    ok = result.returncode == 0
    if ok != expect_ok:
        sys.stderr.write(result.stdout)
        sys.stderr.write(result.stderr)
        expected = "pass" if expect_ok else "fail"
        raise SystemExit(f"expected {expected}: {' '.join(cmd)}")
    return result


def main() -> int:
    base = nickel_cmd()
    for rel in TYPECHECK_FILES:
        run(base + ["typecheck", rel])

    fixture_root = REPO_ROOT / "target" / "typed-nickel-fixtures"
    fixture_root.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="run-", dir=fixture_root) as tmp:
        tmp_path = Path(tmp)
        for name, (content, expect_ok) in FIXTURES.items():
            fixture = tmp_path / name
            fixture.write_text(content.replace("@REPO@", str(REPO_ROOT)))
            run(base + ["export", "--format", "json", str(fixture)], expect_ok=expect_ok)

    print(
        f"typed Nickel fixture checks OK: {len(TYPECHECK_FILES)} typechecks, "
        f"{sum(1 for _, ok in FIXTURES.values() if ok)} positive exports, "
        f"{sum(1 for _, ok in FIXTURES.values() if not ok)} negative exports"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
