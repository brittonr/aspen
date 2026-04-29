## 0. Baseline and target selection

- [ ] R1 Capture current extraction status for completed families and candidate inventory, including Redb Raft KV, coordination, protocol/wire, transport/RPC, blob/castore/cache, KV branch/commit DAG, foundational types, auth/tickets, jobs/CI, trust/crypto/secrets, testing harness, config/plugin, and binary shells; save selection evidence under `openspec/changes/decompose-next-five-crate-families/evidence/selection-baseline.md`. [covers=architecture.modularity.next-decomposition-wave-is-selected]

- [ ] R2 Capture per-family dependency/source baselines for `foundational-types`, `auth-ticket`, `jobs-ci-core`, `trust-crypto-secrets`, and `testing-harness`, including direct dependencies, transitive app/runtime leak paths, representative consumers, existing test rails, and first blockers; save artifacts under `openspec/changes/decompose-next-five-crate-families/evidence/`. [covers=architecture.modularity.next-decomposition-first-blockers-are-explicit]

## 1. Roadmap, manifests, and policy

- [ ] I1 Update `docs/crate-extraction.md` with a next-wave section that lists exactly `foundational-types`, `auth-ticket`, `jobs-ci-core`, `trust-crypto-secrets`, and `testing-harness`, records selected crates, ordering rationale, first blockers, deferred-candidate rationale for config/plugin APIs and binary-shell cleanup, and a required note format for any out-of-order implementation that documents the bypassed prerequisite plus temporary compatibility guard. [covers=architecture.modularity.next-decomposition-wave-is-selected,architecture.modularity.next-decomposition-first-blockers-are-explicit]

- [ ] I2 Upgrade or create full manifests at `docs/crate-extraction/foundational-types.md`, `docs/crate-extraction/auth-ticket.md`, `docs/crate-extraction/jobs-ci-core.md`, `docs/crate-extraction/trust-crypto-secrets.md`, and `docs/crate-extraction/testing-harness.md`, including audience, owner, package metadata, license/publication policy, feature contract, dependency decisions, compatibility plan, representative consumers, downstream fixture plan, positive/negative verification rails, readiness state, and first blocker. [covers=architecture.modularity.next-decomposition-manifests-are-complete]

- [ ] I3 Extend `docs/crate-extraction/policy.ncl` and `scripts/check-crate-extraction-readiness.rs` so `--candidate-family foundational-types`, `auth-ticket`, `jobs-ci-core`, `trust-crypto-secrets`, and `testing-harness` enforce manifest completeness, readiness-state restrictions, dependency boundaries, representative-consumer feature unification, downstream fixture metadata, compatibility evidence, and negative mutation checks. [covers=architecture.modularity.next-decomposition-policy-covers-wave]

## 2. Foundational types/helpers first slice

- [ ] I4 Split `aspen-storage-types` portable data types from Redb table-definition/runtime storage concerns, then update affected consumers to import the Redb table definition from the storage/shell crate that owns it, keeping compatibility paths only when documented in the manifest. [covers=architecture.modularity.next-decomposition-first-blockers-are-explicit,architecture.modularity.next-decomposition-standalone-and-compatibility-proof]

- [ ] I5 Split `aspen-traits` into narrower KV capability traits or equivalent reusable ports while retaining a compatibility composite for existing `KeyValueStore` consumers; prove `aspen-cluster-types`, `aspen-hlc`, `aspen-time`, and `aspen-constants` keep reusable default boundaries documented by the foundational manifest. [covers=architecture.modularity.next-decomposition-first-blockers-are-explicit,architecture.modularity.next-decomposition-standalone-and-compatibility-proof]

## 3. Auth and tickets first slice

- [ ] I6 Migrate portable auth/ticket consumers to canonical `aspen-auth-core` and `aspen-hooks-ticket` imports where possible, document any runtime `aspen-auth` compatibility re-exports, and add token/ticket serialization compatibility tests with negative malformed-token or malformed-ticket cases. [covers=architecture.modularity.next-decomposition-first-blockers-are-explicit,architecture.modularity.next-decomposition-standalone-and-compatibility-proof]

- [ ] I7 Prove runtime-only HMAC, verifier, revocation, storage, and hook-handler APIs remain unavailable from portable auth/ticket defaults unless a manifest-documented runtime feature or shell crate is enabled. [covers=architecture.modularity.next-decomposition-policy-covers-wave,architecture.modularity.next-decomposition-standalone-and-compatibility-proof]

## 4. Jobs and CI core first slice

- [ ] I8 Define jobs/CI reusable surface ownership for schema, scheduler, run-state, artifact metadata, and Nickel config contracts; document worker runtime, executor, process-spawning, node, and handler shells as adapter/runtime surfaces. [covers=architecture.modularity.next-decomposition-first-blockers-are-explicit,architecture.modularity.next-decomposition-standalone-and-compatibility-proof]

- [ ] I9 Move or gate jobs/CI scheduler, run-state, artifact metadata, and Nickel config parsing/evaluation contracts so reusable defaults avoid shell executor, VM executor, Nix executor, concrete process spawning, Aspen node bootstrap, and handler registries. [covers=architecture.modularity.next-decomposition-first-blockers-are-explicit,architecture.modularity.next-decomposition-standalone-and-compatibility-proof]

