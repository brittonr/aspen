## Requirements

- REQ-1: `handle_git_bridge_push` must call `announce_ref_update` for each successfully updated ref
- REQ-2: The announcement must include the blake3 commit hash as `new_hash`
- REQ-3: The announcement must include the previous blake3 hash as `old_hash` when the ref existed before (old_sha1 non-empty), or `None` for new refs
- REQ-4: Announcement failures must not cause the push to fail
- REQ-5: The federation dogfood script must set `ASPEN_CI_FEDERATION_CI_ENABLED=true` for bob's cluster
