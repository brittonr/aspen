## 1. Policy and checker foundation

- [x] I3 Extend `docs/crate-extraction/policy.ncl` and `scripts/check-crate-extraction-readiness.rs` so `foundational-types`, `auth-ticket`, `jobs-ci-core`, `trust-crypto-secrets`, and `testing-harness` are family-selectable, have policy entries, and enforce missing downstream fixture / missing compatibility evidence gates before readiness can be raised. [covers=architecture.modularity.next-decomposition-policy-covers-wave] ✅ completed: 2026-04-30T20:12:00Z

- [x] V1 Save checker transcripts and negative mutation evidence proving at least forbidden runtime dependency, missing owner, invalid readiness state, missing downstream fixture, and missing compatibility evidence failures are caught for the selected-wave checker path. [covers=architecture.modularity.next-decomposition-policy-covers-wave] ✅ completed: 2026-04-30T20:12:00Z

## 2. Foundational types/helpers first slice

- [x] I4 Move or gate `aspen-storage-types` Redb table-definition surface so reusable defaults keep portable storage types only, preserving shell-facing compatibility where needed. [covers=architecture.modularity.next-decomposition-standalone-and-compatibility-proof] ✅ completed: 2026-04-30T20:31:00Z

- [x] I5 Split/prove `aspen-traits` reusable KV capability traits and save downstream fixture, cargo tree, no-default/default, negative boundary, and representative consumer evidence. [covers=architecture.modularity.next-decomposition-standalone-and-compatibility-proof] ✅ completed: 2026-04-30T20:46:00Z

## 3. Auth and tickets first slice

- [ ] I6 Migrate portable consumers to canonical `aspen-auth-core` / `aspen-hooks-ticket` imports or document retained `aspen-auth` compatibility re-exports with owner, tests, and removal criteria. [covers=architecture.modularity.next-decomposition-standalone-and-compatibility-proof]

- [ ] I7 Add token/ticket serialization goldens, malformed-input negative tests, downstream fixture metadata, and compatibility evidence for auth/ticket consumers. [covers=architecture.modularity.next-decomposition-standalone-and-compatibility-proof]

## 4. Jobs and CI core first slice

- [ ] I8 Identify reusable scheduler/config/run-state/artifact surfaces and gate worker/executor/runtime shells behind adapter crates or named features. [covers=architecture.modularity.next-decomposition-standalone-and-compatibility-proof]

- [ ] I9 Add downstream scheduler/config fixture metadata and negative boundary evidence rejecting root app, handler, process-spawn, shell, VM, and Nix executor leaks from reusable defaults. [covers=architecture.modularity.next-decomposition-standalone-and-compatibility-proof]

- [ ] I10 Save compatibility checks for affected jobs/CI consumers, handlers, CLI/dogfood paths, and executor crates named in the manifest. [covers=architecture.modularity.next-decomposition-standalone-and-compatibility-proof]

## 5. Trust, crypto, and secrets first slice

- [ ] I11 Extract or gate pure Shamir/GF/HKDF/share-chain/reconfiguration state logic away from Raft/Iroh/secrets-service runtime shells. [covers=architecture.modularity.next-decomposition-standalone-and-compatibility-proof]

- [ ] I12 Add property-style pure tests, malformed share/key negative tests, downstream metadata, and compatibility checks for trust/secrets runtime consumers. [covers=architecture.modularity.next-decomposition-standalone-and-compatibility-proof]

## 6. Testing harness first slice

- [ ] I13 Inventory reusable simulation/workload/assertion/helper ownership and split reusable helpers from Aspen cluster bootstrap helpers. [covers=architecture.modularity.next-decomposition-standalone-and-compatibility-proof]

- [ ] I14 Add fixture self-tests, negative app-bootstrap dependency checks, and compatibility checks for existing madsim/network/patchbay suites. [covers=architecture.modularity.next-decomposition-standalone-and-compatibility-proof]

## 7. Verification discipline

- [ ] V2 Before checking any implementation task complete, update `verification.md` with exact task text, changed files, source diff artifact, and durable evidence paths. [covers=architecture.modularity.next-decomposition-standalone-and-compatibility-proof]

- [ ] V3 Run `scripts/openspec-preflight.sh decompose-next-five-crate-families-implementation` and save output before claiming completion of any phase. [covers=architecture.modularity.next-decomposition-policy-covers-wave]
