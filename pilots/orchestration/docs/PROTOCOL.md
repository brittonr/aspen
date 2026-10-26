# Batch profile v1: canonical and wire contracts

## Identity domains

An `Id` is a fixed 32-byte value. Its JSON spelling is exactly 64 lowercase hex
characters. Node ids are Iroh public keys. Semantic object ids are BLAKE3 digests
of canonical Preserves values in the following domains:

- `operator-policy.v1`: the immutable explicit operator policy.
- `job-spec.v1`: the complete validated task graph, including resources and retry policy.
- `run-id.v1`: tuple `[cluster, submission_key]`; the key is distinct from spec identity.
- `assignment-id.v1`: tuple `[cluster, generation, run, task, node, attempt_epoch, spec_ref]`.
- `execution-request.v1`: complete fenced request, policy, component, ordered inputs and limits.
- `execution-result.v1`: exact request reference, stopped disposition and outcome.
- `content-manifest.v1`: kind, raw-byte BLAKE3 digest and exact length.
- `controller-state.v1`, `worker-state.v1`: complete versioned snapshots.
- `batch-rpc.request.v1`, `batch-rpc.reply.v1`: request/response adapter envelopes.

A content reference confers no installation, execution or result-acceptance authority.
The standalone content manifest is not claimed to be byte-compatible with an
existing Molten content-store manifest.

## Preserves projection

The exact root is:

```text
<molten.orchestration.document.v1 "schema-name" PROJECTED_VALUE>
```

Projection of the versioned Serde DTO's logical value is closed:

```text
null             => <o.null>
true / false     => <o.true> / <o.false>
signed/unsigned integer => <o.integer "canonical-base-10-decimal">
string           => <o.string "value">
array            => <o.sequence [PROJECTED_VALUE ...]>
object           => <o.map [<o.field "key" PROJECTED_VALUE> ...]>
```

Object fields are sorted by key. Floats are rejected. The codec uses the selected
Preserves library's packed canonical encoder with annotations disabled, then
BLAKE3. Domain-specific record and Serde field/variant names are versioned schema
content; changing them requires a new domain/version and migration.

The JSON reconstruction projection is stored alongside canonical bytes. It is
not authority. A read deserializes the typed DTO under finite byte/recursion bounds,
regenerates both projections, and requires exact equality. This catches missing,
changed, duplicate, noncanonical and trailing fields without accepting arbitrary
packed input into an unbounded parser. State restoration additionally validates
cross-record invariants and recomputes embedded identities.

## RPC materialization

One authenticated Iroh connection uses one bidirectional stream and exactly one
request/response. Each direction contains:

```text
u64 big-endian byte length
that many UTF-8 bytes of {"schema":...,"value_ref":...,"body":...}
EOF
```

Length is checked before allocation. Request/reply frames are canonical normalized
JSON **adapter materializations** whose `value_ref` is computed from the Preserves
semantic projection, not a hash of JSON. Decode requires exact normalized framing,
matching schema and matching identity. A JSON frame is not presented as a native
Molten Preserves transport envelope.

The endpoint admits only the selected ALPN. The server checks the authenticated
remote key against explicit role enrollment before reading the body. No retry,
relay, name lookup, protocol fallback or authority promotion is performed by the
transport adapter. Errors return bounded codes, never backend paths or secrets.

## Request and attempt idempotency

Submission key + cluster identifies a run. Repeating the exact specification with
that key returns the original run. Rebinding the key to a different specification
is a conflict.

Assignment identity is immutable and never reused. A worker retains exact
acceptance and terminal records. A duplicate start must match the full request,
not merely owner/run/epoch. A cancellation for an unknown assignment leaves a
terminal no-start tombstone before acknowledgement. A future delayed start matches
that terminal record, not an empty slot.

Neither an RPC acknowledgement nor a successful hash check proves an external
effect happened exactly once. This runtime exposes no external guest effects.
