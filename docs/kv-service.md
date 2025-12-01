# KV Service Harness

Aspen's `KvServiceBuilder` converts configuration inputs into a running `KvNode`
and hands back a `KvService` wrapper that exposes the node metadata plus a ready
to use `KvClient`. This harness powers the `examples/kv_service.rs` binary and
serves as the recommended entry point for operators or integration tests that
need to spin up KV nodes quickly.

## Environment Variables

The example binary reads three variables to boot a node:

| Variable | Description |
| --- | --- |
| `ASPEN_KV_NODE_ID` | Unsigned integer Raft node ID. Every node in the cluster must have a unique ID. |
| `ASPEN_KV_DATA_DIR` | Filesystem directory for the local Raft log and state machine. The process will create `kv.redb` inside this path if it doesn't exist. |
| `ASPEN_KV_PEERS` | Optional comma-separated list of peer specifications in the form `id=endpoint`. `endpoint` uses `iroh::EndpointAddr` syntax (for example `default/ipv4/127.0.0.1/4001/quic`). |

Example invocation:

```bash
ASPEN_KV_NODE_ID=1 \
ASPEN_KV_DATA_DIR=/tmp/aspen-node-1 \
ASPEN_KV_PEERS="2=default/ipv4/127.0.0.1/4002/quic,3=default/ipv4/127.0.0.1/4003/quic" \
cargo run --example kv_service
```

Each peer listed in `ASPEN_KV_PEERS` is registered immediately after the node
starts so Raft RPCs can flow without manual setup. The builder accepts the same
inputs programmatically (see `KvServiceBuilder::with_peer`/`with_peers`) to ease
test orchestration.

## Notes for Scripts and Tests

1. The harness only starts a node; callers are responsible for invoking
   `raft.initialize` with the membership configuration before issuing writes.
2. When scripting multi-node clusters, ensure every node lists the others in
   `ASPEN_KV_PEERS` so bidirectional connections exist.
3. Integration tests should use deterministic data directories (e.g. via
   `tempfile::TempDir`) to avoid cross-test interference.
