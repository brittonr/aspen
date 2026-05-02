# Design

## Context

`MountRegistry` is the concrete runtime cache for PKI, KV, and Transit stores. It is still appropriate in `aspen-secrets`, where it owns store creation, mount validation, resource bounds, and optional trust-aware encryption provider state. The native secrets handler, however, only needs to resolve stores by mount.

## Decision

Introduce `SecretsMountProvider` in `aspen-secrets::mount_registry`:

- `pki_store(&self, mount) -> Result<Arc<dyn PkiStore>>`
- `transit_store(&self, mount) -> Result<Arc<dyn TransitStore>>`
- `kv_store(&self, mount) -> Result<Arc<dyn KvStore>>`

`MountRegistry` implements the trait by delegating to its existing `get_or_create_*` methods, preserving all validation and mount-count limits. `aspen-secrets` re-exports the trait for downstream compatibility.

`aspen-secrets-handler::SecretsService` now stores `Arc<dyn SecretsMountProvider>` and calls the trait methods. Existing runtime call sites continue passing `Arc<MountRegistry>` through coercion to the trait object.

## Rejected Alternatives

- **Move store/provider traits to `aspen-secrets-core` now.** Rejected because that would expand the core crate beyond DTO/state contracts and introduce async trait/error coupling.
- **Gate all concrete storage behind new features now.** Rejected as broader than this service-boundary slice.
- **Refactor Redb/trust storage now.** Rejected because normal no-default handler/secrets dependencies already avoid Redb/Raft; the immediate leak is the concrete provider type.

## Compatibility

The historical `MountRegistry` type and methods remain public. `SecretsService::new` still accepts an `Arc` provider, and existing `Arc<MountRegistry>` construction sites continue compiling.
