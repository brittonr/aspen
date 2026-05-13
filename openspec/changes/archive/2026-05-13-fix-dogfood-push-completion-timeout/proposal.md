## Why

The archived `fix-dogfood-local-push-timeout` change proved dogfood-local now bypasses workstation hooks and reaches the product push-completion boundary, but the focused `push-check` still times out after CI watch registration. The remaining blocker appears to be pushing Aspen's full Git history into an empty local Forge repo during an acceptance run, which spends the bounded push budget before the Forge/CI trigger path can complete.

## What Changes

- Add a dogfood push snapshot workspace that contains the current source tree as a single commit for local self-hosting acceptance.
- Keep the normal Forge/CI path intact: create repo, register CI watch, push through `git-remote-aspen`, and wait for push completion.
- Preserve redacted receipt evidence for push-completion failures and focused `push-check` verification.

## Capabilities

### Modified Capabilities
- `dogfood-local-connectivity`: local dogfood push acceptance must bound history-transfer overhead and prove source push completion through Forge/CI trigger registration.

## Impact

- **Files**: `crates/aspen-dogfood/src/forge.rs`, dogfood local connectivity OpenSpec.
- **APIs**: no public API changes.
- **Testing**: focused `aspen-dogfood` unit tests, formatting, OpenSpec validation, and `push-check` runtime evidence.
