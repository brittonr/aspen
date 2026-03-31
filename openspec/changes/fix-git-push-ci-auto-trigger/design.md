## Approach

Add `announce_ref_update` calls to the existing `handle_git_bridge_push` function after each successful ref update. The chunked push path (`handle_git_bridge_push_complete`) delegates to the same function, so both paths are fixed in one place.

For the `old_hash` field, look up the old blake3 hash for `ref_update.old_sha1` via the hash mapping store. If the old SHA-1 is empty (new ref), pass `None`.

## Data Flow

```
git push → git-remote-aspen → GitBridgePush RPC
  → handle_git_bridge_push
    → import objects
    → update refs
    → NEW: announce_ref_update for each successful ref  ←── the fix
      → gossip.notify_local_handler (triggers CiTriggerHandler)
      → gossip.broadcast (notifies remote nodes)
```

## Key Decisions

1. **Announce after each ref, not after all refs**: Matches federation.rs pattern. Each ref update is independent.
2. **Resolve old_sha1 to blake3 for old_hash**: The `GitBridgeRefUpdate` has `old_sha1`. Look it up via `importer.get_blake3()` to provide proper `old_hash` for path-based trigger filters.
3. **Don't block on announce failures**: `announce_ref_update` already handles gossip failures gracefully (logs debug, doesn't error).
4. **federation_ci_enabled for dogfood**: Set `ASPEN_CI_FEDERATION_CI_ENABLED=true` on bob's cluster so mirror repo triggers aren't gated.

## Risks

None significant. `announce_ref_update` is battle-tested in the federation path. The only new behavior is that CI auto-trigger actually fires on git push, which is the intended design.
