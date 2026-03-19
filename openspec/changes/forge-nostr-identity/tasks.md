## 1. Identity mapping store

- [x] 1.1 Create `crates/aspen-forge/src/identity/nostr_mapping.rs` — `NostrIdentityStore` with `get_or_create(npub) -> ed25519 keypair`, `get(npub) -> Option<keypair>`, stored encrypted in KV at `_identity:npub:{hex}`
- [x] 1.2 Implement encryption: derive a 32-byte key from the cluster's iroh SecretKey via `blake3::keyed_hash`, use XChaCha20-Poly1305 (from `chacha20poly1305` crate) to encrypt/decrypt the stored ed25519 secret key bytes
- [x] 1.3 Add `UserContext` struct: `{ npub: String, signing_key: iroh::SecretKey, public_key: iroh::PublicKey }`
- [x] 1.4 Unit tests: create mapping, retrieve existing, verify encryption (raw KV value is not plaintext)

## 2. Challenge-response authentication

- [x] 2.1 Add `NostrAuthService` with `create_challenge(npub) -> challenge_id + random bytes` and `verify_challenge(npub, challenge_id, secp256k1_signature) -> Result<UserContext>`
- [x] 2.2 Store pending challenges in a time-bounded in-memory map (60s TTL, max 1000 entries)
- [x] 2.3 On successful verification, issue a capability token carrying the npub via `TokenBuilder` (add npub claim to token)
- [x] 2.4 Add RPC operations: `NostrAuthChallenge { npub }` → `NostrAuthChallengeResponse { challenge_id, challenge }` and `NostrAuthVerify { npub, challenge_id, signature }` → `NostrAuthVerifyResponse { token }`
- [x] 2.5 Unit tests: full challenge-response cycle, expired challenge rejection, invalid signature rejection

## 3. Author struct npub field

- [x] 3.1 Add `pub npub: Option<String>` field to the `Author` struct in `crates/aspen-forge/src/identity/mod.rs`
- [x] 3.2 Update `Author::new()` and `Author::with_timezone()` to accept optional npub
- [x] 3.3 Update `CommitObject::new()` call sites to pass npub through from UserContext
- [x] 3.4 Ensure postcard serialization is backward-compatible (Option<String> defaults to None for old data)

## 4. Per-user signing in ForgeNode

- [x] 4.1 Add `sign_as(&self, ctx: Option<&UserContext>)` helper to ForgeNode that returns the signing key + npub (user's if provided, node default otherwise)
- [x] 4.2 Update `GitBlobStore::commit()` to accept optional UserContext — passes through to Author and SignedObject signing
- [x] 4.3 Update `CobStore` operations (create_issue, add_comment, close_issue, create_patch, etc.) to accept optional UserContext
- [ ] 4.4 Update ForgeServiceExecutor RPC handlers to extract UserContext from the request's auth token and pass it through
- [ ] 4.5 Integration test: two different npubs create commits on the same repo, verify distinct ed25519 author keys and npub fields

## 5. Profile resolution in web UI

- [ ] 5.1 Add `ProfileCache` to AppState — in-memory LRU cache mapping ed25519 hex → `ResolvedProfile { display_name, npub, nip05 }`
- [ ] 5.2 Add `AppState::resolve_author()` method: ed25519 key → KV lookup for npub mapping → relay query for kind 0 event → extract display_name/nip05 → cache
- [ ] 5.3 Add RPC operations for relay query: `NostrQueryProfile { npub }` → returns kind 0 event content (or wire through existing Nostr relay WebSocket internally)
- [ ] 5.4 Update all author-rendering templates (commit log, commit detail, issue detail, patch detail) to call resolve_author and display the profile name
- [ ] 5.5 Fallback chain: profile name → npub bech32 → ed25519 hex truncated

## 6. Web UI login

- [ ] 6.1 Add `/login` page with "Login with Nostr" button — JavaScript calls NIP-07 `window.nostr.signEvent()` to sign the challenge
- [ ] 6.2 Add `/login/manual` fallback page with nsec text input and security warning
- [ ] 6.3 Add login RPC flow: fetch challenge from cluster, submit signed challenge, receive token, store as HTTP cookie
- [ ] 6.4 Add session middleware to forge-web: extract token from cookie, resolve npub, pass to route handlers
- [ ] 6.5 Show logged-in user's profile name in the nav bar, "Login" link when unauthenticated
- [ ] 6.6 Pass UserContext through to issue/patch creation POST handlers so web-created content is attributed to the logged-in user

## 7. Testing

- [ ] 7.1 Patchbay e2e test: authenticate with a test npub, create an issue, verify the issue author carries the npub
- [ ] 7.2 Test profile resolution: store a kind 0 event in the relay, verify the web UI renders the display name
- [ ] 7.3 Test backward compatibility: commits created without auth still work, Author.npub is None
