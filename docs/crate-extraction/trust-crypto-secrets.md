# Extraction Manifest: Trust, Crypto, and Secrets

## Candidate

- **Family**: `trust-crypto-secrets`
- **Canonical class**: `leaf type/helper` plus `service library`
- **Crates**: `aspen-trust`, `aspen-crypto`, reusable pure/state-machine surfaces in `aspen-secrets`, runtime consumer coverage for `aspen-secrets-handler`
- **Intended audience**: systems that need deterministic trust/crypto/secrets state logic without Aspen Raft, Iroh, Redb, or secrets-service runtime shells.
- **Public API owner**: architecture-modularity
- **Readiness state**: `workspace-internal`; owner/public API review complete and promotion deferred pending real checker coverage plus trust/secrets stability-policy blockers.

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
| Secrets crypto | pure encryption/decryption/key-selection helpers, migration planning, and token data parsing via `aspen-auth-core`. | Redb storage, secrets service, handler/client runtime, SOPS file IO, `aspen-auth` verifier/builder helpers behind `aspen-secrets/auth-runtime`. |
| General crypto | BLAKE3/hash helpers and key utilities where transport-free; `aspen-crypto` defaults to this surface. | node identity lifecycle helpers behind `aspen-crypto/identity`; concrete Iroh endpoint/runtime helpers stay outside reusable defaults. |

## Dependency decisions

- Pure logic must not depend on Raft, Iroh endpoint construction, Redb storage, handler registries, node bootstrap, or ambient wall-clock/randomness.
- `aspen-secrets` default token parsing depends on `aspen-auth-core`; `aspen-auth` runtime verifier/builder helpers require the explicit `auth-runtime` feature.
- Cryptographic dependencies such as HKDF, AEADs, zeroize, secrecy, and BLAKE3 are allowed when they are the crate purpose.
- Runtime `aspen-secrets-handler` is a compatibility consumer; its default executor surface stays portable, while factory registration requires the explicit `runtime-adapter` feature.

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

I12 adds property-style pure trust tests, malformed share/digest negative coverage, downstream metadata, and runtime handler compatibility evidence. I11 inventory started the isolation decision. `aspen-trust` is the first reusable pure trust surface for Shamir/GF256/HKDF/share-chain/envelope/reconfiguration helpers; it checks cleanly without Aspen Raft, Redb, handler registry, or node bootstrap shells. `aspen-secrets --no-default-features` remains buildable with SOPS/client/transport/trust integrations feature-gated. The first blocker is now complete: `aspen-crypto` defaults to the transport-free cookie/hash helper surface, and node identity lifecycle utilities live behind the explicit `identity` feature using `iroh-base` key types rather than concrete `iroh` endpoint/runtime dependencies. The aggregate family remains `workspace-internal`; the first blocker is complete, but the owner/public API review requires real crate-level checker coverage, explicit `aspen-trust` async/default dependency policy, and trust/secrets serialization-contract evidence before promotion. First-blocker evidence is recorded under `openspec/changes/archive/2026-05-02-complete-trust-crypto-first-blocker/evidence/`.

## Auth runtime boundary

`d2a4d4ba1 Keep secrets auth runtime optional` removes the `aspen-auth` runtime shell from the `aspen-secrets` default token parsing path. `SecretsProvider::get_token` and config token parsing now use `aspen-auth-core::CapabilityToken`, while `SecretsManager::build_token_verifier` and `SecretsManager::build_token_builder` require `aspen-secrets/auth-runtime`. The root node `secrets` feature enables that runtime feature to preserve bootstrap compatibility. Evidence is recorded under `openspec/changes/archive/2026-05-02-complete-secrets-auth-runtime-boundary/evidence/`.

## KV traits boundary

`97f0518e6 Use lightweight KV traits in secrets storage` removes direct `aspen-core` usage from `crates/aspen-secrets`. `AspenSecretsBackend`, `MountRegistry`, and the SOPS runtime KV manager now expose `aspen_traits::KeyValueStore` and import request/result/error contracts from `aspen-kv-types`; existing runtime stores continue to compile through the compatibility re-exported trait. Evidence is recorded under `openspec/changes/archive/2026-05-02-complete-secrets-kv-traits-boundary/evidence/`.

## Secrets handler runtime adapter boundary

`ada266bb7 Gate secrets handler runtime adapter` makes `aspen-secrets-handler --no-default-features` compile against the reusable RPC executor surface plus `aspen-traits`/`aspen-kv-types` instead of `aspen-core` and the full `aspen-rpc-core/runtime-context` graph. `SecretsHandlerFactory` and the runtime-context re-exports are now behind `aspen-secrets-handler/runtime-adapter`; `aspen-rpc-handlers/secrets` enables that adapter to preserve node registration compatibility. Evidence is recorded under `openspec/changes/archive/2026-05-02-complete-secrets-handler-runtime-adapter-boundary/evidence/`.

## Secrets core type boundary

`2b3571242 Extract secrets core type contracts` adds `aspen-secrets-core` as the owner of dependency-light secrets constants plus KV, Transit, and PKI DTO/state contracts. `aspen-secrets` now depends on that core crate and preserves historical `constants`, `kv::types`, `transit::types`, `pki::types`, and root exports through compatibility re-export shims. Runtime stores, cryptographic execution, SOPS file IO, auth runtime helpers, trust integration, and handler adapters remain outside the core crate. Evidence is recorded under `openspec/changes/archive/2026-05-02-complete-secrets-core-type-boundary/evidence/`.

## Secrets mount provider boundary

`1691dc2b3 Decouple secrets handler mount provider` adds `aspen_secrets::SecretsMountProvider` as the mounted-store resolution contract for PKI, KV, and Transit stores. `MountRegistry` implements the provider by delegating to its existing bounded `get_or_create_*` methods, while `aspen-secrets-handler::SecretsService` now stores `Arc<dyn SecretsMountProvider>` instead of concrete `Arc<MountRegistry>`. Runtime/node call sites continue passing `Arc<MountRegistry>` for compatibility, but the handler service boundary no longer depends on the concrete registry/cache implementation. Evidence is recorded under `openspec/changes/archive/2026-05-02-complete-secrets-mount-provider-boundary/evidence/`.

## Owner/public API review

`review-trust-crypto-secrets-public-api` records the aggregate readiness decision after the transport-free crypto, auth-runtime, KV traits, handler runtime-adapter, secrets-core type, and mount-provider boundary slices. The review keeps the family at `workspace-internal` because the current readiness checker still maps the family to an aggregate pseudo-candidate and warns that direct package dependency checks are deferred.

Canonical reusable surfaces are `aspen-crypto` default/no-default helpers and `aspen-secrets-core` type/state contracts. Selected `aspen-trust` pure helper/state surfaces remain candidates, but default async/Tokio service APIs and trust serialization contracts need explicit policy before readiness promotion. `aspen-secrets` remains a service/runtime implementation crate for now, and `aspen-secrets-handler` remains a compatibility/runtime consumer. Evidence is recorded under `openspec/changes/archive/2026-05-03-review-trust-crypto-secrets-public-api/evidence/` while active.
