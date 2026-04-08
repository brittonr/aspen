## 1. Reconfiguration State Machine

- [x] 1.1 Define `ReconfigCoordinator` sans-IO state machine in `aspen-trust/src/reconfig.rs` with states: `CollectingOldShares`, `Preparing`, `Committed`
- [x] 1.2 Define `ReconfigCtx` trait with `send_get_share()`, `propose_new_config()`, `connected_members()`
- [x] 1.3 Implement `CollectingOldShares` state: track received shares per node, validate digests, transition to `Preparing` when K valid shares collected
- [x] 1.4 Implement `Preparing` state: reconstruct old secret, generate new secret, split into new shares, build encrypted chain, produce Raft proposal
- [x] 1.5 Implement timeout handling: `on_timeout()` method returns error if share collection stalls
- [x] 1.6 Build `TestReconfigCtx` for deterministic testing of the state machine

## 2. Encrypted Secret Chain

- [x] 2.1 Define `EncryptedSecretChain` type in `aspen-trust/src/chain.rs`: `salt: [u8; 32]`, `data: Vec<u8>`, epoch metadata
- [x] 2.2 Implement `encrypt_chain(prior_secrets: BTreeMap<u64, Secret>, new_secret: &Secret, cluster_id, epoch) -> EncryptedSecretChain` using ChaCha20Poly1305 with HKDF-derived key
- [x] 2.3 Implement `decrypt_chain(chain: &EncryptedSecretChain, secret: &Secret, cluster_id, epoch) -> BTreeMap<u64, Secret>`
- [x] 2.4 Test: encrypt then decrypt roundtrip recovers all prior secrets
- [x] 2.5 Test: tampered ciphertext fails authentication
- [x] 2.6 Test: wrong secret fails decryption
- [ ] 2.7 Add Verus spec for chain length invariant: chain at epoch N contains exactly N-1 prior secrets

## 3. Raft Integration

- [ ] 3.1 Add `TrustReconfiguration` variant to Raft log entry types: contains new shares (per-node), encrypted chain, epoch, digests
- [ ] 3.2 Hook into openraft `change_membership` completion callback to trigger trust reconfiguration
- [ ] 3.3 In Raft leader: start `ReconfigCoordinator`, send `GetShare` requests via Iroh to old members
- [ ] 3.4 Handle `GetShare` responses: feed into coordinator state machine, on transition to `Preparing` propose the `TrustReconfiguration` entry
- [ ] 3.5 In state machine apply handler: each node extracts its new share and stores it, updates epoch, stores encrypted chain
- [ ] 3.6 Handle leader change during reconfiguration: new leader detects pending reconfiguration flag and restarts

## 4. Share Collection via Iroh

- [ ] 4.1 Add `TRUST_ALPN` constant to `aspen-transport` for trust protocol messages
- [ ] 4.2 Implement `GetShareRequest { epoch: u64 }` and `ShareResponse { epoch: u64, share: Share }` message types
- [ ] 4.3 Implement request handler: on `GetShareRequest`, load share from redb, validate requester is in current config, respond
- [ ] 4.4 Implement response handler: validate share digest, feed into `ReconfigCoordinator`
- [ ] 4.5 Add timeout: if fewer than K shares collected within `TRUST_SHARE_COLLECTION_TIMEOUT_MS` (default: 10_000), abort

## 5. Testing

- [ ] 5.1 Unit test the `ReconfigCoordinator` state machine with `TestReconfigCtx`: full flow from init through share collection to commit
- [ ] 5.2 Property test: for random membership changes on random cluster sizes, reconfiguration always produces valid shares that reconstruct the new secret
- [ ] 5.3 Property test: encrypted chain at epoch N always contains N-1 decodable secrets
- [ ] 5.4 Integration test: 3-node cluster → add voter → verify new shares and old chain → remove voter → verify rotation
- [ ] 5.5 Crash test: kill leader during share collection, verify new leader completes reconfiguration
