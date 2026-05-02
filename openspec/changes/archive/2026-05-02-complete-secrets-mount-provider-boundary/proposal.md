# Complete secrets mount provider boundary

## Why

The previous trust/crypto/secrets slices made the handler default portable and moved secrets DTO/state contracts into `aspen-secrets-core`, but `aspen-secrets-handler::SecretsService` still accepted and stored the concrete `aspen_secrets::MountRegistry`. That leaked the runtime registry/cache implementation into the handler service boundary even though handler code only needs mounted store resolution.

## What Changes

- Add `aspen_secrets::SecretsMountProvider` as the narrow mounted-store provider contract.
- Implement the provider for `MountRegistry` by delegating to the existing bounded get-or-create methods.
- Change `aspen-secrets-handler::SecretsService` to store `Arc<dyn SecretsMountProvider>` instead of concrete `Arc<MountRegistry>`.
- Preserve runtime/node compatibility by continuing to pass `Arc<MountRegistry>` at construction sites.

## Scope

In scope: service-boundary decoupling, compatibility checks, dependency/source guards, and OpenSpec evidence.

Out of scope: changing store implementations, moving async store traits into `aspen-secrets-core`, changing wire DTOs, or splitting Redb/trust runtime storage.

## Verification

Focused checks cover `aspen-secrets`, `aspen-secrets-handler`, node secrets integration, mount-registry tests, handler tests, dependency guards, source guards, OpenSpec validation, markdownlint, and `git diff --check`.
