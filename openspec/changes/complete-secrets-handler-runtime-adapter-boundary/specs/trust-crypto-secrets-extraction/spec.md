## ADDED Requirements

### Requirement: Secrets handler runtime adapter is explicit [r[trust-crypto-secrets-extraction.secrets-handler-runtime-adapter-boundary]]

The trust/crypto/secrets extraction SHALL keep `aspen-secrets-handler` default builds free of broad Aspen runtime-context and Redb/Raft storage dependencies; only the node/runtime factory adapter MAY enable the full runtime context graph.

#### Scenario: Portable handler default avoids runtime storage [r[trust-crypto-secrets-extraction.secrets-handler-runtime-adapter-boundary.default-portable]]

- GIVEN `aspen-secrets-handler` is built with no default features
- WHEN its normal dependency graph is inspected
- THEN it SHALL compile without normal dependencies on `aspen-core`, `aspen-core-shell`, `aspen-raft`, `aspen-redb-storage`, or `redb`
- AND its executor SHALL use `aspen_traits::KeyValueStore` plus `aspen-kv-types` request types instead of `aspen_core` compatibility re-exports.

#### Scenario: Runtime factory adapter remains opt-in compatible [r[trust-crypto-secrets-extraction.secrets-handler-runtime-adapter-boundary.runtime-adapter]]

- GIVEN Aspen node/runtime handler registration needs `ClientProtocolContext` and `HandlerFactory`
- WHEN `aspen-secrets-handler/runtime-adapter` or the aggregate `aspen-rpc-handlers/secrets` bundle is enabled
- THEN the secrets handler factory SHALL compile with the runtime context and preserve node secrets compatibility.
