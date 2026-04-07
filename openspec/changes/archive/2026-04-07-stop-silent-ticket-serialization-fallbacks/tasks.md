## 1. Classify the affected encoders

- [x] 1.1 Inventory ticket and signed-identity serializers that currently use `unwrap_or_default()` or equivalent silent fallback
- [x] 1.2 Split them into two groups: APIs that can return `Result` and trait-imposed `to_bytes() -> Vec<u8>` implementations that cannot
- [x] 1.3 Confirm which of those payloads are externally shared or authority-bearing and prioritize them first

## 2. Fix fallible wrapper APIs

- [x] 2.1 Change `AutomergeSyncTicket` construction and serialization helpers to surface encoding failures explicitly
- [x] 2.2 Update affected callers and tests to handle the new `Result`-returning API
- [x] 2.3 Audit other wrapper types around capability tokens or signed identities for the same silent-fallback pattern

## 3. Fix trait-constrained ticket encoders

- [x] 3.1 Replace silent default-byte fallbacks in `Ticket::to_bytes()` implementations with fail-fast handling and invariant documentation
- [x] 3.2 Review client ticket and signed cluster ticket implementations to ensure they never emit empty payloads on serializer failure
- [x] 3.3 Apply the same rule to other externally shared identity/ticket encoders that cannot return `Result`

## 4. Add regression coverage

- [x] 4.1 Add a test showing oversized capability-token input is rejected at `AutomergeSyncTicket` creation time
- [x] 4.2 Add round-trip tests for client and signed ticket encoders after the fallback removal
- [x] 4.3 Add a targeted test or assertion that no empty payload is produced on the old silent-fallback path

## 5. Verify the resulting API surface

- [x] 5.1 Run crate tests for the touched ticket and identity modules
- [x] 5.2 Check any CLI or handler code that mints these tickets still reports actionable errors to the caller
- [x] 5.3 Document any remaining internal-only serializers left unchanged and why they are lower risk

## Unchanged internal-only serializers (lower risk)

The following `unwrap_or_default()` postcard/serde serialization sites were reviewed and intentionally left unchanged. They are internal-only: their output never leaves the process as an externally shared ticket, identity document, or authority-bearing payload.

| File | Type | Why unchanged |
|------|------|---------------|
| `aspen-forge/src/gossip/types.rs` | `Announcement::to_bytes()`, `SignedAnnouncement::to_bytes()` | Gossip wire format between cluster peers, not user-shared. Recipients deserialize and reject malformed messages. |
| `aspen-forge/src/types.rs` | `ForgeObject<T>::to_bytes()` | Internal content-address computation. Empty bytes produce a different BLAKE3 hash, but the object is never stored or shared — the caller uses the hash to address the real object. |
| `aspen-forge/src/identity/mod.rs` | `ForgeRepoIdentity::repo_id()` | Computes a BLAKE3 repo ID from the identity struct. Used as an internal lookup key. |
| `aspen-forge/src/resolver.rs` | `RefEntry` serialization | Federation sync object packaging. Internal to the resolver; malformed entries are rejected by the sync protocol. |
| `aspen-docs/src/origin.rs` | `OriginTracker::to_bytes()` | JSON serialization for KV storage. Read back by `from_bytes()` which returns `None` on failure. Not shared externally. |
| `aspen-forge-handler/src/executor.rs:1503` | `serde_json::to_value(&payload)` | Internal hook event dispatch payload. Consumed by in-process hook handlers only. |
