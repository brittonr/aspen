## Context

The embedded Nostr relay (`aspen-nostr-relay`) runs NIP-01 over WebSocket, stores events in Raft KV. The hook system fires `ForgePushCompleted` events on git push. The NostrAuthService issues capability tokens with npub facts. The web UI shows `npub:abc...` for authenticated authors but has no way to resolve display names.

## Goals / Non-Goals

**Goals:**

- Publish NIP-34 events on forge state changes (repo create, push, issue, patch)
- Resolve Nostr profiles (kind 0) to display names in the web UI
- CLI authentication without a browser

**Non-Goals:**

- Fetching profiles from external Nostr relays (local relay only for now)
- Two-way sync (Nostr→Forge event import)
- NIP-34 patch/issue comments (just the top-level objects)

## Decisions

**The bridge is a hook subscriber, not inline code.**

A `NostrBridgeHook` implements the hook dispatch interface. When registered, it receives forge events and publishes corresponding NIP-34 events to the relay. This keeps the bridge decoupled from the forge handler — disabling the Nostr relay doesn't break forge operations.

**Profile resolution queries the local relay's KV store directly.**

The web frontend talks to the cluster via RPC. Adding a `NostrGetProfile { npub }` RPC operation that queries the relay's event store for kind 0 events is simpler than having the web frontend open a separate WebSocket to the relay. Results are cached in AppState with a 5-minute TTL.

**CLI auth stores the token in a file.**

`aspen-cli auth login` reads the nsec from stdin (or `--nsec-file`), signs the challenge locally, sends only the signature over the network, and writes the received token to `~/.config/aspen/token`. Subsequent CLI commands read the token automatically. The nsec is never sent to the server.

## Risks / Trade-offs

**[NIP-34 events signed by cluster key, not user key]** → The bridge publishes events signed with the cluster's Nostr identity, not individual users' keys. This is correct for repo announcements (the cluster hosts the repo) but less ideal for patches/issues (which should ideally be signed by the author). Acceptable for now — author attribution is in the event tags.
