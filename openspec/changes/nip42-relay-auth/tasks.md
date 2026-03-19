## 1. Config and constants

- [x] 1.1 Add `WritePolicy` enum (`Open`, `AuthRequired`, `ReadOnly`) with `Default` (Open), `Serialize`, `Deserialize`, `JsonSchema` to `crates/aspen-nostr-relay/src/config.rs`
- [x] 1.2 Add `write_policy: WritePolicy` and `relay_url: Option<String>` fields to `NostrRelayConfig`, defaulting to `Open` and `None`
- [x] 1.3 Add NIP-42 constants to `crates/aspen-nostr-relay/src/constants.rs`: `AUTH_EVENT_KIND: u16 = 22242`, `AUTH_CHALLENGE_BYTES: usize = 32`, `AUTH_TIMESTAMP_WINDOW_SECS: u64 = 60`

## 2. Auth verification logic

- [x] 2.1 Create `crates/aspen-nostr-relay/src/auth.rs` with `AuthState` struct holding `challenge_hex: String` and `authed_pubkey: Option<PublicKey>`
- [x] 2.2 Implement `AuthState::new()` — generates 32 random bytes, stores hex-encoded challenge, sets `authed_pubkey = None`
- [x] 2.3 Implement `AuthState::verify_auth_event(event, relay_url, now) -> Result<PublicKey, AuthError>` — validates kind 22242, checks challenge tag matches, checks relay tag if relay_url is Some, checks created_at within ±60s, verifies signature, returns pubkey on success
- [x] 2.4 Add `AuthError` enum with variants: `WrongKind`, `InvalidChallenge`, `InvalidRelayUrl`, `TimestampOutOfRange`, `InvalidSignature(String)`
- [x] 2.5 Unit tests for `verify_auth_event`: valid event passes, wrong kind rejected, wrong challenge rejected, wrong relay URL rejected, expired timestamp rejected, invalid signature rejected, relay URL check skipped when None

## 3. Connection handler integration

- [x] 3.1 Update `handle_connection` signature to accept `write_policy: WritePolicy` and `relay_url: Option<String>` parameters
- [x] 3.2 Create `AuthState` at start of `handle_connection`, send `RelayMessage::Auth { challenge }` to client immediately after connection setup
- [x] 3.3 Add `ClientMessage::Auth` arm to `handle_message` — call `auth_state.verify_auth_event()`, on success set `auth_state.authed_pubkey`, send OK true; on failure send OK false with error message
- [x] 3.4 Add write policy check in `handle_event`: if `AuthRequired` and not authed, respond with `OK false "auth-required: please authenticate"`; if `ReadOnly`, respond with `OK false "blocked: relay is read-only"`
- [x] 3.5 Thread `write_policy` and `relay_url` from `NostrRelayService` through to the connection spawn in `relay.rs`

## 4. NIP-11 relay info update

- [x] 4.1 Add `42` to `supported_nips` array in `relay_info_json()`
- [x] 4.2 When `write_policy` is `AuthRequired`, add `"limitation": { "auth_required": true }` to the NIP-11 JSON document
- [x] 4.3 When `write_policy` is `ReadOnly`, add `"limitation": { "auth_required": true, "read_only": true }` to the NIP-11 JSON document

## 5. Integration tests

- [x] 5.1 Test: connect to relay with `Open` policy, send EVENT without auth, verify accepted
- [x] 5.2 Test: connect to relay with `AuthRequired` policy, send EVENT without auth, verify rejected with `auth-required` message
- [x] 5.3 Test: connect to relay with `AuthRequired` policy, complete NIP-42 auth, then send EVENT, verify accepted
- [x] 5.4 Test: connect to relay with `ReadOnly` policy, send EVENT after auth, verify rejected with `blocked: relay is read-only`
- [x] 5.5 Test: connect to relay with `ReadOnly` policy, send REQ, verify subscription works normally
- [x] 5.6 Test: verify `publish()` API works regardless of write policy (bridge bypass)
- [x] 5.7 Test: verify NIP-11 document includes 42 in supported_nips and shows limitation when auth_required
