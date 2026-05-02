# Extraction Manifest: Trust, Crypto, and Secrets

## Candidate

- **Family**: `trust-crypto-secrets`
- **Canonical class**: `leaf type/helper` plus `service library`
- **Crates**: `aspen-trust`, `aspen-crypto`, reusable pure/state-machine surfaces in `aspen-secrets`, runtime consumer coverage for `aspen-secrets-handler`
- **Intended audience**: systems that need deterministic trust/crypto/secrets state logic without Aspen Raft, Iroh, Redb, or secrets-service runtime shells.
- **Public API owner**: architecture-modularity
- **Readiness state**: `workspace-internal`; first blocker complete as of `complete-trust-crypto-first-blocker`.

## Package metadata

- **Documentation entrypoint**: trust/secrets Rustdoc, `docs/trust-quorum.md`, and this manifest.
- **License policy**: AGPL-3.0-or-later until human license strategy changes.
- **Repository/homepage policy**: Aspen monorepo path until publication policy is decided.
- **Semver policy**: no external semver guarantee; serialized/share formats become compatibility contracts after tests/goldens land.
- **Publication policy**: no publishable/repo-split state in this change.

## Feature contract

| Surface | Reusable default | Runtime/adapter boundary |
| --- | --- | --- |
| Trust crypto | Shamir/GF/HKDF/share-chain helpers with explicit randomness/time inputs. | Iroh trust-share exchange, peer probing, cluster bootstrap. |
| Reconfiguration state | deterministic membership/share/decryption-key-selection state machine inputs and outputs. | Raft log application, storage transactions, node shutdown/expungement effects. |
| Secrets crypto | pure encryption/decryption/key-selection helpers and migration planning. | Redb storage, secrets service, handler/client runtime, SOPS file IO. |
| General crypto | BLAKE3/hash helpers and key utilities where transport-free; `aspen-crypto` defaults to this surface. | node identity lifecycle helpers behind `aspen-crypto/identity`; concrete Iroh endpoint/runtime helpers stay outside reusable defaults. |

## Dependency decisions

- Pure logic must not depend on Raft, Iroh endpoint construction, Redb storage, handler registries, node bootstrap, or ambient wall-clock/randomness.
- Cryptographic dependencies such as HKDF, AEADs, zeroize, secrecy, and BLAKE3 are allowed when they are the crate purpose.
- Runtime `aspen-secrets-handler` remains a compatibility consumer, not reusable core.

## Compatibility plan

- Keep runtime trust/secrets behavior compatible through adapters around the pure core.
- Representative consumers: trust reconfiguration paths, cluster bootstrap, `aspen-secrets`, `aspen-secrets-handler`, CLI secret flows, NixOS trust/secrets VM tests.
- Any moved pure helper needs old path/new path, owner, test, and removal/retention criteria.

## Downstream fixture plan

- Fixture reconstructs valid shares and reconfiguration decisions from deterministic inputs.
- Fixture selects decryption keys across epochs without storage/network runtime.
- Negative fixture rejects malformed share, wrong epoch, insufficient quorum, corrupted digest, and stale key cases.

## Verification rails

- Positive downstream: pure-core unit/property tests, downstream fixture metadata/check/test, runtime compatibility checks.
- Negative boundary: dependency-boundary malformed share/key/quorum/digest tests and checker mutation for Raft/Iroh/Redb/handler dependency leaks.
- Compatibility: trust/secrets focused tests and VM evidence named by implementation tasks.

## First blocker

I12 adds property-style pure trust tests, malformed share/digest negative coverage, downstream metadata, and runtime handler compatibility evidence. I11 inventory started the isolation decision. `aspen-trust` is the first reusable pure trust surface for Shamir/GF256/HKDF/share-chain/envelope/reconfiguration helpers; it checks cleanly without Aspen Raft, Redb, handler registry, or node bootstrap shells. `aspen-secrets --no-default-features` remains buildable with SOPS/client/transport/trust integrations feature-gated. The first blocker is now complete: `aspen-crypto` defaults to the transport-free cookie/hash helper surface, and node identity lifecycle utilities live behind the explicit `identity` feature using `iroh-base` key types rather than concrete `iroh` endpoint/runtime dependencies. The aggregate family remains `workspace-internal` until trust/secrets split policy, publication policy, and any remaining runtime-adapter ownership reviews are resolved. Evidence is recorded under `openspec/changes/complete-trust-crypto-first-blocker/evidence/`.
