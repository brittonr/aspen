## Context

`aspen-secrets` storage APIs already use `aspen_traits::KeyValueStore` and `aspen-kv-types`, but `aspen-secrets-handler` still had two independent runtime couplings: it unconditionally enabled `aspen-rpc-core/runtime-context` for factory registration and imported KV contracts through `aspen-core`.

## Decisions

### 1. Gate the runtime factory adapter

**Choice:** Put `SecretsHandlerFactory`, `ClientProtocolContext`, `HandlerFactory`, `RequestHandler`, and `ServiceHandler` re-exports behind a new `runtime-adapter` feature.

**Rationale:** The factory is the only part of `aspen-secrets-handler` that needs full node runtime context. The executor and `SecretsService` can compile against lightweight RPC/KV contracts.

**Alternative:** Keep the feature unconditional and accept runtime dependencies. Rejected because it hides Redb/Raft behind a no-default handler build.

### 2. Use lightweight KV contracts in the handler

**Choice:** Replace `aspen_core::KeyValueStore` and `aspen_core::ReadRequest` with `aspen_traits::KeyValueStore` and `aspen_kv_types::ReadRequest`.

**Rationale:** This matches `aspen-secrets` after the KV traits boundary and avoids broad compatibility-surface coupling.

### 3. Preserve runtime compatibility through the aggregate bundle

**Choice:** Make `aspen-rpc-handlers`' `secrets` feature enable `aspen-secrets-handler/runtime-adapter`.

**Rationale:** Node/runtime code still uses the factory registration path; compatibility should remain opt-in at the aggregate runtime boundary rather than in the portable handler default.
