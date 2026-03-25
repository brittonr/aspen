## 1. NIP-34 bridge hook

- [x] 1.1 Create `crates/aspen-forge/src/nostr_bridge.rs` — `NostrBridgeHook` that takes an `Arc<NostrRelayService>` and implements hook dispatch by converting forge events to NIP-34 Nostr events
- [x] 1.2 Implement `publish_repo_announcement` — builds kind 30617 event with d-tag (repo ID), name tag, description tag, and publishes to relay
- [x] 1.3 Implement `publish_push_event` — builds event with repo ID, ref, and commit hash tags on ForgePushCompleted hook
- [x] 1.4 Wire the bridge into node startup: when both `forge` and `nostr-relay` features are enabled, register the NostrBridgeHook with the hook service
- [x] 1.5 Unit test: create a relay + bridge, simulate a push event, verify the relay received a NIP-34 event with correct tags

## 2. Profile resolution

- [x] 2.1 Add `NostrGetProfile { npub_hex }` → `NostrGetProfileResult { display_name, nip05 }` RPC operation to client-api
- [x] 2.2 Implement the handler in ForgeServiceExecutor — query the relay's KvEventStore for kind 0 events by the npub, parse the JSON content for display_name/name/nip05 fields
- [x] 2.3 Add `AppState::resolve_profile(npub)` in forge-web — calls the RPC, returns display name or fallback
- [x] 2.4 Add `ProfileCache` to AppState — LRU map of npub → display name with 5-minute TTL, checked before RPC call
- [x] 2.5 Update `author_display()` in templates — when npub is present, call resolve_profile to get the display name instead of showing truncated npub
- [x] 2.6 Patchbay test: store a kind 0 event in the relay, create a commit with that npub, verify the web UI renders the display name

## 3. CLI authentication

- [x] 3.1 Add `aspen-cli auth login` subcommand — reads nsec from stdin (with prompt) or `--nsec-file`, derives npub, runs challenge-response, stores token
- [x] 3.2 Add `aspen-cli auth status` subcommand — shows current auth state (logged in npub, token expiry, or "not logged in")
- [x] 3.3 Token storage: write to `~/.config/aspen/token`, read on startup if `--token` flag not provided
- [x] 3.4 Wire token into AspenClient: if a stored token exists, pass it as `AuthToken` to `AspenClient::connect`
- [x] 3.5 Test: login with a test nsec, verify token file is created, verify subsequent command includes the token
