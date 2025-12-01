## Cluster Smoke Test

The `scripts/aspen-cluster-smoke.sh` helper mirrors the control‑plane flow from
the upstream OpenRaft example. It:

1. Builds `aspen-node`.
2. Spawns five nodes with HTTP control ports (`21001`‑`21005`) and per-node
   `NodeServer` listener ports (`26001`‑`26005`).
3. Exercises `POST /init`, `POST /add-learner`, `POST /change-membership`,
   `POST /write`, and `POST /read` so you can manually verify the transport
   wiring before the real Raft implementation is in place.

Run it from the repository root:

```bash
scripts/aspen-cluster-smoke.sh
```

Logs are written to `n1.log` … `n5.log`, and curl responses are pretty-printed
with `jq` if it is installed. Because the current Raft actor is still a stub,
the HTTP API updates in-memory state only; once the OpenRaft integration lands,
the same script will drive a real replicated cluster.

### Hiqlite backend

Run `scripts/aspen-hiqlite-smoke.sh` to spin up a three-node local Hiqlite
cluster (via `run-hiqlite-cluster.sh`), point the smoke harness at
`--control-backend hiqlite`, and exercise the HTTP workflow against the real
backend. Logs for the temporary Hiqlite nodes are stored at
`target/hiqlite-smoke.log`. The base smoke script forwards any extra CLI
arguments to each `aspen-node`, so other backends can be tested by passing
flags directly to `scripts/aspen-cluster-smoke.sh`.

## HTTP Contract

`aspen-node` exposes a small control-plane shim that will eventually forward to
an external Raft/DB implementation. Each request uses `application/json` and
returns a JSON object on success. Failures surface as `{"message":"..."}` along
with an HTTP status code (`400` for invalid input, `404` for missing keys,
`500` for backend failures).

### `GET /health`

Lightweight readiness probe.

```json
{ "status": "ok", "node_id": 1 }
```

### `GET /metrics`

Returns Raft/log counters plus the cluster snapshot maintained by the
controller stub.

```json
{
  "node_id": 1,
  "cluster": "aspen::primary",
  "log_next_index": 1,
  "last_applied": 0,
  "cluster_state": {
    "members": [1, 2, 3],
    "learners": [{"id": 4, "addr": "127.0.0.1:21004"}],
    "nodes": [
      {"id": 1, "addr": "127.0.0.1:21001"},
      {"id": 2, "addr": "127.0.0.1:21002"}
    ]
  }
}
```

### `GET /iroh-metrics`

Exposes the raw OpenMetrics text collected from the optional Iroh transport.
The route returns `404` when `--enable-iroh` is not supplied. Use it to check
`magicsock_*` counters during smoke tests or to feed the values into a local
Prometheus scrape target.

### `POST /init`

Bootstrap the cluster with an explicit voter set. The body must list at least
one node.

```json
{
  "initial_members": [
    {"id": 1, "addr": "127.0.0.1:21001"},
    {"id": 2, "addr": "127.0.0.1:21002"}
  ]
}
```

Response:

```json
{ "cluster": { "members": [1, 2], "learners": [], "nodes": [...] } }
```

### `POST /add-learner`

Registers a non-voting node so it can catch up. The controller rejects duplicate
IDs.

```json
{ "learner": { "id": 4, "addr": "127.0.0.1:21004" } }
```

Response mirrors `/init`.

### `POST /change-membership`

Replaces the voting set with the provided list.

```json
{ "members": [1, 2, 3, 4, 5] }
```

### `POST /write`

Persists a key/value mutation. Commands are tagged with a `type`, allowing the
future backend to extend the enum without changing the HTTP handler.

```json
{ "command": { "type": "set", "key": "foo", "value": "bar" } }
```

Response:

```json
{ "write": { "command": { "type": "set", "key": "foo", "value": "bar" } } }
```

### `POST /read`

Reads the provided key and returns `404` if it does not exist.

```json
{ "key": "foo" }
```

Response:

```json
{ "read": { "key": "foo", "value": "bar" } }
```

## Expected Sequence / Assumptions

The smoke script follows the intended bootstrap flow:

1. Call `/init` on one node after all peers are listening.
2. Register future members as learners via `/add-learner`.
3. Promote them with `/change-membership`.
4. Drive writes and reads through `/write`/`/read`.

Every node must run `aspen-node` with the same `--cookie` value and a unique
`--id`. The HTTP listener defaults to `127.0.0.1:21001` unless overridden.
Cluster namespaces (e.g. `--cluster-namespace aspen::primary`) are surfaced in
`/metrics` so operators can correlate metrics with backend logs.

## Iroh Transport (Experimental)

`aspen-node` can remap the actor cluster traffic onto [`iroh`](https://iroh.computer)
instead of the default TCP sockets. Launch nodes with:

```
aspen-node \
  --enable-iroh \
  --iroh-secret-hex <64-hex-bytes> \
  --iroh-endpoint-file /tmp/node1.iroh.json \
  --iroh-peer <peer-endpoint-id-or-json>
```

Key notes:

- `--iroh-secret-hex` makes the EndpointId deterministic (32-byte ed25519 secret
  encoded as hex). If omitted, a random key is generated per run.
- `--iroh-endpoint-file` dumps the current EndpointId plus any discovered relay/IP
  addresses as JSON so automation can read the value without scraping logs.
- Each `--iroh-peer` may be a plain EndpointId (`<hex>`) or a JSON-serialized
  `EndpointAddr`. Nodes connect to those peers after their endpoint reports
  `online`.

The CLI will dial the configured peers and feed the resulting QUIC streams into
`NodeServer` via the external transport hook, letting ractor actors communicate
over QUIC/relay links without touching the built-in control-plane protocol.
