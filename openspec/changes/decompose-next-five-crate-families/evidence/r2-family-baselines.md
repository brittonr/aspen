# Per-Family Dependency and Source Baselines

Source: selected `crates/*/Cargo.toml` dependency snapshots captured in `r2-direct-dependency-snapshot.txt`, current extraction manifests, and `docs/crate-extraction.md`.

## foundational-types

- **Crates**: `aspen-storage-types`, `aspen-traits`, `aspen-cluster-types`, `aspen-hlc`, `aspen-time`, `aspen-constants`.
- **Direct dependency shape**:
  - `aspen-storage-types`: `serde`, `bincode`; current blocker is not the manifest dependencies but the Redb table definition/public storage concern documented in `docs/crate-extraction/foundational-types.md`.
  - `aspen-traits`: `aspen-cluster-types`, `aspen-kv-types`, optional `async-trait`.
  - `aspen-cluster-types`: `serde`, `thiserror`, optional `iroh-base` key helpers.
  - `aspen-hlc`: `uhlc` without defaults, `blake3`, `serde` alloc/derive.
  - `aspen-time` and `aspen-constants`: no normal dependencies.
- **Transitive app/runtime leak paths to watch**: `aspen-storage-types` must not keep Redb/libc in the foundational graph through table definitions; `aspen-traits` representative consumers can re-enable `aspen-cluster-types` defaults; `aspen-cluster-types[iroh]` must stay opt-in.
- **Representative consumers**: `aspen-core`, `aspen-core-shell`, `aspen-coordination`, `aspen-commit-dag`, `aspen-kv-branch`, `aspen-jobs`, `aspen-testing-core`.
- **Existing rails**: `cargo check -p aspen-core --no-default-features`, `cargo check -p aspen-core-no-std-smoke`, `scripts/check-aspen-core-no-std-boundary.py`, and prior no-std evidence documented in AGENTS/napkin.
- **First blockers**: split `SM_KV_TABLE` / `redb::TableDefinition` from portable storage types; split/prove narrower `aspen-traits` KV capability traits and default-feature leak absence.

## auth-ticket

- **Crates**: `aspen-auth-core`, `aspen-auth`, `aspen-ticket`, `aspen-hooks-ticket`.
- **Direct dependency shape**:
  - `aspen-auth-core`: `iroh-base` key type, `serde`, `postcard`, `base64`, `blake3`, `thiserror`.
  - `aspen-auth`: runtime shell over `aspen-auth-core` plus `aspen-core-shell`, HMAC/SHA2, randomness, tracing, and async traits.
  - `aspen-ticket` / `aspen-hooks-ticket`: cluster address types plus iroh ticket/postcard/serde helpers; selected runtime iroh helpers must stay feature-gated where applicable.
- **Transitive app/runtime leak paths to watch**: portable consumers should not depend on runtime `aspen-auth`; HMAC/verifier/revocation/storage APIs must not enter `aspen-auth-core`; hook config/event schema should stay separate from hook ticket URLs unless runtime `aspen-hooks` is explicitly used.
- **Representative consumers**: `aspen-client-api`, `aspen-cli`, `aspen-rpc-core`, `aspen-rpc-handlers`, `aspen-hooks`, `aspen-ci`, Forge/CI ticket paths.
- **Existing rails**: auth-core/token unit tests, client-api postcard/golden serialization tests, and hook ticket URL parsing tests.
- **First blockers**: migrate portable consumers to canonical `aspen-auth-core` / `aspen-hooks-ticket`; document retained `aspen-auth` compatibility re-exports; add malformed token/ticket negative tests.

## jobs-ci-core

- **Crates**: `aspen-jobs`, `aspen-jobs-protocol`, `aspen-jobs-guest`, `aspen-jobs-worker-blob`, `aspen-jobs-worker-maintenance`, `aspen-jobs-worker-replication`, `aspen-jobs-worker-shell`, `aspen-jobs-worker-sql`, `aspen-ci-core`, `aspen-ci`, `aspen-ci-executor-shell`, `aspen-ci-executor-vm`, `aspen-ci-executor-nix`.
- **Direct dependency shape**:
  - `aspen-ci-core` is the lightest schema/config surface with `serde`, `serde_json`, `schemars`, `snafu`, `uuid`, and `chrono`.
  - `aspen-jobs` still depends on `aspen-core`, `aspen-coordination`, concrete Iroh/Tokio/runtime utilities, optional blob/branch workers, and process/storage oriented helpers.
  - `aspen-ci` depends on root runtime/service crates (`aspen-core`, `aspen-forge`, `aspen-jobs`, `aspen-blob`, `aspen-auth`, `aspen-ticket`) plus executor crates and optional Nickel/SNIX evaluation.
