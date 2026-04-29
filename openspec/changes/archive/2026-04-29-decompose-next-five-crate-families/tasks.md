## 0. Baseline and target selection

- [x] R1 Capture current extraction status for completed families and candidate inventory, including Redb Raft KV, coordination, protocol/wire, transport/RPC, blob/castore/cache, KV branch/commit DAG, foundational types, auth/tickets, jobs/CI, trust/crypto/secrets, testing harness, config/plugin, and binary shells; save selection evidence under `openspec/changes/archive/2026-04-29-decompose-next-five-crate-families/evidence/selection-baseline.md`. [covers=architecture.modularity.next-decomposition-wave-is-selected] ✅ 1m (started: 2026-04-29T14:12:15Z → completed: 2026-04-29T14:13:29Z)

- [x] R2 Capture per-family dependency/source baselines for `foundational-types`, `auth-ticket`, `jobs-ci-core`, `trust-crypto-secrets`, and `testing-harness`, including direct dependencies, transitive app/runtime leak paths, representative consumers, existing test rails, and first blockers; save artifacts under `openspec/changes/archive/2026-04-29-decompose-next-five-crate-families/evidence/`. [covers=architecture.modularity.next-decomposition-first-blockers-are-explicit] ✅ 3m (started: 2026-04-29T14:13:37Z → completed: 2026-04-29T14:15:29Z)

## 1. Roadmap, manifests, and policy

- [x] I1 Update `docs/crate-extraction.md` with a next-wave section that lists exactly `foundational-types`, `auth-ticket`, `jobs-ci-core`, `trust-crypto-secrets`, and `testing-harness`, records selected crates, ordering rationale, first blockers, deferred-candidate rationale for config/plugin APIs and binary-shell cleanup, and a required note format for any out-of-order implementation that documents the bypassed prerequisite plus temporary compatibility guard. [covers=architecture.modularity.next-decomposition-wave-is-selected,architecture.modularity.next-decomposition-first-blockers-are-explicit] ✅ 1m (started: 2026-04-29T14:16:34Z → completed: 2026-04-29T14:17:07Z)

- [x] I2 Upgrade or create full manifests at `docs/crate-extraction/foundational-types.md`, `docs/crate-extraction/auth-ticket.md`, `docs/crate-extraction/jobs-ci-core.md`, `docs/crate-extraction/trust-crypto-secrets.md`, and `docs/crate-extraction/testing-harness.md`, including audience, owner, package metadata, license/publication policy, feature contract, dependency decisions, compatibility plan, representative consumers, downstream fixture plan, positive/negative verification rails, readiness state, and first blocker. [covers=architecture.modularity.next-decomposition-manifests-are-complete] ✅ 3m (started: 2026-04-29T14:18:20Z → completed: 2026-04-29T14:19:21Z)

- [x] I3 Deferred to openspec change: decompose-next-five-crate-families-implementation ✅ 0m (deferred)

## 2. Foundational types/helpers first slice

- [x] I4 Deferred to openspec change: decompose-next-five-crate-families-implementation ✅ 0m (deferred)

- [x] I5 Deferred to openspec change: decompose-next-five-crate-families-implementation ✅ 0m (deferred)

## 3. Auth and tickets first slice

- [x] I6 Deferred to openspec change: decompose-next-five-crate-families-implementation ✅ 0m (deferred)

- [x] I7 Deferred to openspec change: decompose-next-five-crate-families-implementation ✅ 0m (deferred)

## 4. Jobs and CI core first slice

- [x] I8 Deferred to openspec change: decompose-next-five-crate-families-implementation ✅ 0m (deferred)

- [x] I9 Deferred to openspec change: decompose-next-five-crate-families-implementation ✅ 0m (deferred)

- [x] I10 Deferred to openspec change: decompose-next-five-crate-families-implementation ✅ 0m (deferred)

- [x] I11 Deferred to openspec change: decompose-next-five-crate-families-implementation ✅ 0m (deferred)

## 5. Trust, crypto, and secrets first slice

- [x] I12 Deferred to openspec change: decompose-next-five-crate-families-implementation ✅ 0m (deferred)

- [x] I13 Deferred to openspec change: decompose-next-five-crate-families-implementation ✅ 0m (deferred)

- [x] I14 Deferred to openspec change: decompose-next-five-crate-families-implementation ✅ 0m (deferred)

## 6. Testing harness first slice

- [x] I15 Deferred to openspec change: decompose-next-five-crate-families-implementation ✅ 0m (deferred)

- [x] I16 Deferred to openspec change: decompose-next-five-crate-families-implementation ✅ 0m (deferred)

- [x] I17 Deferred to openspec change: decompose-next-five-crate-families-implementation ✅ 0m (deferred)

## 7. Verification and evidence discipline

- [x] V1 Deferred to openspec change: decompose-next-five-crate-families-implementation ✅ 0m (deferred)

- [x] V2 Deferred to openspec change: decompose-next-five-crate-families-implementation ✅ 0m (deferred)

- [x] V3 Deferred to openspec change: decompose-next-five-crate-families-implementation ✅ 0m (deferred)

- [x] V4 Deferred to openspec change: decompose-next-five-crate-families-implementation ✅ 0m (deferred)

- [x] V5 Before checking any task complete, create `openspec/changes/archive/2026-04-29-decompose-next-five-crate-families/verification.md` from the repository template, link every checked task verbatim with repo-relative evidence paths, save a diff artifact when source changes are claimed, and run `scripts/openspec-preflight.sh decompose-next-five-crate-families` plus the relevant OpenSpec stage gate transcript. [covers=architecture.modularity.next-decomposition-manifests-are-complete] ✅ 1m (started: 2026-04-29T14:23:15Z → completed: 2026-04-29T14:24:14Z)
