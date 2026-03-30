## Why

Git bridge fetch returns all objects in a single `ClientRpcResponse::GitBridgeFetch` message. For the Aspen workspace (33,897 objects, ~490 MB of data), `postcard::to_stdvec` serializes the entire response into memory, then `send.write_all` tries to push it over the QUIC stream. The write fails: `failed to write Client response`. git-remote-aspen never receives the objects.

The push direction already solved this with `GitBridgePushStart/Chunk/Complete` — the client sends objects in 4 MB batches. The fetch direction needs the same treatment but server → client.

This is the final blocker for federated git clone of large repos. The DAG integrity fix (from `fix-federation-git-clone-dag-truncation`) confirmed all 33,897 objects are reachable, but the delivery path chokes.

## What Changes

- Add a chunked fetch protocol: `GitBridgeFetchStart/Chunk/Complete` RPC variants. The server sends object batches (~4 MB each), the client accumulates and writes loose objects incrementally.
- git-remote-aspen switches from a single `GitBridgeFetch` RPC to chunked when the server advertises it (capability probe or batch count > 1).
- The existing `GitBridgeFetch` (single-shot) stays for small repos. The server picks chunked when the object count exceeds a threshold (~2,000 objects).

## Capabilities

### New Capabilities

- `chunked-git-fetch`: Server-to-client chunked object transfer for git bridge fetch, mirroring the existing `GitBridgePushStart/Chunk/Complete` protocol for push.

### Modified Capabilities

- `federated-git-clone`: The federation fetch handler delegates to the chunked fetch path for large repos, resolving the response-too-large failure.

## Impact

- **`crates/aspen-client-api/src/messages/forge.rs`** — New `ClientRpcRequest::GitBridgeFetchStart` and `ClientRpcResponse::GitBridgeFetchChunk/Complete` variants.
- **`crates/aspen-forge-handler/src/handler/handlers/git_bridge.rs`** — `handle_git_bridge_fetch` gains a chunked mode that batches objects and returns them across multiple RPC round-trips.
- **`src/bin/git-remote-aspen/main.rs`** — `handle_fetch_batch` switches to chunked protocol: sends `FetchStart`, loops receiving `FetchChunk`, waits for `FetchComplete`. Writes loose objects incrementally per chunk (no 490 MB in-memory accumulation).
- **`crates/aspen-forge-handler/src/handler/handlers/federation_git.rs`** — `handle_federation_git_fetch` delegates to the chunked fetch path.
- **`crates/aspen-rpc-handlers/src/client.rs`** — May need streaming response support (multiple write_all calls per request) or the chunked protocol uses separate RPC round-trips.
