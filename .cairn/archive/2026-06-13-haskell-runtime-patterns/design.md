## Context

Molten's architecture already requires canonical Preserves boundaries, explicit effect handlers, deterministic replay, SAM-style turn semantics, capability/policy gates, and a first-class testing harness. Haskell offers a mature set of patterns that can make those requirements easier to implement correctly:

- pure functions at the semantic core,
- effect capabilities chosen at the boundary,
- property laws with shrinking and replay seeds,
- transactional updates,
- law-based adapter conformance,
- type distinctions for ids and authority,
- typed state machines,
- golden tests,
- composable parsers,
- structured traversals,
- explicit strictness/resource thinking.

Molten should adapt these ideas into Rust, Preserves, Cairn, Trellis, Hegel, Nickel, Steel, Basalt, and the testing harness. It should not adopt Haskell syntax, runtime behavior, package ecosystem, or compatibility promises.

## Goals

- Make semantic runtime transitions pure, deterministic, and directly testable.
- Route all nondeterminism and side effects through declared capabilities and admitted handlers.
- Encode authority and identity distinctions in Rust types and canonical Preserves schemas.
- Define laws for adapters and profiles, then test those laws with hand-written, property, and replay suites.
- Capture generated and shrunk property failures as deterministic Preserves fixtures.
- Make actor turns atomic transactions with explicit staged actions and rollback.
- Use structured traversal for redaction, diffs, evidence filtering, and report rendering.
- Bound resource growth and deferred work rather than relying on ambient runtime behavior.

## Non-Goals

- Do not claim Haskell language, GHC, Cabal, Stack, Hackage, QuickCheck, Hedgehog, STM, mtl, lens, parser-combinator, or typeclass compatibility.
- Do not require laziness or higher-kinded type encodings that make the Rust implementation opaque.
- Do not let type wrappers replace runtime policy or capability checks.
- Do not let property tests replace formal Trellis predicates or Cairn receipt validation.
- Do not make golden artifacts immutable truth; golden updates still need review and receipts.

## Haskell reference boundary

Haskell is prior art for patterns and vocabulary only. Molten-specific names, schemas, effects, handlers, receipts, and runtime semantics are normative. Documentation should say "Haskell-inspired" or name the specific pattern while explaining the Molten boundary.

Examples:

- Use "pure core / effectful shell" to describe the Molten core/adapters split.
- Use "mtl-style capabilities" as an analogy for effect manifests and handler profiles.
- Use "QuickCheck/Hedgehog-style" to describe property generation, shrinking, and replay seeds implemented through Hegel.
- Use "STM-style" to describe atomic turn staging, not Haskell STM API compatibility.
- Use "lens/optic-inspired" to describe structured Preserves traversal, not a lens library promise.

## Pure core / effectful shell

Core runtime logic should be organized as deterministic transition functions over explicit input data:

```text
state + event + policy/effect decisions + profile identity
    -> staged actions + trace/evidence records + next state hash
```

The core does not read clocks, random, filesystem, environment, process state, network, thread scheduling, databases, Steel VM state, Wasm host state, or remote peers. Those observations arrive as effect responses, policy decisions, or declared inputs represented as canonical Preserves values or content refs.

This makes unit tests, replay, transcripts, and property checks inspect the same semantic transition boundary.

## Capability effect handlers

Molten effect manifests and handler profiles can use the discipline of mtl-style capabilities without adopting Haskell abstractions. Actor/job code declares required effects such as dataspace send/observe, clock, random, blob, storage, policy ask, trace, spawn, remote invoke, Wasm hostcall, or Steel trusted callable. A handler profile supplies concrete handlers:

- `pure`: no effects admitted,
- `local`: deterministic in-process fixtures,
- `record`: admitted real adapters plus canonical effect logs,
- `replay`: recorded responses only,
- `chaos`: deterministic fault profiles,
- `profiling`: deterministic execution with cost metadata,
- `production`: real admitted adapters.

Handlers are explicit profile bindings, not ambient globals. Each request carries effect id, execution id, sequence/replay metadata, capabilities, policy refs, and canonical input hash. Each response or denial is traceable and receipted.

## Property laws and shrinking

Hegel property tests should use the law style common in Haskell testing:

- state laws: replay identity, snapshot hash stability, transaction rollback,
- adapter laws: read-after-write, content hash integrity, denial-before-side-effect,
- scheduler laws: total order, idempotent replay, bounded fairness,
- Preserves laws: canonical roundtrip, pattern binding order, redaction stability,
- authority laws: attenuation monotonicity, revocation cleanup, no authority minting,
- resource laws: bounded queues, monotonic budget consumption, no silent drops.

Generated inputs, seeds, shrink paths, and final shrunk counterexamples are stored as Preserves fixtures when a suite persists them across an evidence boundary. A persisted shrunk counterexample must be runnable without the generator and suitable for repro bundle export. Automatic export of every Hegel shrink result is a future harness extension unless a specific suite implements it.

## STM-style transactional turns

Actor turns are transactions over runtime-visible state. During a turn, actor code may stage:

- actor state updates,
- dataspace assertions/retractions,
- messages,
- effect intents,
- trace/evidence records,
- child actor/service lifecycle actions,
- resource consumption records.

