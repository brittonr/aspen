# Aspen JobDag composition profiles

Aspen retains the internal `molten` Rust/CLI names and canonical schema IDs. This
profile extends the existing JobDag parser, Trellis planner, executor and cache;
it does not introduce another scheduler or a universal graph representation.

## Compatibility: `job-dag-v1`

The existing label, schema and `version = "v1"` retain their meaning:

- Nodes declare bounded lists of nonempty input/output labels. Duplicate labels
  remain accepted. The second member of `<port NAME ...>` is legacy metadata;
  it does not establish a checked type. A declaration is not a stage signature.
- Edges require known node endpoints. Port names need not match declarations.
  `from_port` does **not** select a predecessor output. Both port strings affect
  lexical input ordering: stable sort by `(to_port, from_node, from_port)`.
  Each edge then appends the predecessor's **whole** value sequence. This is
  deterministic order, not order independence. Renaming these labels or node IDs
  can change noncommutative consumer results.
- Repeated edges repeat the whole predecessor sequence. Fan-out supplies that
  sequence to each consumer without re-executing its producer. Equal-looking
  nodes are not merged. A diamond runs one shared producer once, but data can
  intentionally reach the join along both arms.
- Optional edge schema references are syntactically checked content references,
  not value-schema admission. `single`/`partitioned` and
  `stream`/`typed-ref`/`content-ref` are checked metadata labels; they do not
  partition or materialize data in `gather_inputs`. Edge `checks` strings do not
  establish admission.
- Roots name nodes, not ports. Requested roots retain order and repetitions;
  each contributes its complete sequence. An empty root selection uses sink IDs
  in lexical order. The default request uses the graph's roots when supplied.
- The existing stage adapters own source/map/filter/reduce/materialize behavior,
  effects and failures. Stage/policy/effect/schema references retain their
  existing owners; none of these labels grants permission.

No v1 graph is silently migrated. Previously unsupported selected-stage configs
remain unsupported under v1 execution. Richer port claims require explicit v2.
Existing bounds remain: 256 nodes/roots, 4,096 edges/references/stage or gathered
values, 64 declared ports per direction, and 256 checks.

## Checked `selected-sequence-v2`

The distinct envelope is `<job-dag-v2 "molten.job-dag.selected-sequence.v2" ...>`.
Node/edge outer encodings are reused; meaning is selected by the graph version.
`selected_node_value` and `selected_dag_value` construct this profile, and
`parse_job_dag_value` is authoritative for every producer.

The closed registry binds kind, operation, exact implementation source,
input/output order, schemas, required/multiplicity rules, routing and sharing.
Each node configuration carries `<selected-stage-v2 OPERATION PARAMETERS
SIGNATURE-REF>`. The signature identifies an admitted built-in implementation,
not a fictitious installed executable artifact. An arbitrary artifact cannot
replace it. Changing registry/routing implementation text changes the signature.
The reused port declaration's metadata member must be `<none>` in v2: schemas
come from the selected registry, not a conflicting producer-written descriptor.

| Operation | Kind | Data input | Data outputs |
| --- | --- | --- | --- |
| `source-u64` | source | none | `out` |
| `identity-u64` | map | required, exactly one edge to `in` | `out` |
| `concat-u64` | map | required, one or more edges to `in` | `out` |
| `partition-even-u64` | map | required, exactly one edge to `in` | `left` (even), `right` (odd) |
| `drop-u64` | filter | required, exactly one edge to `in` | empty `out` |
| `sum-u64` | reduce | required, one or more edges to `in` | one `out`; overflow fails |

Data schemas are exact canonical identities for closed u64 sequences. Source
parameters are a u64 sequence; other operations require empty parameters.
Functions, fractions, negative numbers, arbitrary records, content-resource
handles and live capabilities are not graph data in this profile. Copy/drop
eligibility comes from this closed registry, never an author-written label.

Every node additionally declares optional-many `after` and Unit output `done`.
A `done -> after` edge uses the distinct exact Unit schema. It participates in
normal topology/readiness but forwards no application values, including when a
source or drop stage produces an empty sequence. No replacement readiness path
or false data value is used to carry sequencing.

Admission rejects unknown/reversed ports, wrong signatures/kinds, missing data
inputs, excessive single-input connections, mismatched schemas, duplicate
connections, cycles and unsupported edge/profile combinations. Only
`single`/`stream` edges are supported. Effect requirements are rejected by this
pure-only profile; they cannot become ambient execution authority. Effectful
profiles and distributed worker admission remain separately owned and are not
qualified by these local built-ins.

Fan-in uses the explicit lexical rule above, selecting **only** the referenced
producer port before concatenating. Ordinary u64 values can fan out; one
instantiated node still executes once. Port-labelled `<job-port-output-v2 NODE
PORT [VALUES...]>` records remain intact in executor state, cached outputs,
receipts and final results. The Unit port is represented by an empty sequence.
Roots still name nodes and publish declared ports in signature order, once per
root; duplicate graph/request roots reject before execution. Select a downstream
consumer when a boundary should publish one chosen output. Roots are not silently
interpreted as arbitrary string-shaped output handles.

Graph identity binds all edges/selected ports, signatures and version. Existing
plan/request/cache dependency identities bind that graph identity. Switching a
selected output or implementation cannot reuse an incompatible cache entry;
reuse remains conservative across graph changes. Cached v2 port inventories,
instance references, Unit values and u64 values are rechecked on retrieval.

Bounds cover 256 instances, 4,096 edges, 128-byte v2 instance identifiers, fixed
registry ports, 4,096 values per stage **across** its ports, 4,096 gathered data
values per consumer, and 4,096 final data values across requested roots. Fan-in
and final aggregation use checked counts before extending sequences. A failed
run may leave earlier bounded stage-cache entries; no successful final output
or effect authority is invented.

