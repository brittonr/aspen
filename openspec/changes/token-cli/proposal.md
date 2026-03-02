## Why

Aspen's `aspen-auth` crate has a complete capability token system (creation, verification, delegation, revocation) wired into the RPC pipeline, but there's no operational tool to create, inspect, or delegate tokens. The only way to generate a token today is programmatically via `generate_root_token()`. This blocks enabling `require_auth=true` on real clusters — you can't flip the switch if there's no way to mint the tokens that authorize access. For the self-hosted goal (Forge, CI), operators need to generate root tokens at bootstrap, mint scoped tokens for CI agents and forge users, and inspect tokens for debugging.

## What Changes

- Add a `token` subcommand to `aspen-cli` for token lifecycle management (generate, inspect, delegate)
- Token generation: create root tokens from a node's secret key or a provided key
- Token inspection: decode and display a token's issuer, capabilities, expiry, audience, delegation depth
- Token delegation: mint child tokens with reduced scope from a parent token
- All tokens are base64-encoded `CapabilityToken` structs, matching the existing `--token` flag format

## Capabilities

### New Capabilities

- `token-management`: CLI subcommands for generating, inspecting, and delegating capability tokens

### Modified Capabilities

## Impact

- `aspen-cli`: New `token` subcommand with `generate`, `inspect`, `delegate` commands
- `aspen-auth`: May need minor additions to support inspection (display formatting) if not already present
- No changes to the auth verification pipeline, RPC handlers, or token wire format
- No new crates — this lives in the existing CLI