- **Transitive app/runtime leak paths to watch**: scheduler/config/run-state logic can pull root `aspen-core`, worker runtime, shell/VM/Nix executors, process spawning, handler registries, and concrete Iroh through `aspen-jobs`/`aspen-ci` unless reusable contracts move to lighter crates/features.
- **Representative consumers**: `aspen-ci`, `aspen-jobs`, `aspen-job-handler`, `aspen-ci-handler`, `aspen-dogfood`, `aspen-cli`, executor crates, Forge CI trigger paths.
- **Existing rails**: `cargo test -p aspen-ci --features nickel`, Forge/CI trigger tests, dogfood CI VM tests, job orchestrator tests, executor-focused unit tests.
- **First blockers**: define canonical ownership for schema, scheduler, run-state, artifact metadata, and Nickel config contracts; gate worker runtime/executor/process-spawning/node/handler shells behind adapter features or crates.

## trust-crypto-secrets

- **Crates**: `aspen-trust`, `aspen-crypto`, reusable pure/state-machine surfaces in `aspen-secrets`, and runtime `aspen-secrets-handler` consumer coverage.
- **Direct dependency shape**:
  - `aspen-trust`: crypto primitives (`chacha20poly1305`, `hkdf`, `sha3`, `zeroize`, `secrecy`), randomness, async traits, serde, tokio sync, snafu.
  - `aspen-crypto`: `blake3`, `hex`, `rand`, `thiserror`, `tokio`, and concrete `iroh` helpers.
  - `aspen-secrets`: root `aspen-core`, runtime `aspen-auth`, `aspen-crypto`, SOPS/age and certificate crates, concrete `iroh`, async runtime, and serialization.
- **Transitive app/runtime leak paths to watch**: pure Shamir/GF/HKDF/share-chain and reconfiguration logic can be obscured by Raft log application, Iroh trust-share exchange, startup peer probing, redb secrets storage, and secrets-service runtime shells.
- **Representative consumers**: trust reconfiguration paths in `aspen-raft`/cluster bootstrap, `aspen-secrets`, `aspen-secrets-handler`, CLI secret flows, NixOS trust/secrets VM tests.
- **Existing rails**: trust quorum tests, secrets-at-rest rotation/migration tests, VM trust/secrets tests, and documented trust-quorum invariants.
- **First blockers**: isolate pure Shamir/GF/HKDF/share-chain helpers and reconfiguration/decryption-key-selection state machines behind deterministic inputs/outputs; keep runtime shells as adapters.

## testing-harness

- **Crates**: `aspen-testing-core`, `aspen-testing`, `aspen-testing-fixtures`, `aspen-testing-madsim`, `aspen-testing-network`, `aspen-testing-patchbay`.
- **Direct dependency shape**:
  - `aspen-testing-core` depends on lightweight type/trait crates plus async runtime utilities.
  - `aspen-testing` brings `aspen-core-shell`, fixtures, optional madsim/network, optional raft/blob/cluster/jobs/coordination/forge/CI integration, Iroh, Tokio, OpenRaft, and simulation dependencies.
  - `aspen-testing-madsim` brings core test types plus `aspen-raft`, madsim/mad-turmoil, OpenRaft, Iroh, Tokio, serde, rand, tracing, and parking_lot.
- **Transitive app/runtime leak paths to watch**: generic simulation/workload/assertion helpers can pull root `aspen`, node bootstrap, handler registries, concrete cluster runtime dependencies, binary shells, patchbay namespaces, or Iroh runtime unless split/gated.
- **Representative consumers**: madsim suites, network tests, patchbay suites, NixOS VM tests, downstream extraction fixtures.
- **Existing rails**: nextest quick/default profiles, madsim tests, patchbay harness commands, `scripts/test-harness.sh check`, generated inventory freshness checks.
- **First blockers**: inventory helper ownership, then split reusable simulation/workload/assertion helpers from Aspen cluster bootstrap, node config, concrete transport, and binary shell helpers.
