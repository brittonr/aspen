## Why

Molten's artifact registry and remote execution model rely on trusting installed artifacts enough to run them or use them as policy, schema, migration, and handler code. Content addressing proves identity, not origin, review status, or safe build process.

## What Changes

- Define supply-chain provenance requirements for executable, policy, schema, migration, and documentation artifacts.
- Attach builder attestations, source refs, dependency closures, toolchain refs, review records, signatures, and Octet/Valence evidence to artifacts.
- Support reproducible build records and verification receipts where possible.
- Gate artifact installation and execution on provenance policy.
- Distinguish trusted source, trusted builder, reviewed artifact, reproducible artifact, sandbox-only artifact, and denied artifact states.
- Integrate provenance with remote sync, catalog/MCP, upgrade sessions, deterministic replay, and evaluation cache.

## Impact

Molten can decide not just what an artifact is, but why it is trusted for a given use. The first milestone can attach provenance refs and review receipts to artifact metadata and require them for Wasm/Steel/policy artifact installation.