The staged turn is validated, policy-admitted, resource-checked, and then committed atomically. If actor code fails, policy denies, a required effect is unavailable, or a deterministic guard fails, staged changes are discarded. Adapter effects either occur after commit admission, deny before side effect, or are split by a future explicit reserve/commit/abort effect-record extension so replay can prove no invisible partial side effect occurred. This archived slice records the fail-closed boundary; it does not claim a general reserve/commit/abort adapter API.

## Adapter laws and conformance

Each adapter must publish a small law set before it becomes evidence-bearing. Examples:

- storage: read-after-write, version/transaction isolation where promised, replayed reads match logs,
- blobs/chunks: content ref integrity, range read consistency, no delivery before verification,
- policy: deny before side effect, deterministic decision identity for deterministic inputs,
- network/Iroh: envelope identity preserved, recorded deliveries replay in canonical order,
- Wasm: no hostcall outside admitted effect manifest, fuel/resource denial before unbounded execution,
- Steel: public runtime API only, reviewed dynamic predicate boundary, no secret mutation,
- scheduler: total canonical key, stable replay, explicit liveness outcomes.

The first-class testing harness runs these laws as conformance suites with canonical traces and receipts.

## Newtype and phantom-authority discipline

Rust types should distinguish concepts that are both semantically and security distinct, even if they share string or hash representations:

- `ActorId`, `SessionId`, `VatId`, `PeerId`, `RunId`, `TurnId`,
- `ArtifactRef`, `SchemaRef`, `PolicyRef`, `ReceiptRef`, `EvidenceRef`, `EffectLogRef`,
- `CapabilityRef`, `SecretRef`, `ContentRef`, `SnapshotRef`, `TraceRef`,
- profile and state markers such as deterministic/replay/record/non-replayable, staged/committed, redacted/revealed.

Type distinctions reduce accidental mixups, but they do not grant authority by themselves. Runtime policy and capability checks remain normative. Canonical Preserves schemas must preserve these distinctions at boundaries.

## Typed protocol and state machines

Molten already plans Trellis-backed choreography and bounded predicates. Haskell session-type/state-machine ideas should be adapted as:

- protocol state represented by typed Rust/Preserves DTOs,
- legal transitions checked by Trellis or pure validators,
- illegal transitions rejected before side effects,
- transition evidence stored in traces and receipts,
- replay diagnostics showing expected/actual protocol states.

The goal is to make invalid states unrepresentable where practical and denied where dynamic data requires runtime checks.

## Golden canonical traces

Golden tests should be over canonical Preserves trace, receipt, snapshot, fixture, and state-hash artifacts. Text snapshots are rendered views only. Golden updates should require receipts identifying old and new refs, authority/reviewer, reason class, and compatibility/migration notes; suites without update receipts must treat that as a future explicit extension rather than silent authority.

## Parser-combinator-style DSLs

Molten can use parser-combinator-style composition for small deterministic languages:

- Preserves pattern subsets,
- transcript stanza parsing,
- policy fixture snippets,
- oracle predicates,
- redaction selectors,
- canonical diff filters.

These parsers must produce deterministic ASTs/DTOs with explicit error spans and canonical Preserves representation where outputs cross runtime or evidence boundaries.

## Optic-inspired traversal

Redaction, canonical diffs, evidence filtering, report rendering, and selective snapshot comparison need structured traversal over Preserves data, not string search. Optic-inspired traversals should:

- select by schema path, label, capability class, or confidentiality mark,
- preserve canonical identity for unchanged subtrees,
- emit redaction evidence rather than silently deleting data,
- support minimal safe diffs for first-divergence reports,
- respect reveal authority and policy.

## Strictness and resource guards

Molten should avoid hidden unbounded work. Deferred actions, lazy artifact fetches, queued messages, traces, property generators, transcript rendering, and report exports must have explicit budgets. The harness and resource governance layer should test for:

- bounded queue and assertion growth,
- bounded trace/report output,
- bounded effect request counts,
- deterministic cancellation/yield points,
- no hidden accumulation of deferred closures or thunks,
- explicit materialization points for content and snapshots.

Wall-clock performance may be advisory; deterministic resource budgets are the gate.

## Integration points

- Runtime spine: pure core, typed refs, effect shell, canonical hashes.
- Effect handlers: capability-style effect manifests and handler profiles.
- SAM runtime: STM-style turn transactions and assertion lifetime laws.
- Testing harness: law/conformance/property/golden/replay suites.
- Hegel: generated laws, shrinking, seeds, replayable counterexamples.
- Trellis: typed protocol/state-machine predicates and transition gates.
- Policy/evidence: Basalt/Nickel/Steel admission plus Cairn receipts.
- Redaction/confidentiality: optic-inspired traversal and reveal policy.
- Resource governance: strictness, budgets, and no hidden work.

## Open Questions

- Which remaining id/ref wrappers should become Rust newtypes instead of schema-only distinctions?
- When should the current fail-closed pre-commit effect boundary grow a general reserve/commit/abort adapter API?
- Should effect handler traits be hand-written first or derived from effect manifests/WIT metadata?
- How much of the Preserves pattern/oracle parser should be implemented with combinators versus generated parsers?
- Which adapter law suites become mandatory before an adapter can be used by dogfood or release gates?