## Captured Nickel authoring library

`molten::job_dag::lower_captured_nickel_job(CapturedNickelJobInput)` is the supported
Linux library route. It evaluates source construction, exports closed JSON,
lowers it to canonical Preserves and calls the normal JobDag parser. Callers can
then use `plan_job_dag`, `install_job_dag` and `run_job_dag` normally. Lowering and
inspection do not execute workload stages.

The source is `fun modules => fun parameters => CLOSED_GRAPH`. Modules are an
explicit captured array of unique logical keys and exact source text, made
available through a deterministically ordered record. For example, a selected
`modules.source` function can construct two distinct occurrences with explicit
`first`/`second` IDs; reusing `{instance, port}` handles retains one producer.
Template functions are inert authoring values and finish before export. No
identity is inferred from a display/template name or equal configuration.

The DTO has exactly `profile`, `instances`, `connections`, `outputs`, `origins`.
Instances have `id`, `operation`, `values`. Connections have explicit `from`/`to`
handles, each containing `instance` and `port`. `values` is a u64 array (empty
for non-sources). Instance and connection declaration order is normalized;
output order is preserved. Normalization is not arbitrary alpha-renaming or
isomorphism invariance. There are no serialized Nickel closures at runtime.

Captured parameters support null, booleans, strings, lossless u64 numbers,
arrays and records, to depth 32. Strings containing interpolation syntax are
rejected. The graph DTO narrows runtime data to u64 arrays and bounded IDs.
JSON numbers not exactly supported as u64, unresolved/missing fields, functions,
unknown fields and live-handle encodings reject. All graph checks are native;
an unused Nickel `checks` field cannot establish admission.

The conservative captured-source profile rejects any `import` or `%{` substring,
including comments and strings. This is intentionally stricter than a full
Nickel frontend, not a rewriting of raw imports. Authors select captured modules
instead. It leaves the existing Choregraph restricted-source profile unchanged.

The caller explicitly selects absolute Nickel and Linux `prlimit` executables,
their expected measured content refs, and a workspace. Measurements must match
before evaluation and remain unchanged afterward. This is integrity/currentness,
not independent source trust or executable authorization. Production hosts must
provision those independently. The exact supported cohort is CLI `nickel 1.17.0
(rev 1320a98)`, selected by the repository's pinned Nix input. Existing Bounded
Exec supplies empty inherited environment, bounded stdin/stdout, a 3-second
wall deadline and process-group teardown; `prlimit` enforces a 512-MiB address
space and 2-second CPU limit. Source/module closure, assembled input and export
are each at most 1 MiB; module count is 64. Expansion additionally obeys native
graph budgets. Resource failure never admits a partial graph. These mechanisms
are not a native-process filesystem/network sandbox.

Provenance follows the existing canonical Preserves source-string/export-string
hash convention and links exact assembled source/modules/parameters, observed
export, evaluator/limiter measurements, lowerer code and post-lowering graph.
Changing source, captured modules, parameters, outputs, implementations, profile,
evaluator or lowerer affects the corresponding bindings. Source/module ordering
is claimed only for this explicit keyed captured-module record, not arbitrary
Nickel import order. Output identity is not permission.

Origins contain bounded `instance`, `template`, `module`, and optional declared
`port` mappings (at most 256 entries). Module/instance/port references are checked;
they remain author-declared attribution, not inferred source spans or proof.
The logical source name is retained separately. Missing source locations and
missing origin entries are reported as unavailable, not synthesized. Evaluator
text is discarded instead of publishing source/secret-bearing diagnostics.

## Shared construction dependency

[Choregraph #6](https://github.com/OnixResearch/choregraph/issues/6) remains open.
The inspected available `choregraph-nickel` library exposes flat evaluated target
projections and import/identity validation, not the requested reusable typed
fragment/template/instance library. No competing general-purpose composition
library or shared composition-law claim is added here. Native Nickel functions
and the explicit Aspen DTO are usable now; qualification of the shared library,
its nested hygienic instances and cross-target laws remains an upstream
prerequisite. Aspen stage semantics are not Choregraph machine or numeric graph
semantics.

## Reproducible qualification commands

The following commands are prescribed, **not recorded worker execution**. Run
from the work root in the pinned Nix environment. The parent integration must
provide `pkgs.util-linux` on Linux for `prlimit`.

```sh
nix develop --command cargo test --lib selected_sequence_tests
nix develop --command bash -c '
  export MOLTEN_GRAPH_NICKEL="$(command -v nickel)"
  export MOLTEN_GRAPH_PRLIMIT="$(command -v prlimit)"
  cargo test --lib captured_nickel_live -- --ignored --nocapture
'
```

The normal suite includes actual local execution/cache selection, distinct
outputs, diamond sharing, deterministic fan-in, Unit sequencing, malformed
wiring/signatures/schemas/multiplicity, bounded growth, unchanged v1 behavior,
canonical fixture matching, and an independent integer-map reference interpreter
including overflow failure. The explicit live suite evaluates sequence,
diamond, two parameterized module instances, inline/module equivalence and Unit
examples; it rejects raw imports, fractional values, missing parameters and
nonterminating construction. Its executable paths are required, not silently
skipped when missing.

`config/job-graphs/source.preserves.in` is independently authored Preserves with
one explicitly named implementation-signature slot. Tests instantiate that slot
from the selected registry before canonical parsing; it is not presented as an
already admitted artifact. `diamond.json` is independently authored closed DTO
data, not a generated export. Retain actual command results and canonical output
artifacts only after parent qualification. No build, test, formatter, Octet or
Cairn gate was run by this worker, and shared construction acceptance is not
marked complete by the presence of these fixtures.
