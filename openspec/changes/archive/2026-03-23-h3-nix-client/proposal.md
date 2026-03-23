## Why

The nix binary cache gateway serves HTTP/3 over iroh QUIC (`h3-serving` feature) and `aspen-h3-proxy` bridges HTTP/1.1 clients to H3 endpoints, but neither is packaged in the Nix flake and no test proves the end-to-end path works. Nix clients can only reach the gateway over plain TCP today. The H3 path is dead code until it's packaged, wired together, and tested.

## What Changes

- Build `aspen-h3-proxy` binary as a Nix package in `flake.nix`
- Build `aspen-nix-cache-gateway` with `h3-serving` feature enabled (new variant or flag)
- Create a NixOS VM integration test that exercises the full H3 path: gateway (`--h3`) → h3-proxy → nix client fetching narinfo/NAR
- Close the two deferred tasks from `iroh-upgrade-097` (4.4 and 5.7)

## Capabilities

### New Capabilities

- `h3-nix-client-path`: End-to-end HTTP/3 binary cache access — Nix client fetches store paths through `aspen-h3-proxy` forwarding to `aspen-nix-cache-gateway` over iroh QUIC

### Modified Capabilities

- `nix-cache-gateway`: Add H3 transport variant to existing gateway requirements

## Impact

- `flake.nix`: New package entries for `aspen-h3-proxy` and h3-enabled gateway variant
- `nix/tests/`: New VM test file (`nix-cache-h3-test.nix`)
- No Rust code changes expected — the crates already compile. Packaging and integration only.
- Closes iroh-upgrade-097 tasks 4.4 and 5.7
