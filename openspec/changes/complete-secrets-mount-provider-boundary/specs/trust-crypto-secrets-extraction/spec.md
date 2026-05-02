## ADDED Requirements

### Requirement: Secrets handler uses mount provider boundary [r[trust-crypto-secrets-extraction.secrets-mount-provider-boundary]]

The trust/crypto/secrets extraction SHALL keep the native secrets handler service boundary coupled to a mounted-store provider contract instead of the concrete runtime mount registry implementation.

#### Scenario: Handler service depends on provider trait [r[trust-crypto-secrets-extraction.secrets-mount-provider-boundary.handler-provider-trait]]

- GIVEN `aspen-secrets-handler` constructs a `SecretsService`
- WHEN it resolves PKI, KV, or Transit stores by mount
- THEN it SHALL call a provider trait exported by `aspen-secrets` rather than storing or invoking the concrete `MountRegistry` implementation directly.

#### Scenario: Mount registry remains runtime-compatible [r[trust-crypto-secrets-extraction.secrets-mount-provider-boundary.mount-registry-compatibility]]

- GIVEN existing node/runtime code constructs a concrete `MountRegistry`
- WHEN that registry is passed to `SecretsService::new`
- THEN it SHALL compile through the provider trait while preserving existing mount validation, caching, and store creation behavior.
