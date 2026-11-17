# SPDX-License-Identifier: AGPL-3.0-or-later
# r[impl molten.batch_orchestration.vendor]
# r[impl molten.batch_orchestration.boundary]
{
  schema = "molten.standalone-batch-adoption.v1",
  bundle = "aspen-orchestration-batch-implementation.zip",
  bundle_origin = "repository-root candidate archive supplied for independent validation; bundle README names the brittonr/aspen molten base",
  bundle_blake3 = "74e9b89aa577943ee4edee37ecdc1cce0ebfdae254989ef6b6ae893b2b626874",
  bundle_member_prefix = "aspen-orchestration-full/overlay/pilots/orchestration/",
  molten_base = "bb6f3830ee7327da9875ea85a8c8e25697eddc35",
  adoption_base = "f3c088ab9a950eebe40e420e3544d7af3440648b",
  independent_review_date = "2026-09-09",
  license = "AGPL-3.0-or-later",
  maintenance_owner = "Molten contributors; pilots/orchestration",
  deployment_profile = "standalone-single-controller-import-free-component-batch-v1",
  optional_workload_profiles = ["pure-net-eval/v1"],
  pure_net_source = {
    repository = "https://github.com/OnixResearch/noble",
    revision = "f7af9e972aa2968d22978794a667d267cf2745e8",
    selection_policy = "../../vendor/sources.ncl",
    generated_lock = "../../vendor/sources.lock.json",
    permission_record = "exact-revision operator AGPL-3.0-or-later grant; selected Rust/Cargo metadata and only the separately granted result.noble asset",
    consumer_overlay = "separately declared bounded sequential/control and independent reference hooks; originals preserved",
    adoption_decision = "opt-in only; frozen producer performance NO-GO is not overturned",
  },
  repairs = [
    "adapters/src/storage.rs: import redb::ReadableDatabase for both read-transaction open_table calls",
    "adapters/src/root.rs: reopen the held directory with read flags before fsync; do not fsync an O_PATH handle",
    "batch-core/src: rename fallible Resources::add to checked_add and migrate all callers without weakening overflow checks",
    "batch-core::Command::Schedule: borrow the transient ExecutionRequest, avoiding an oversized enum and redundant clone without boxing",
    "adapters/src/storage.rs: propagate every fallible set_durability(Immediate) result before table mutation or commit",
  ],
  adoption_adjustments = [
    "add repository AGPL SPDX headers to imported Rust, scripts, manifests, WIT/WAT and service source",
    "pin the repository nightly-2026-05-26 toolchain rather than floating stable",
    "commit the resolved standalone Cargo.lock and enforce --locked for tests, Clippy and release builds",
    "scripts/demo.sh: honor CARGO_TARGET_DIR for the release executable",
    "adapters/tests/root.rs: retain directory-sync, held-root rename and create-only publication regression coverage",
    "review docs/MOLTEN-INTEGRATION.md with explicit per-port follow-on gates and preserve mainline assignment-token fencing",
    "pure-net-eval/v1: exact installed manifest and independent native grants; same authoritative attempt ledger; RPC v3 and artifact/state schema v2 clean cutover; legacy snapshots fail closed",
  ],
  non_claims = [
    "not installed as a Molten system extension and not part of the root Cargo workspace",
    "operator policy and standalone database are not Basalt/Cairn production admission",
    "same-host execution does not qualify cross-host, HA, production isolation or valuable workloads",
    "passing fixtures do not prove Wasmtime, component, worker, transport or whole-system correctness",
    "pure-net native calculation is trusted in-process work, not a sandbox, speedup, parallel/checkpoint admission or default backend",
  ],
}
