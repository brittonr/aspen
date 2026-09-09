# Proposal: Adopt the standalone batch orchestration pilot

## Why

An independently authored candidate delivery (`aspen-orchestration-batch-implementation`,
based on branch `molten` at `bb6f3830ee7327da9875ea85a8c8e25697eddc35`) provides a
single-controller, multi-worker batch/DAG service: pure `no_std` batch and allocation
transition cores, controller/worker effect ordering over owned ports, real Redb
persistence, Iroh peer-authenticated RPC, import-free Wasmtime component execution,
canonical Preserves identities, recovery, cancellation fencing, resource reservations,
and an operator CLI.

An independent review on 2026-09-09 confirmed the delivery did not compile as authored.
After three bounded repairs (a missing `redb::ReadableDatabase` import, an O_PATH
directory `fsync` that always fails with `EBADF` on Linux, and two strict-Clippy
trait-method lints) all 61 workspace tests pass and the end-to-end demo completes a
two-process DAG with exact output. The source has useful, near-working mechanism but
is not yet enrolled, licensed into the tree, or connected to Molten admission.

## What Changes

- Vendor the reviewed overlay source under `pilots/orchestration` with the three review
  fixes applied, a recorded source provenance identity, and a resolved `Cargo.lock`.
  r[molten.batch_orchestration.vendor]
- Repair the delivery's real defects upstream of integration: the Redb read-trait
  import, the capability-root directory durability sync, and strict-Clippy conformance
  across all seven crates. r[molten.batch_orchestration.repairs]
- Reproduce the independent validation: workspace tests, strict-Clippy, release build,
  and the two-process end-to-end demo, recorded as repo-owned evidence. r[molten.batch_orchestration.validation]
- Run the guarded reference scheduler patch (`lease_epoch`) against the exact base
  commit and pass `scripts/reference-tests.sh` with repository dependencies. r[molten.batch_orchestration.reference]
- Keep the pilot outside the admitted system: no `SystemExtensionHost` registration, no
  root-workspace membership, no ALPN registry change, no invented Basalt/Cairn
  admission receipts, and a distinct named deployment profile with its own database. r[molten.batch_orchestration.boundary]
- Define the native integration plan from `docs/MOLTEN-INTEGRATION.md` as explicit,
  separately-gated follow-on requirements for placement, admission, content identity,
  delivery, and execution ports. r[molten.batch_orchestration.integration]

## Impact

- **`pilots/orchestration`**: new pilot workspace (core, application, batch-core,
  batch-application, codec, adapters, cli) with AGPL-3.0-or-later headers consistent
  with the repository.
- **Molten fabric scheduler**: exact `lease_epoch` completion requirement plus updated
  constructors, delivered only through the guarded patch path at the pinned base.
- **No change** to node daemon, ALPN registries, system-tier manifests, or workspace
  membership in this change.

## Out of Scope

- Controller HA/Raft, persistent services, containers/native commands, WASI or
  hostcalls, rolling deployment, automatic uncertain-worker replacement, history GC,
  production isolation, and WAN/relay qualification.
- Basalt/Cairn production admission of the pilot as a system extension.
- Claiming Wasmtime, component, worker, transport, or whole-system correctness, or
  production readiness for valuable workloads.

## Affected Specs

- `batch-orchestration-adoption`: vendor provenance, defect repairs, validation
  evidence, reference patch conformance, pilot boundary, and integration planning.
