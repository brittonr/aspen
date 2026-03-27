## Why

Push and pull are unidirectional — you either send or receive. To keep two clusters in sync, an operator has to run `push` on one side and `pull` on the other, track which direction is stale, and handle the case where both sides have new commits. A single `federation sync` command should compare ref heads, pull what's newer on the remote, and push what's newer locally, in one connection.

## What Changes

- **`aspen-cli federation sync --peer <id> --repo <hex> [--addr <hint>]`**: Bidirectional sync for a specific repo. Connects once, compares local and remote ref heads, pulls missing remote objects, pushes missing local objects. Reports counts for both directions.
- **New RPC `FederationBidiSync`**: Single request carrying peer, repo, and optional direction preference. Returns combined pull+push stats.
- **New handler `handle_federation_bidi_sync`**: Orchestrates the flow — connect, handshake, get remote state, get local state, compute diff, pull delta, push delta.
- **Ref comparison logic**: Pure function that takes local heads and remote heads, produces `to_pull` (remote-only or remote-ahead refs) and `to_push` (local-only or local-ahead refs). For refs that exist on both sides with different hashes, the remote wins by default (pull-biased, matching git fetch semantics). A `--push-wins` flag inverts this.
- **Integration test**: Two clusters with divergent repos — A has commits B lacks, B has commits A lacks. Sync from A produces correct bidirectional transfer.

## Capabilities

### New Capabilities

- `federation-bidi-sync`: Bidirectional federation sync that compares ref heads and transfers objects in both directions over a single connection.

### Modified Capabilities

## Impact

- `crates/aspen-cli/`: New `Sync` subcommand args (currently `sync` does discovery-only; extend with `--repo`)
- `crates/aspen-client-api/`: New `FederationBidiSync` request/response types
- `crates/aspen-forge-handler/`: New `handle_federation_bidi_sync` orchestrator
- `crates/aspen-federation/src/verified/`: Pure ref-comparison function
- `crates/aspen-federation/tests/`: Wire-level test for bidirectional transfer
