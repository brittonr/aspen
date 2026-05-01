# Extraction Manifest: Trust, Crypto, and Secrets

## Candidate

- **Family**: `trust-crypto-secrets`
- **Canonical class**: `leaf type/helper` plus `service library`
- **Crates**: `aspen-trust`, `aspen-crypto`, reusable pure/state-machine surfaces in `aspen-secrets`, runtime consumer coverage for `aspen-secrets-handler`
- **Intended audience**: systems that need deterministic trust/crypto/secrets state logic without Aspen Raft, Iroh, Redb, or secrets-service runtime shells.
- **Public API owner**: owner needed
- **Readiness state**: `workspace-internal`

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
| General crypto | BLAKE3/hash helpers and key utilities where transport-free. | concrete Iroh identity/runtime helpers. |

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
- Negative boundary: malformed share/key/quorum/digest tests and checker mutation for Raft/Iroh/Redb/handler dependency leaks.
- Compatibility: trust/secrets focused tests and VM evidence named by implementation tasks.

## First blocker

Isolate Shamir/GF/HKDF/share-chain helpers and trust reconfiguration/decryption-key-selection state machines behind deterministic inputs and outputs.
