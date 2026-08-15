## Context

Molten live Iroh transport needs a stable endpoint identity across restarts. If a node silently generates a new endpoint secret on every boot, peer bootstrap tickets, handoff bundles, diagnostics, and operator expectations can drift even when the logical node id and state root are unchanged. Conversely, accepting any presented endpoint key would let transport identity blur into authority.

The accepted persistent-node-identity spec already requires canonical identity records, resolution order, no-identity-authority, secret redaction, file backends, drift detection, receipts, config contracts, peer bootstrap refs, replay refs, and restart/property tests. This change adapts main's concrete Iroh persistence pattern into Molten's canonical evidence model.

## Design

### Pure resolution core

A pure identity-resolution core should decide from explicit in-memory facts:

- node id and state-root/profile identity;
- explicit key metadata if supplied;
- secret-backend availability metadata;
- persisted-file metadata and public-key digest when available;
- existing identity record refs and expected endpoint public key;
- policy refs for generation, recovery, and rotation;
- platform capability facts for restricted permissions.

The core returns `load-explicit`, `load-backend`, `load-file`, `generate-and-persist`, `rotate`, or `deny` with diagnostics. It must not read files, generate keys, set permissions, inspect environment variables, or log secrets.

### Shell-owned secret handling

The startup shell owns secret IO:

```text
read profile/config facts
  -> read secret backend or file metadata and bytes as allowed
  -> call pure resolution core with redacted facts
  -> generate or rotate only after pass decision
  -> persist with owner-only permissions where supported
  -> emit redacted identity/startup receipt
```

Secret bytes remain shell-local. Receipts may bind endpoint public key, key-source class, secret-backend ref, file ref or redacted path class, permission status, policy refs, previous identity ref, and rotation/recovery receipt refs. Receipts must not include private key bytes, bearer tokens, raw ticket material, or unrestricted local paths when redaction policy forbids them.

### Drift and rotation

If an existing node identity record binds one endpoint public key and startup observes a different key for the same node/state scope, startup must deny unless rotation or recovery policy evidence explicitly admits the change. Rotation receipts bind old public identity, new public identity, reason, policy refs, operator authority refs, and revocation or peer-refresh obligations.

### Peer and replay boundary

Peer bootstrap handshakes and replay/startup evidence should include node identity refs and endpoint public identity refs. Those refs support transport binding and diagnostics only. Operation authority still requires capability, authority, policy, resource, replay/idempotency, and subsystem-specific evidence.

### Fixtures

Positive fixtures should cover first boot generation, restart with the same state root preserving endpoint id, explicit-key precedence, secret-backend load, rotation with admitted policy, and redacted receipts. Negative fixtures should cover malformed key material, missing backend in fail-closed mode, unsafe file permissions, endpoint drift without rotation, stale rotation evidence, private-key receipt leakage, and endpoint identity used as authority.

## Non-goals

- Do not define a fleet-wide PKI or trust anchor in this slice.
- Do not store private key material in canonical Preserves receipts.
- Do not let endpoint public keys replace peer admission, capability grants, or authority contexts.
- Do not require replay to access private key material.