- [ ] I10 Document every retained jobs/CI runtime edge as a named feature or adapter crate, then prove shell/VM/Nix executor APIs are unavailable from reusable defaults without those features. [covers=architecture.modularity.next-decomposition-policy-covers-wave,architecture.modularity.next-decomposition-standalone-and-compatibility-proof]

- [ ] I11 Add downstream-style jobs/CI fixture coverage that uses canonical scheduler/config/run-state APIs without root `aspen`, handler registries, node bootstrap, shell executors, VM executors, or concrete process/Nix runtime dependencies in reusable defaults. [covers=architecture.modularity.next-decomposition-policy-covers-wave,architecture.modularity.next-decomposition-standalone-and-compatibility-proof]

## 5. Trust, crypto, and secrets first slice

- [ ] I12 Isolate pure Shamir/GF/HKDF/share-chain helpers from Raft log application, Iroh trust-share exchange, startup peer probing, redb secrets storage, and secrets-service runtime shells; keep time/randomness/storage inputs explicit. [covers=architecture.modularity.next-decomposition-first-blockers-are-explicit,architecture.modularity.next-decomposition-standalone-and-compatibility-proof]

- [ ] I13 Isolate trust reconfiguration and decryption-key-selection state machines behind deterministic inputs and outputs, leaving Raft/Iroh/secrets-service orchestration in documented adapter shells. [covers=architecture.modularity.next-decomposition-first-blockers-are-explicit,architecture.modularity.next-decomposition-standalone-and-compatibility-proof]

- [ ] I14 Add positive and negative tests for trust/crypto/secrets pure cores, including valid reconstruction/reconfiguration cases plus malformed share, wrong epoch, insufficient quorum, corrupted digest, and stale-key rejection cases; preserve runtime trust/secrets compatibility through documented adapters. [covers=architecture.modularity.next-decomposition-policy-covers-wave,architecture.modularity.next-decomposition-standalone-and-compatibility-proof]

## 6. Testing harness first slice

- [ ] I15 Inventory testing-harness helper ownership and classify reusable simulation, workload, assertion, fixture, madsim, network, and patchbay helpers separately from Aspen cluster bootstrap, node config, concrete transport, and binary shell helpers. [covers=architecture.modularity.next-decomposition-first-blockers-are-explicit,architecture.modularity.next-decomposition-standalone-and-compatibility-proof]

- [ ] I16 Move or gate reusable simulation/workload/assertion helpers behind defaults that avoid root `aspen`, node bootstrap, handler registries, concrete cluster runtime dependencies, and binary shells, then add manifest-tracked compatibility paths for existing madsim, network, and patchbay suites. [covers=architecture.modularity.next-decomposition-first-blockers-are-explicit,architecture.modularity.next-decomposition-standalone-and-compatibility-proof]

- [ ] I17 Add downstream-style harness fixtures and self-tests that prove generic harness helpers can be used without root `aspen`, node bootstrap, handler registries, or concrete cluster runtime dependencies in reusable defaults. [covers=architecture.modularity.next-decomposition-policy-covers-wave,architecture.modularity.next-decomposition-standalone-and-compatibility-proof]

## 7. Verification and evidence discipline

- [ ] V1 Run `scripts/check-crate-extraction-readiness.rs` for all five selected candidate families and save markdown/JSON outputs plus negative mutation evidence for forbidden runtime dependency, missing owner, invalid readiness state, missing downstream fixture, and missing compatibility evidence. [covers=architecture.modularity.next-decomposition-policy-covers-wave]

- [ ] V2 Save positive downstream fixture `cargo metadata`, `cargo check`, and focused test evidence for all five selected families, proving canonical APIs are used directly and forbidden app/runtime crates are absent from reusable defaults except documented adapter-purpose exceptions; use one family-scoped evidence file per target named `v2-downstream-<family>.md`. [covers=architecture.modularity.next-decomposition-standalone-and-compatibility-proof]

- [ ] V3 Save compatibility compile/test evidence for every representative Aspen workspace consumer named in the five manifests, including direct migrations and any temporary compatibility paths with owner, test, and removal criteria; use one family-scoped evidence file per target named `v3-compat-<family>.md`. [covers=architecture.modularity.next-decomposition-standalone-and-compatibility-proof]

- [ ] V4 Save positive and negative tests for the first blocker in each selected family, ensuring happy-path behavior still works and invalid inputs, malformed data, missing features, or forbidden dependency paths fail correctly; use one family-scoped evidence file per target named `v4-tests-<family>.md`. [covers=architecture.modularity.next-decomposition-first-blockers-are-explicit,architecture.modularity.next-decomposition-standalone-and-compatibility-proof]

- [ ] V5 Before checking any task complete, create `openspec/changes/decompose-next-five-crate-families/verification.md` from the repository template, link every checked task verbatim with repo-relative evidence paths, save a diff artifact when source changes are claimed, and run `scripts/openspec-preflight.sh decompose-next-five-crate-families` plus the relevant OpenSpec stage gate transcript. [covers=architecture.modularity.next-decomposition-manifests-are-complete]
