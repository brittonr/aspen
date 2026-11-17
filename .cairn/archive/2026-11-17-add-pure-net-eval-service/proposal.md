## Why

Issue #33 requires one bounded deterministic pure interaction-net service, not a new scheduler or job ledger. The operator explicitly authorized an opt-in consumer on 2026-11-17 despite Noble's frozen performance NO-GO; this decision does not authorize default adoption, a speedup claim, parallel reduction or portable checkpoints (#34).

Noble revision `f7af9e972aa2968d22978794a667d267cf2745e8` supplies the corrected I64 revision-3 lowerer, validator, sequential reducer and complete ordered readback. Its original project metadata has no license declaration. The operator separately asserted the necessary rights and granted AGPL-3.0-or-later reuse/redistribution of Onix-owned Rust in `noble-ngraph`, `noble-kernel` and `noble-contracts` at that exact revision, including a reviewed source-only snapshot with normalized Cargo manifests. Third-party material, vendored inets proofs and test suites are excluded; no upstream license or repository setting is changed.

The additional interactive grant covers only the indispensable embedded `crates/noble-contracts/src/fixtures/result.noble` asset at that same revision and BLAKE3 `694a6766ca91c9434329df739070f7b26dda43576dfc3788cb7b7c2c271cb4fe`. Source policy v2 admits Rust extensions and explicitly named metadata/assets, excluding every other non-Rust fixture program. The submitted grant is recorded in `evidence/source-permission-2026-11-17.json`; it is not independent upstream ownership verification.

## What Changes

- Install an exact `pure-net-eval/v1` workload manifest in the existing standalone controller/worker composition. Reuse its controller/worker CAS records, resource reservations, cancellation tombstones, stopped-result publication and confirmed-process-exit recovery. This is a selected standalone hosting profile, not activation of the root node daemon or system-tier registry.
- Add a separate explicitly admitted trusted in-process pure-net adapter. Retain Noble's arena during bounded cooperative local quanta, preserve cumulative interaction fuel, bound setup/readback separately, and never implement a quantum by rerunning reduction. Native execution is trusted, not a sandbox.
- Materialize checked source/program, exact net and ordered input through the authoritative producer. Bind rule/arithmetic/lowerer/runtime/readback/canonicalization revisions, executable/dependency closure, manifest, limits and current grants independently of successful output.
- Make the worker carrier artifact-neutral and version its wire contract in one cutover; retain the existing pure-component profile without aliases or silent native selection.
- Enforce fresh result-publication authority, generation/cancellation fences and bounded logical-attempt work. Add real worker-loss/reopen controls at acceptance, begin, calculation, content, terminal and acknowledgement boundaries.
- Retain independent arithmetic and malformed/boundary fixtures, deterministic semantic outcomes, and matched native/reference phase/resource/retry measurements. Preserve slower/no-benefit results and all evidence-class limits.

## Impact

- **Owners and consumer**: Aspen runtime/pilot maintainers own the service and integration. Existing standalone operators are the first consumer; native fabric activation remains outside this profile.
- **Files**: reviewed `vendor/sources.ncl` snapshot selection and generated lock; standalone core/application/codec/adapters/CLI; a pure producer bridge; workload contracts/fixtures; architecture and operator documentation; Cairn requirements and execution evidence.
- **Checks**: existing worker/controller baselines, pure producer and negative admission tests, actual local service execution and process-loss cases, matched reference measurements, snapshot/notice checks, focused formatting/Clippy/Octet/Nix rails and strict Cairn validation.
- **Durable contribution**: a repeatable manifest-bound pure workload over the existing authoritative attempt lifecycle, with exact-source intake and retained positive/negative execution evidence rather than a standalone evaluator demonstration.

## Out of Scope

No default-backend switch, universal performance/refinement/termination claim, new job ledger, per-rewrite durable transaction, unrestricted Noble language/effects, cross-host qualification, parallel backend, portable net checkpoint, inets relicensing, source-trust bypass, production-readiness or release authority.
