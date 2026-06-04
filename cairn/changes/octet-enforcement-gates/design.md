## Context

Molten already depends on Octet/Valence and has requirements for pure core boundaries, Preserves communication, explicit effect handlers, deterministic replay, first-class harness evidence, Haskell-inspired law discipline, secret redaction, resource governance, and Cairn receipts. Octet can enforce parts of those laws statically and produce bounded evidence artifacts that the harness and Cairn can reference.

The intended flow is:

```text
source code
  -> cargo octet check / fingerprint gates
  -> Valence function objects, caveats, review manifests, SARIF/sidecars
  -> content refs in harness reports
  -> Cairn receipts for CI/release/admission decisions
```

Octet is not the runtime policy engine and Valence is not a correctness proof. Runtime policy still uses Basalt/UCAN, Nickel, reviewed Steel predicates, Trellis predicates, and Cairn receipts. Octet/Valence catch source-shape and boundary violations early and make review evidence reproducible.

## Goals

- Enforce pure-core/effectful-shell boundaries before runtime execution.
- Make critical source surfaces identifiable by marker attributes, manifests, or Octet config.
- Fail closed on ambient-effect caveats for core transitions unless reviewed.
- Require adapter boundary code to expose effect, trace, receipt, resource, and replay obligations.
- Prevent public boundary APIs from collapsing ids, refs, capabilities, secrets, and receipts into interchangeable strings or hashes.
- Prevent harness fixtures and debug helpers from becoming invisible runtime backdoors.
- Keep test-only capabilities out of production profiles unless explicitly admitted and evidenced.
- Tie source fingerprint drift to harness replay, conformance suites, golden updates, migration receipts, or human/policy review.
- Attach Octet/Valence artifacts to harness reports and Cairn receipts as bounded evidence.

## Non-Goals

- Do not treat Octet/Valence evidence as proof of semantic correctness.
- Do not replace runtime policy, capability checks, deterministic replay, property tests, Trellis predicates, or Cairn validation.
- Do not require all source code to be exhaustively represented as Valence function objects in the first milestone.
- Do not rely on source-token caveat absence as proof of purity; it is a fail-closed heuristic/evidence input.
- Do not expose test-only bypasses in production just because Octet approved the source shape.

## Source surface markers

Molten should identify critical source surfaces using marker attributes, module path manifests, or Octet config. Initial marker concepts may include:

```rust
#[molten_core_transition]
fn step(...) -> TransitionResult { ... }

#[molten_adapter_boundary(effect = "storage.write")]
fn write_storage(...) -> EffectResult { ... }

#[molten_test_capability]
fn inject_fixture(...) -> FixtureResult { ... }

#[molten_secret]
struct SecretRef(...);
```

The exact marker syntax can evolve. The important requirement is that Octet can classify surfaces and apply the correct lint/fingerprint/evidence gates. Marker use is evidence metadata, not authority.

## Core purity gate

Functions or modules marked as core transitions must be deterministic and side-effect free. Octet should reject or require review receipts for source caveats such as:

- filesystem, network, process, environment, wall-clock, entropy, database, or scripting access,
- `unsafe` unless specifically reviewed for a pure data operation,
- `panic`, `unwrap`, `expect`, or unstructured aborts in evidence-bearing core paths,
- synchronization or thread scheduling observations that affect semantics,
- direct adapter calls or runtime shell access.

Valence function objects/fingerprints for core transitions should be attached to harness reports and Cairn receipts. Absence of coarse caveats is not a proof of purity; it is a precondition for accepting the function as deterministic evidence without extra review.

## Adapter boundary evidence gate

Adapter boundary functions are allowed to perform effects, but only as explicit shell code. Octet should require adapter surfaces to identify:

- effect kind and effect manifest id,
- handler profile compatibility,
- capability and policy check location,
- trace and receipt emission obligation,
- resource budget/checkpoint behavior,
- replay/record behavior or non-replayable status,
- error category mapping.

A change to adapter boundary fingerprints should require adapter conformance suites and replay/golden checks before release/admission gates accept the new evidence.

## Authority typing gate

Boundary APIs must not accept or return raw `String`, byte arrays, or generic hashes where a typed id/ref/authority object is required. Octet should flag public runtime, policy, storage, harness, and adapter APIs that use stringly values for:

- `ActorId`, `SessionId`, `PeerId`, `RunId`, `TurnId`,
- `ArtifactRef`, `SchemaRef`, `PolicyRef`, `ReceiptRef`, `EvidenceRef`, `EffectLogRef`,
- `CapabilityRef`, `SecretRef`, `ContentRef`, `SnapshotRef`, `TraceRef`,
- deterministic/replay/record/non-replayable profile markers,
- staged/committed/redacted/revealed state markers.

Raw strings may still appear at CLI/config parsing edges, but they must be parsed and validated into typed refs before crossing runtime/evidence boundaries.

## Harness backdoor gate

The first-class testing harness must not use private runtime APIs to mutate stores, inject actor state, skip policy, or invent receipts. Octet should flag direct access to runtime internals from harness modules unless the function is marked as an explicit test capability and the code path emits trace/receipt evidence.

Allowed test-only operations must be:

- visible as capabilities,
- disabled outside admitted test/debug/record/replay profiles,
- represented in canonical Preserves evidence,
- excluded from production profiles unless policy admits them.

## Production/test separation gate

Octet should enforce feature/profile separation for test-only APIs, fixture adapters, bypass capabilities, debug hooks, and exploratory non-replayable profiles. Production builds should fail or require explicit policy/review evidence if those surfaces are reachable.

This gate complements runtime policy; it catches accidental public exports or feature leaks before deployment.

## Secret and capability rendering gate

Types marked as secret or capability-bearing must not leak through unredacted debug, display, serialization, report export, logs, or tracing. Octet should flag derived or manual rendering surfaces that do not route through redaction policy or encrypted refs.

Serialization for canonical storage/evidence may be allowed only when the schema marks confidentiality and export paths apply redaction/reveal policy.

## Resource/source-shape gate

Octet should enforce source-shape rules for bounded execution:

- unbounded loops in actor/core/harness paths need deterministic yield/cancel checkpoints,
- queues and collections with runtime data need bounds or resource accounting,
- trace/report builders need output limits,
- Wasm/Steel/native adapters need fuel or operation-budget hooks,
- recursive/deferred work needs explicit depth/size budgets.

Octet findings become preflight failures for evidence-bearing harness profiles unless reviewed with resource rationale.

## Fingerprint drift gate

Valence function objects and Octet fingerprints identify critical source surfaces. Drift on selected surfaces should require related evidence before acceptance:

| Surface changed | Required follow-up evidence |
| --- | --- |
| Core transition | deterministic replay/property/golden trace report or review receipt |
| Adapter boundary | adapter conformance report and replay/record evidence |
| Harness oracle/report validator | harness self-test and golden report validation |
| Redaction/export path | confidentiality/security suite report |
| Protocol transition gate | Trellis predicate/check report and replay diagnostics |
| Golden update tool | golden update receipt and migration notes |

Fingerprint drift is not inherently bad. It is a trigger to re-run or review the evidence that depends on that source surface.

## Evidence integration

Harness reports and Cairn receipts should include or reference:

- Octet command/config version,
- Octet findings and severity summary,
- Valence function object refs for critical surfaces,
- source caveat summaries,
- review/suppression manifests,
- fingerprint drift summaries,
- links to required harness/conformance/replay/golden reports.

Evidence consumers must display caveats clearly. For example, a Valence function object may identify normalized source and caveats but must not claim behavioral correctness.

## Initial CI shape

An initial CI/release gate can require:

```text
cargo octet check --artifact-dir artifacts/octet
cargo octet fingerprint --check --review-manifest artifacts/fingerprint-review.json <critical paths>
molten test run <core/harness/conformance suites>
cairn validate --strict
```

The exact commands may evolve, but the gate accepts the run only if Octet artifacts, harness canonical reports, and Cairn validation all agree that required evidence exists.

## Open Questions

- Should marker attributes be inert Rust attributes, macro attributes, module-level manifests, or external Octet config first?
- Which core transition functions should be fingerprinted in the first implementation slice?
- How should review manifests link to Cairn receipts: direct receipt refs, content refs, or both?
- Which Octet lints already cover the first gate and which require custom Molten rules?
