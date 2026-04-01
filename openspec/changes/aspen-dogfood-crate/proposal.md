## Why

The three dogfood scripts (`dogfood-local.sh` at 1,278 lines, `dogfood-local-vmci.sh` at 935 lines, `dogfood-federation.sh` at 895 lines) have grown into substantial orchestration programs that shell out to `aspen-cli` ~50 times, parse its text output with `grep`, and manage process lifecycles with PID files. This is fragile — a CLI output format change silently breaks assertions, error handling is string matching, and there's no shared structure between the three scripts despite ~60% overlap in logic (start cluster, push to forge, wait for CI, verify artifacts).

A proper `aspen-dogfood` binary crate can use `aspen-client` directly over Iroh, share types with the rest of the workspace, and replace the CLI-shelling + grep-parsing layer with typed RPC calls. The three scripts collapse into one binary with subcommands.

## What Changes

- New `crates/aspen-dogfood/` binary crate with subcommands matching current script modes: `start`, `stop`, `status`, `push`, `build`, `deploy`, `verify`, `full-loop`
- Uses `aspen-client` for all cluster communication instead of shelling out to `aspen-cli`
- Typed process management for node lifecycles (start, health-check, graceful shutdown) replacing PID-file + `kill -0` polling
- Federation dogfood mode as a subcommand (`aspen-dogfood federation`) instead of a separate 895-line script
- VM-CI mode as a flag (`--vm-ci`) instead of a separate 935-line script
- Flake.nix `nix run .#dogfood-local` updated to run the new binary
- Old shell scripts moved to `scripts/deprecated/` initially, deleted after one release cycle

## Capabilities

### New Capabilities

- `dogfood-orchestration`: Core orchestration logic — cluster lifecycle, forge push, CI wait, artifact verification — as a typed Rust API callable from the binary or tests
- `dogfood-cli`: Binary with clap subcommands replacing the three shell scripts

### Modified Capabilities

## Impact

- `scripts/dogfood-local.sh`, `scripts/dogfood-local-vmci.sh`, `scripts/dogfood-federation.sh` — replaced
- `flake.nix` — `dogfood-local`, `dogfood-local-vmci` app entries updated to point at new binary
- `crates/aspen-client/` — may need minor additions if any CLI-only operations lack client API equivalents
- NixOS VM tests (`ci-dogfood-test`, `ci-dogfood-self-build`) — update to use new binary
- `AGENTS.md` dogfood section — update commands
