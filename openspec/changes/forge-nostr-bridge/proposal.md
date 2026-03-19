## Why

The Nostr identity system authenticates users and assigns them keys, but the Forge is still invisible to the Nostr ecosystem. Repos, pushes, issues, and patches should be published as NIP-34 events to the embedded relay so any Nostr client can discover and display them. Users should be able to authenticate from the CLI without a browser, and the web UI should show real profile names instead of truncated npub hex.

## What Changes

- Forge→Nostr bridge: hook into forge events (push, issue create, patch create) and publish NIP-34 events (kind 30617 repos, kind 1617 patches, kind 1621 issues) to the embedded relay
- Profile fetching: on user authentication, fetch their kind 0 profile from the relay and cache it. The web UI resolves author keys to display names via cached profiles.
- CLI auth: `aspen-cli auth login` command that does challenge-response with the user's nsec locally, stores the token for subsequent commands

## Capabilities

### New Capabilities

- `nip34-bridge`: Publish NIP-34 events to the embedded Nostr relay when Forge state changes
- `profile-fetch`: Fetch and cache Nostr kind 0 profiles for display name resolution
- `cli-auth`: CLI-side Nostr authentication with local nsec signing

### Modified Capabilities
