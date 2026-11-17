# Batch artifact/state profile v2 and RPC v3 contracts

## Identity domains

An `Id` is a fixed 32-byte value. Its JSON spelling is exactly 64 lowercase hex
characters. Node ids are Iroh public keys. Semantic object ids are BLAKE3 digests
of canonical Preserves values in the following domains:

- `operator-policy.v2`: explicit component and separate native manifest/program grants.
- `job-spec.v2`: the complete validated task graph, typed workload, resources and retry policy.
- `run-id.v1`: tuple `[cluster, submission_key]`; the key is distinct from spec identity.
- `assignment-id.v1`: tuple `[cluster, generation, run, task, node, attempt_epoch, spec_ref]`.
- `execution-request.v2`: complete fenced request, policy, typed workload, ordered inputs and limits.
- `execution-result.v2`: exact request reference, stopped disposition, outcome and optional failure report.
- `content-manifest.v2`: component/data/pure-net kind, raw-byte BLAKE3 digest and exact length.
- `controller-state.v2`, `worker-state.v2`: complete versioned snapshots.
- `batch-rpc.request.v3`, `batch-rpc.reply.v3`: request/response adapter envelopes.
- `pure-net-eval/{manifest,program,ordered-i64-inputs,receipt}/v1`: explicit native
  manifest, source plus reconstructed checked IR, ordered inputs and semantic report.

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

RPC v3 preserves controller-side selected-run status filtering and adds the
artifact-neutral workload carrier. Selected status returns only that run;
complete-state status remains explicit for inspection and planning. All clients,
controllers and workers must use ALPN `molten-orchestrator/batch-rpc/v3` together.
There is no v1/v2 carrier fallback, native selection through a component grant,
or mixed-version compatibility. Content, policy, job, request/result and durable
snapshot identities advance to v2; old v1 snapshots fail closed instead of
opening a second empty ledger. This release provides no in-place database
migration. Progress event/cursor and run/assignment identity domains are unchanged.

`Workload::PureNet` binds both an explicitly installed manifest and a checked
program object. One ordered-input frame supplies the pure I64 arguments. The
manifest independently binds the exact executable/compiled dependency closure,
producer and consumer overlay, rules/arithmetic/readback, backend and finite
limits. A completed semantic receipt is not a controller-accepted output:
current authority, generation/cancellation and durable copy/terminal publication
remain separate prerequisites. A stopped failed attempt may retain a bounded
report without turning that report into successful output.

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
