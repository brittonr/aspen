## 1. Shamir Secret Sharing Core

- [x] 1.1 Create `crates/aspen-trust/` crate with `Cargo.toml` (deps: `sha3`, `zeroize`, `secrecy`, `subtle`, `rand`, `serde`)
- [x] 1.2 Implement GF(2^8) arithmetic in `src/gf256.rs` — multiplication table, polynomial evaluation (port from Oxide's `gfss` or evaluate `sharks` crate)
- [x] 1.3 Implement `split_secret(secret: &[u8; 32], threshold: u8, total: u8) -> Result<Vec<Share>, SplitError>` in `src/shamir.rs`
- [x] 1.4 Implement `reconstruct_secret(shares: &[Share]) -> Result<[u8; 32], ReconstructError>` in `src/shamir.rs`
- [x] 1.5 Implement `Share` type (33 bytes: 1-byte x-coordinate + 32-byte y-values) with `Zeroize` and `ZeroizeOnDrop` derives
- [x] 1.6 Implement `share_digest(share: &Share) -> Sha3_256Digest` for tamper detection
- [x] 1.7 Property test: for all K in 2..10 and N >= K, split then reconstruct with K shares recovers original
- [x] 1.8 Property test: reconstruct with fewer than K shares produces a different value or error
- [x] 1.9 Add Verus spec in `verus/shamir_spec.rs` for threshold bounds (K >= 1, K <= N, N <= 255)

## 2. Cluster Secret and Key Derivation

- [x] 2.1 Define `ClusterSecret` type in `src/secret.rs` — 32-byte `SecretBox`, `Zeroize` on drop, `OsRng` generation, constant-time equality
- [x] 2.2 Implement `Threshold(u8)` newtype with `default_for_cluster_size(n: u32) -> Threshold` returning `(n/2) + 1`
- [x] 2.3 Implement HKDF-SHA3-256 key derivation in `src/kdf.rs`: `derive_key(secret, context: &[u8], cluster_id: &[u8], epoch: u64) -> [u8; 32]`
- [x] 2.4 Define standard context constants: `CONTEXT_SECRETS_AT_REST`, `CONTEXT_TRANSIT_KEYS`, `CONTEXT_RACK_SECRETS`
- [x] 2.5 Test: different contexts produce different keys; same inputs produce same key
- [x] 2.6 Test: derived key is never all-zeros (assertion in production code too)

## 3. Share Storage in Redb

- [x] 3.1 Add `trust_shares` table definition to `aspen-raft` storage: `TableDefinition<u64, &[u8]>` (epoch → serialized share)
- [x] 3.2 Add `trust_digests` table: `TableDefinition<(u64, u64), &[u8]>` (epoch, node_id → SHA3-256 digest)
- [x] 3.3 Implement `store_share(epoch, share)` and `load_share(epoch) -> Option<Share>` on `RedbStorage`
- [x] 3.4 Implement `store_digests(epoch, digests: BTreeMap<NodeId, Digest>)` and `load_digests(epoch)` on `RedbStorage`
- [x] 3.5 Ensure `trust_shares` table is excluded from application KV scan operations

## 4. Cluster Init Integration

- [x] 4.1 Add `TrustConfig` to cluster init parameters: `enabled: bool`, `threshold: Option<u8>`
- [x] 4.2 In `init_cluster` flow (Raft leader): generate `ClusterSecret`, split into shares, compute digests
- [x] 4.3 Create a `TrustInitialized` Raft log entry type containing each node's encrypted share and all digests
- [x] 4.4 In the state machine apply handler: each node extracts its own share from the log entry and stores it
- [x] 4.5 Add `--trust` and `--trust-threshold` flags to `aspen-cli cluster init`
- [ ] 4.6 Integration test: init a 3-node cluster with trust, verify each node has a share, reconstruct the secret from 2 nodes

## 5. Feature Flag and Documentation

- [x] 5.1 Gate all trust functionality behind `trust` feature flag in `aspen-trust`, `aspen-raft`, and `aspen-core`
- [x] 5.2 Add `trust` to the `full` feature set
- [x] 5.3 Document cluster secret architecture in `docs/trust-quorum.md` with references to Oxide's design
- [x] 5.4 Add trust-quorum reference doc link to `AGENTS.md` architecture section
