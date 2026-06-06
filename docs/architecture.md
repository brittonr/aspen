# Molten Architecture

Molten is a policy-gated distributed runtime built around a canonical, evidence-bearing envelope spine. The core design rules are simple:

> Effects happen only through admitted adapters; all communication and evidence has a canonical Preserves boundary.

> Deterministic playback is a central law: the same artifacts, dependency closure, initial state, schema refs, policy refs, handler profile, and deterministic seed or recorded effect log must produce the same canonical traces, receipts, outputs, and final state hash.

Molten combines several prior-art lines without adopting them as compatibility targets:

- Synit / the Syndicated Actor Model inspire the dataspace interaction model.
- Spritely Goblins inspires the vat/object-capability execution model.
- Trellis provides verified finite choreography and Raft/consensus primitives.
- Preserves provides the canonical value and schema boundary.

## Layer map

```text
Nickel config / policy manifests
        |
        v
Typed Molten config and artifacts
        |
        v
+-------------------------------------------------------------+
| molten-core                                                  |
| Envelope, ids, content refs, capabilities, evidence refs     |
| Serde DTOs, Preserves conversion, Blake3 hashes              |
| Pure validation only: no IO, network, clocks, or scripting   |
+-------------------------------------------------------------+
        |                         |                         |
        v                         v                         v
Synit/SAM dataspace        Goblins-style vat/object     Trellis layers
assertions/retractions     transactional actormap       choreography + Raft
Observe patterns           near/far refs, promises      protocol + control plane
        |                         |                         |
        +------------+------------+-------------------------+
                     v
+-------------------------------------------------------------+
| policy and evidence                                          |
| Basalt/UCAN, Nickel contracts, reviewed Steel predicates,    |
| Trellis predicates, Cairn receipts, Octet/Valence evidence   |
+-------------------------------------------------------------+
                     |
                     v
+-------------------------------------------------------------+
| adapters                                                     |
| local runtime, Iroh gossip/blobs/docs, Wasmtime/WASI, Steel, |
| Redb store, tracing, CLI                                     |
+-------------------------------------------------------------+
```

## Core envelope spine

The envelope is the stable boundary object for runtime communication. It carries sender identity, routable subject, Preserves body, blob/content references, capabilities, and evidence references.

Required properties:

- Preserves canonical bytes define boundary identity.
- Blake3 hashes are over canonical Preserves bytes, not Rust memory layout or debug formatting.
- Large payloads use Preserves metadata and content references; the bytes may live in Iroh blobs or another store.
- Core validation is deterministic and side-effect free.
- Runtime playback identity includes artifact ids, dependency closure hash, initial state hash, schema refs, policy refs, handler profile id, seed or recorded effect-log hash, and relevant runtime/tool versions.

Communication surfaces that need canonical Preserves representations include:

- actor and dataspace messages,
- dataspace assertions and retractions,
- choreography protocol messages,
- Raft command and transport envelopes,
- Wasmtime hostcall messages,
- Steel/runtime API boundary values,
- policy decisions,
- receipts and evidence references,
- durable records whose identity matters.

## Dataspace layer: Synit/SAM-inspired

The local runtime is based on reactive conversational state rather than only one-shot messages.

Key concepts:

- **Assertions**: facts maintained by an actor/session/facet/live reference.
- **Retractions**: withdrawal of previously asserted facts.
- **Observe patterns**: subscription assertions over Preserves patterns.
- **Automatic cleanup**: assertions are retracted when their owner dies, disconnects, or loses authority.
- **Turn semantics**: one event enters an actor, pending actions accumulate, policy gates run, then actions commit or roll back as a unit.
- **Service dependency assertions**: demand, readiness, failure, restart, and exposed service objects are represented as dataspace facts.

A normal local exchange should look like:

```text
actor receives event
  -> computes pending assertions/messages/effects
  -> pure validation and policy admission
  -> commit turn
  -> dataspace routes assertions/retractions/messages to observers
```

The dataspace is the default path for ordinary actor traffic. Raft is not used for ordinary actor messages.

## Vat/object layer: Goblins-inspired

The public local runtime model remains Syndicate/SAM-style: actors, entities, facets, assertions, retractions, `Observe` patterns, and turns. A vat is an optional internal execution structure that a Rust, Wasm, or Steel actor/service may host when it needs fine-grained object-capability state.

A vat is a local event-loop/object territory with a transactional actormap. It groups objects that are near to each other, meaning they can call each other synchronously during one actor turn. Anything outside that vat is far and must be called asynchronously.

Key concepts:

- **Vat**: local object territory hosted inside an actor/service; not the public actor model itself.
- **Actormap**: transactional mapping from object references to behavior/state.
- **Near reference**: same vat; synchronous call allowed inside a turn.
- **Far reference**: different vat/actor/process/machine/sandbox/session; asynchronous call only.
- **Promise/vow**: result of a far call.
- **Promise pipelining**: bounded queued calls against unresolved future references.
- **Revocable/attenuated proxy**: narrows authority, logs use, rewrites/filters payloads, or cancels access.
- **Rights amplification**: sealer/unsealer or branded-token pattern for private cooperation without ambient identity checks.

Benefits of the vat/actormap structure:

- **Local synchronous programming without distributed blocking**: near objects can use direct call/return ergonomics, while far references remain explicitly asynchronous.
- **Cheap transactional rollback**: object state changes and pending outbound actions commit only if the enclosing turn succeeds.
- **Clear authority boundaries**: references are capabilities; if an object does not hold a reference, it has no ambient path to that authority.
- **Better failure containment**: a failed near-call chain rolls back to the previous actormap instead of leaving half-mutated object state.
- **Efficient high-latency operation**: promise pipelining lets callers queue work against future far references without extra round trips.
- **Revocation and attenuation as ordinary object structure**: proxies can narrow, log, transform, or cancel authority without rewriting the target object.
- **Persistence and upgrade hooks**: object snapshots can preserve the authority graph and apply explicit upgrade recipes.
- **Time-travel debugging**: turn traces plus actormap deltas make it possible to replay and inspect object state at prior turns, subject to debugging authority.

Turn rollback includes object state:

```text
start turn with committed actormap
  near calls update transactional view
  far sends and dataspace actions are pending
  if admitted and successful: commit delta + pending actions
  otherwise: discard delta + pending actions
```

## Choreography layer: Trellis-backed protocol shape

Choreography describes legal multi-party workflows. It does not implement transport and does not replace the dataspace.

Flow:

```text
Molten protocol manifest / DSL
        |
        v
role, label, payload registries
        |
        v
Trellis GlobalChoreo
        |
        v
Trellis projectability gate
        |
        v
Trellis LocalChoreo per role
        |
        v
dataspace-backed endpoint interpreter
```

Use choreography for:

- protocol installation workflows,
- capability negotiation,
- client/worker/auditor flows,
- reviewed dynamic predicate flows,
- artifact publication workflows,
- workflows that call into Raft-backed control-plane operations.

Do not use choreography to define Raft internals. Choreography may orchestrate a consensus operation, but the Raft protocol itself is owned by the consensus layer.

## Consensus layer: Trellis-backed Raft for control plane

Raft is scoped narrowly to strongly consistent control-plane state.

Use Raft for:

- installed protocol registry,
- Raft group membership,
- capability/policy version registry,
- durable receipt indexes,
- replay/session sequence ledgers,
- linearizable configuration and admission-policy updates,
- explicit control-plane locks or leases.

Do not use Raft for:

- normal actor messages,
- ordinary choreography step traffic,
- blob transfer,
- gossip fanout,
- local-only dataspace assertions.

Raft commands are canonical Molten command envelopes. The replicated state machine is deterministic and side-effect free; adapters persist logs/snapshots and publish resulting facts only after admission and commit.

## Policy and evidence

Every trust-boundary action is admitted before side effects occur.

| Need | Mechanism |
| --- | --- |
| Static declarative config, schema, resource, ability, adapter policy | Nickel contracts |
| Reviewed dynamic predicates or trusted callables | Steel contracts |
| Capability-bearing request enforcement | Basalt / UCAN |
| Bounded predicates and verified primitives | Trellis |
| Receipt validation | Cairn |
| Function/module/provenance evidence | Octet / Valence |

The current harness implementation makes this boundary explicit for the early Preserves deny-rule policy fixture: before any runtime turn or effect request, it canonicalizes the static policy snapshot, derives deterministic Nickel static source/export refs using `nickel-lang`, validates a Basalt Nickel contract envelope, and emits `<policy-gate-v1 "molten.harness.policy-gate.v1" ...>` evidence with a bound Basalt preflight receipt. Report validation rejects missing, stale, or tampered policy-gate evidence, and unreviewed Steel/dynamic predicate records are not accepted as static policy data. Steel may orchestrate through public runtime APIs, but must not bypass the envelope spine, admission gates, or receipt emission.

Authority evidence is canonical and explicit. `<authority-identity-v1 "molten.authority.identity.v1" ...>` records principal/node/actor/service/session/artifact/execution ids and key/parent/metadata refs, but names and ids alone grant no authority. Trust-boundary requests use `<authority-context-v1 "molten.authority.context.v1" ...>` records with scoped capabilities, delegation/key/policy/evidence refs, attenuation, expiry, and revocation refs. Admission emits `<authority-receipt-v1 "molten.authority.receipt.v1" ...>` receipts for pass/denial/expiry/revocation/cleanup/replay; `<authority-revocation-v1 "molten.authority.revocation.v1" ...>` targets keys, principals, delegations, capabilities, live refs, handler bindings, sessions, artifacts, and authority contexts. Gatekeeper resolution returns scoped `<authority-live-ref-v1 "molten.authority.live-ref.v1" ...>` values and cleanup retracts authority-bound assertions when revocation is effective.

Resource governance is separate from data authority. `<resource-grant-v1 "molten.resources.grant.v1" ...>` and `<resource-consumption-v1 "molten.resources.consumption.v1" ...>` bind operational quotas for turns, fuel, memory, mailboxes, assertions/subscriptions, blobs/storage/network, effect calls, trace bytes, remote fetches, and job slots; a grant limits work but does not grant access to data or capabilities. Deterministic backpressure emits `<resource-receipt-v1 "molten.resources.receipt.v1" ...>` for consume/throttle/deny/cleanup decisions, bounds mailbox overflow without silent drops, enforces turn/assertion/adapter/job budgets, and uses `<resource-scheduler-v1 "molten.resources.scheduler.v1" ...>` for OS-timing-independent fairness.

Scoped chain hashing adds tamper-evident continuity only where an evidence profile asks for it. A `<chain-link-v1 "molten.evidence.chain-link.v1" ...>` commits to a specific `(scope, id, epoch)`, previous link ref, payload ref, context refs, producer ref, Trellis predicate ref, and checks; its ref is the Blake3 hash of the canonical Preserves link bytes. Payload refs are not rewritten by linking. Append, verify, and checkpoint receipts now bind canonical `<chain-predicate-receipt-v1 "molten.evidence.chain-predicate-receipt.v1" ...>` artifacts for bounded Trellis predicates covering genesis/append validity, no-gap segments, no-fork policy, anchor descent, and checkpoint range coverage. Harness pass gate receipts embed a scoped local chain evidence bundle that binds the report ref, suite ref, final state ref, chain verify receipt, range predicate, anchor descent, and checkpoint freshness before emitting pass evidence. They also embed actor-scoped turn journals: one local chain per primary actor/session, binding each turn observation to step/input, admission, effect-log, trace, before-state, and after-state refs without introducing a global actor-message head. Chunk-store publication lineage uses the same scoped model: `<chunk-lineage-v1 "molten.chunk-store.lineage.v1" ...>` binds manifest refs, chunk-root refs, chunk-store receipts, Iroh publish/fetch ticket evidence, chain links, verify receipts, and predicate receipts per manifest/root. Iroh chain exchange wraps scoped segments and checkpoint artifacts in `<chain-segment-bundle-v1 "molten.evidence.chain-segment-bundle.v1" ...>` values; fetch/import verifies canonical refs, continuity, predicates, checkpoint bindings, and fork policy locally before any ledger import. Multiple scoped chains can advance independently for evidence ledgers, turn journals, artifact publication, or control-plane checkpoints; this is not a cryptocurrency, fork-choice system, or global total order for actor messages.

## Remote and storage adapters

Iroh is the first remote substrate:

- `iroh-gossip` for envelope-sized messages,
- `iroh-blobs` for large immutable payloads,
- `iroh-docs` for replicated mutable document/state surfaces.

Remote transport does not define semantics. Semantics come from envelope content, dataspace state, choreography endpoint state, consensus state, and policy/evidence gates. Nodes resolve persistent endpoint identity through canonical `<node-identity-v1 "molten.node.identity.v1" ...>` records and `<node-identity-receipt-v1 "molten.node.identity-receipt.v1" ...>` startup receipts that name key source class, endpoint id, backend refs, policies, drift/rotation decisions, and bootstrap/replay refs without exposing private key material; stable identity grants no capability by itself. Peer bootstrap then exchanges canonical `<peer-handshake-v1 "molten.peer.handshake.v1" ...>` records with identity refs, endpoint ids, feature vectors, resource limits, requested joins, and scoped capability offers. Negotiation emits `<peer-agreement-v1 "molten.peer.agreement.v1" ...>` plus `<peer-bootstrap-receipt-v1 "molten.peer.bootstrap-receipt.v1" ...>` evidence, selects the highest mutually admitted features, denies unsafe security downgrades unless policy allows them, and gates gossip/docs/remote-sync/protocol/job joins through explicit admitted capabilities; Iroh transport identity alone is not authority. The first federated pull-sync model keeps sync receiver-driven: peers can publish signed `<federation-announcement-v1 "molten.federation.announcement.v1" ...>` and `<federation-inventory-v1 "molten.federation.inventory.v1" ...>` hints, but receivers compute missing sets, verify local fixture signatures and content hashes, apply delegate/capability checks plus local resource/rate policies, and only then import artifacts with `<federation-receipt-v1 "molten.federation.receipt.v1" ...>` evidence. Chunk manifest announcements pull through the verified content-addressed chunk-store/Iroh path, and sync outcomes can be reflected as local federation status/import/denial dataspace assertions. Federation is not global Raft, a global dataspace, or push-import authority.

Remote SAM dataspace traffic uses canonical `<remote-dataspace-envelope-v1 "molten.remote-dataspace.envelope.v1" ...>` records for message/assert/retract/observe actions and `<remote-dataspace-transport-receipt-v1 "molten.remote-dataspace.transport-receipt.v1" ...>` evidence for publish/deliver/deny decisions. The first implementation is a deterministic `iroh-local-gossip` adapter that stores canonical envelope bytes under an Iroh-shaped local transport root, validates declared blob refs before delivery, and applies delivered envelopes through the local runtime turn boundary. Live `iroh-gossip` publication uses the same envelope and receipt boundary; recorded delivery logs (`<remote-dataspace-delivery-log-v1 ...>`) are required for deterministic replay, and pass evidence can be summarized with `<remote-dataspace-gate-receipt-v1 ...>` binding envelope, transport, bootstrap, authority, resource, and turn-journal refs. Iroh endpoint/topic membership remains transport evidence, not actor authority.

Redb is the first local durable metadata store for:

- receipt indexes,
- replay/admission caches,
- protocol artifacts,
- local metadata,
- content-reference bookkeeping,
- later object/vat snapshot indexes.

## Execution adapters

Wasmtime actors are sandboxed actors. They receive narrow hostcalls such as:

- send envelope,
- subscribe/observe,
- blob get,
- blob put.

WASI is deny-by-default. Filesystem, network, clocks, environment, and sockets require explicit capabilities and policy admission. The local harness currently accepts Wasm hostcall actors only with explicit module/WIT/allowed-hostcall fixtures, `wasmparser` validation, import inspection receipts, no ambient/WASI imports, and no-WASI Wasmtime execution receipts for core modules. Modules that export the `molten.wasm.abi.v1` memory/alloc/dealloc ABI receive canonical Preserves actor-input bytes and return canonical Preserves output bytes with input/output refs in the execution receipt.

Steel is trusted orchestration and experimentation glue, but only through admitted runtime APIs. The local harness now executes reviewed Steel hostcall actors in a Steel VM when explicit source/callable fixtures, source refs, review receipts, forbidden ambient-token checks, and allowed-hostcall contracts validate; ambient APIs remain disabled and each admitted step records a Steel execution receipt.

## Debugging, tracing, and persistence

Trace records are data, not just logs. Deterministic test and playback depend on trace records being canonical, hashable, and tied to state snapshots.

Molten should be able to emit canonical Preserves trace records for:

- actor start/stop,
- turns,
- assertions and retractions,
- messages,
- policy decisions,
- choreography transitions,
- Raft proposals/commits/reads,
- receipts and evidence references,
- object/vat state deltas where authorized,
- deterministic scheduler keys, before/after state hashes, effect requests/responses, and replay divergence boundaries.

The replay law is:

```text
same artifacts
+ same dependency closure
+ same initial state
+ same schema refs
+ same policy refs and capability state
+ same handler profile
+ same deterministic seed or recorded effect log
= same canonical trace records, receipts, outputs, and final state hash
```

Replay should stop at the first divergent boundary: scheduler order, input hash, effect request, effect response, policy decision, committed action, receipt, trace, output, or state hash.

The first-class testing harness makes determinism and replay core invariants: evidence-bearing integration, transcript, property, chaos, dogfood, CI, and admission runs are deterministic by construction, replayed from a recorded effect log, recorded for replay, or explicitly marked non-replayable and excluded from deterministic gates. The harness uses the same Preserves communication rail as the runtime. Harness control commands, actor stimuli, dataspace assertions/retractions, adapter fixtures, effect request/response records, observations, traces, receipts, diagnostics, oracles, and reports cross the harness/runtime boundary as canonical Preserves values or Molten envelopes. Text, JSON, JUnit, TAP, and markdown reports are rendered views over canonical evidence, not the normative oracle. Preflight guards require explicit harness privileges, hermetic deterministic inputs, versioned schemas, fail-closed evidence, visible fixture mutations, production/test separation, secret/capability hygiene, governed golden updates, bounded resources/logical time, canonical scheduler/liveness outcomes, adapter contract gates, and replay eligibility gates. Additional harness rails cover adapter conformance, cross-actor-kind interoperability for native Rust/Steel/Wasm/adapter-backed/remote-proxy actors, system-layer suites for Synit/SAM-like demand-driven services and supervision, reproducibility bundles, counterexample shrinking, negative/security suites, upgrade replay, boundary coverage, deterministic multi-peer simulation, resource regression, golden traces, and flake prevention.

The Goblins-inspired vat layer adds a long-term goal of time-travel debugging:

```text
snapshot actormap
+ record turn inputs/deltas/policy decisions
+ replay to failure point
+ inspect authority graph subject to debugging capability
```

Persistence must preserve authority graphs. Object snapshots should not be able to claim authority that the object did not already hold.

## Coordination modes

| Problem | Use |
| --- | --- |
| Reactive local routing and service discovery | Dataspace assertions / Observe patterns |
| Fine-grained local object behavior | Vat/object layer |
| Async calls across vats/actors/peers | Far refs + promises/vows |
| Legal multi-party workflow shape | Trellis choreography |
| Strongly consistent replicated control-plane state | Trellis Raft |
| Large immutable content | Content refs + Iroh blobs |
| Remote message fanout | Iroh gossip |
| Mutable shared document/state surface | Iroh docs, with envelope evidence |
| Static policy/config/schema gates | Nickel contracts |
| Dynamic reviewed predicates | Steel contracts |
| Capability enforcement | Basalt / UCAN |
| Receipts and audit evidence | Cairn + Octet/Valence |
| Tamper-evident evidence continuity | Scoped chain hashing + Trellis predicates |

## Roadmap Cairn changes

Current architectural changes are recorded under `cairn/changes/`:

- `runtime-spine` — canonical envelope, Preserves boundary, adapters, policy/evidence/storage gates.
- `synit-sam-runtime` — dataspace assertions, Observe patterns, turn semantics, service dependency assertions, tracing.
- `goblins-vat-runtime` — vats, transactional actormap, object refs, promises, revocation, safe serialization, debugging.
- `trellis-choreography` — protocol manifests, Trellis projectability/projection, dataspace endpoint interpreter.
- `trellis-raft-consensus` — Raft-backed strongly consistent control-plane state.
- `trellis-runtime-predicates` — follow-up Trellis predicates for dataspace visibility, turns, patterns, promises, revocation, snapshots, and service dependency admission.
- `unison-artifact-registry` — content-addressed runtime artifacts, names as metadata, dependency graph, semantic docs/transcripts.
- `unison-effect-handlers` — Unison ability-inspired effect/capability manifests with admitted handler profiles for production, local, chaos, and profiling execution.
- `unison-remote-artifact-sync` — Iroh-backed dependency closure sync and remote execution envelopes for admitted artifacts, not arbitrary mobile closures.
- `unison-typed-storage` — schema/type/artifact-bound durable values, typed references, storage handlers, and migration recipe artifacts.
- `unison-upgrade-sessions` — structured, receipt-backed upgrade/refactor sessions for aliases, artifacts, schemas, protocols, policies, and cleanup.
- `unison-schema-identity` — unique vs structural schema identity, compatibility decisions, and schema evidence for storage/protocol/effect boundaries.
- `unison-evaluation-cache` — deterministic validation/projection/test cache keyed by canonical inputs, dependency closures, handler profiles, and policy refs.
- `unison-executable-transcripts` — markdown-like executable docs, examples, and bug repros with canonical trace/receipt expectations.
- `unison-structured-rewrite` — structural find/rewrite over canonical artifacts with preview, validation, receipts, and upgrade-session integration.
- `unison-artifact-catalog-mcp` — catalog/search/rendering and MCP-style introspection tools for artifacts, dependencies, receipts, transcripts, and upgrades.
- `unison-distributed-job-dag` — lazy content-addressed distributed job DAGs with stage artifacts, placement, fusion, memoization, and handler profiles.
- `deterministic-test-playback` — central replay law, deterministic scheduler, logical time/random, record/replay handlers, turn journals, snapshots, and first-divergence diagnostics.
- `first-class-testing-harness` — canonical suite/case/fixture/report artifacts, preflight implementation guards, fresh deterministic local runner, Preserves harness communication rail, canonical oracles, fixture adapters, first-divergence reports, transcript/replay/chaos/property/dogfood integration, adapter conformance, cross-actor-kind interoperability, system-layer suites, repro bundles, security suites, upgrade replay, boundary coverage, deterministic multi-peer simulation, resource regression, golden traces, and flake prevention.
- `admission-evidence-validation` — fail-closed admission evidence validation: mandatory per-step admission decision records, suite-step request binding, policy recomputation, denied-turn rollback checks, denied-effect suppression, policy-decision replay divergence, and gate receipt admission checks.
- `policy-boundary-preflight` — initial explicit policy preflight before side effects: canonical policy snapshots, policy-gate report evidence, static boundary/Basalt context/Steel predicate review checks, and pass-evidence receipt refs.
- `nickel-basalt-policy-preflight` — executable policy preflight evidence: deterministic Nickel source/export normalization, Basalt Nickel contract envelopes, bound Basalt preflight receipts, stale/tampered evidence rejection, and Nickel/Basalt gate receipt refs.
- `capability-context-admission` — capability-bearing admission context: canonical capability fixtures, deny-by-default grants, authority-bound admission decisions, effect authority checks, capability divergence diagnostics, and Basalt/UCAN replacement seam.
- `basalt-ucan-capability-preflight` — executable authority preflight evidence: Basalt authority contract envelopes, bound authority preflight receipts, explicit local UCAN proofset refs, grant-ref binding, stale/tampered evidence rejection, and authority gate receipt refs.
- `mandatory-capability-fixtures` — no implicit authority for evidence-bearing suites: explicit capability fixtures required for execution, validation, gate receipts, examples, and future Basalt/UCAN proof bundles.
- `mandatory-actor-registry-fixtures` — no inferred actors for evidence-bearing suites: explicit actor registries required for execution, validation, gate receipts, examples, and future Steel/Wasm/adapter/remote-proxy executor evidence.
- `mandatory-budget-fixtures` — no default resource policy for evidence-bearing suites: explicit budget fixtures required for execution, validation, gate receipts, examples, and future Nickel/Basalt resource policy refs.
- `nickel-basalt-budget-preflight` — executable resource-policy evidence: deterministic Nickel budget source/export normalization, Basalt resource contract envelopes, bound resource preflight receipts, stale/tampered budget-gate rejection, and budget usage binding checks.
- `sealed-repro-bundles` — portable pass evidence bundles with seal metadata, embedded report gate receipts, report/suite/effect/policy/capability/budget refs, exact receipt recomputation, and diagnostic-only failure bundles.
- `sealed-repro-verify-unpack` — explicit sealed bundle lifecycle: canonical verification receipts, verified unpacking of report/suite/receipts, and fail-closed rejection of failure/unsealed/tampered bundles.
- `sealed-repro-redaction-preflight` — conservative confidentiality rail for portable pass artifacts: redaction policy/gate refs, forbidden sensitive marker scans, and fail-closed rejection of missing/tampered redaction evidence.
- `redacted-repro-export-profiles` — explicit `deny-sensitive`, redacted-diagnostic, and encrypted-private export profiles with transform/reveal receipts and gate-preserving vs diagnostic-only classifications.
- `signed-evidence-receipts` — signed receipt envelopes that bind canonical evidence bytes to signer identity, key purpose, trust roots, revocation status, and parent receipt chains.
- `chain-hashed-evidence-ledger` — scoped canonical hash chains for tamper-evident evidence continuity, turn journals, artifact publication lineages, and Trellis/Raft control-plane checkpoints without introducing a cryptocurrency or global actor-message chain.
- `local-evidence-ledger-store` — local content-addressed evidence ledger for reports, receipts, bundles, indexes, retention pins, import/export receipts, and GC-safe dependencies.
- `executor-hostcall-boundary` — canonical hostcall envelopes, mandatory executor preflight, conformance refs, and negative security tests for native, reviewed Steel, and reviewed Wasm hostcall actors.
- `wasm-preserves-abi` — `molten.wasm.abi.v1` core-module memory ABI for canonical Preserves bytes across Wasmtime actor input, hostcall request/response, and output boundaries.
- `steel-vm-executor` — reviewed Steel VM execution receipts for explicit source/callable fixtures with no ambient authority.
- `adapter-remote-proxy-preflight` — executable adapter and remote-proxy preflight fixtures with manifest/peer/contract/transcript evidence before deterministic gates.
- `iroh-sealed-repro-exchange` — Iroh blob publish/fetch flow for sealed bundles and receipt chains with local verification, ledger import, and confidentiality-preserving reveal boundaries.
- `haskell-runtime-patterns` — Haskell-inspired but non-compatible runtime laws: pure core/effectful shell, capability-style effect handlers, Hegel property laws and shrinking, STM-style transactional turns, adapter law conformance, newtype/phantom-authority discipline, typed protocol state machines, golden canonical traces, parser-combinator-style DSLs, optic-inspired redaction/diffs, and strictness/resource guards.
- `octet-enforcement-gates` — Octet/Valence source/evidence gates for core purity, adapter boundary evidence, authority typing, harness backdoors, production/test separation, secret rendering, resource source shape, fingerprint drift, fail-closed caveats, and review receipt linkage.
- `authority-identity-revocation` — principals, node/actor/service/session ids, UCAN authority contexts, revocation, key rotation, and gatekeeper resolution.
- `resource-governance-backpressure` — resource grants, quotas, deterministic backpressure, Wasmtime/Steel/native budgets, and overload receipts.
- `delivery-idempotency-replay` — operation ids, delivery classes, dedup ledgers, sequence windows, retries, and replay protection.
- `failure-supervision-lifecycle` — lifecycle states, links/monitors, supervisors, restart policy, service assertions, failure rollback, and cleanup.
- `retention-gc-pinning` — retention classes, pin sources, reference indexes, tombstones, redaction hooks, and safe GC eligibility.
- `secrets-redaction-confidentiality` — secret refs, confidential field labels, redaction markers, encrypted refs, reveal receipts, and safe replay/catalog output.
- `supply-chain-provenance-builds` — provenance records, trust states, reproducible build evidence, artifact install gates, and review/attestation receipts.
- `peer-bootstrap-negotiation` — peer bootstrap inputs, handshake records, feature negotiation, capability offers, resource limits, and join admission.
- `content-addressed-chunk-store` — deterministic chunk manifests/Merkle roots, chunk-level dedup, streaming verification, resumable fetch, range reads, and chunk-aware retention/GC.
- `persistent-node-identity` — stable Iroh/node identity across restarts, drift detection, key-source receipts, and bootstrap/replay integration.
- `operator-receipts-dogfood` — operator confidence rail with local dogfood workflow, durable receipts, receipt CLI, and replay/transcript evidence.
- `plugin-host-abi` — versioned host ABI discipline for sandboxed artifacts, lifecycle callbacks, Preserves result encoding, and hostcall/effect mapping.
- `federated-pull-sync` — Aspen-inspired sovereign pull-sync for artifacts, chunks, docs, catalogs, receipts, and app resources with signed verification and local admission.
- `blob-ref-job-submission` — job envelopes by artifact/blob/chunk refs, verified fetch, content-addressed caching, worker status assertions, and result refs.
- `coordination-primitives` — strongly consistent control-plane services for locks, fencing tokens, queues, semaphores, rate limits, elections, barriers, and service registry, exposed via dataspace assertions.
- `remote-dataspace-harness-cli` — operator CLI for canonical remote dataspace envelopes, local publish/deliver, two-peer replayable harness runs, and remote gate receipts.
- `molten-node-runtime-daemon` — durable node process with explicit config/state roots, adapter lifecycle, local Preserves control surface, health, shutdown, and startup receipts.
- `node-control-socket-runtime` — file-backed local Preserves control inbox/outbox, queue receipts, and active startup-bound control locks.
- `node-control-operation-dispatch` — side-effecting node control operations for artifact install, node-local job run, and strict source-gate validation with ledger-resolved payloads and fail-closed preflight.
- `node-control-daemon-loop` — bounded local node control loop with deterministic inbox drain order, heartbeat/loop receipts, idempotent duplicate dispatch, and shutdown stop semantics.
- `node-control-provenance-gates` — canonical provenance records/receipts and node-control install/run preflights that require admitted reviewed/reproducible/policy-trusted provenance before side effects.
- `node-control-iroh-ingress` — deterministic local-Iroh ingress envelopes and receipts that validate peer bootstrap, authority, policy, resource, and scoped delivery idempotency before enqueueing into the durable control inbox.
- `node-control-supervised-runner` — bounded `molten node serve` supervisor around local-Iroh ingress delivery and the durable control loop, with service locks, heartbeat receipts, duplicate-runner denial, and shutdown stop receipts.
- `node-control-live-iroh-transport` — real `iroh-gossip` transport boundary for canonical node-control ingress bytes, live transport receipts, and loopback coverage that feeds the same durable ingress path without granting authority.
- `node-control-live-serve-listener` — bounded `serve --live-iroh` listener mode that records listener/session/neighbor receipts, accepts live gossip events through the live receive boundary, and drains through the supervised control loop.
- `node-control-authority-delegation` — canonical node-control authority grants and receipts that make live ingress resolve peer/node/operation/scope/epoch/revocation delegation evidence before enqueue while keeping Iroh transport, bootstrap, policy/resource, and provenance gates separate.
- `node-control-live-peer-tickets` — canonical live endpoint tickets and peer admission receipts that bind live ingress peer bootstrap refs to node/topic/endpoint evidence before enqueue, while preserving separate authority delegation and provenance gates.
- `node-control-supervisor-policy` — canonical supervisor policies and receipts for bounded restart admission/denial, stale service-lock recovery, duplicate-runner denial, heartbeat/restart/shutdown drain bounds, and fail-closed service locking.
- `node-control-live-send-ux` — external `control-ingress-live-send` workflow that uses bound live tickets to join real Iroh gossip topics, publish canonical ingress bytes, and record send/transport receipts without treating transport as authority.
- `sam-service-supervision-runtime` — demand-driven SAM services with readiness/failure assertions, logical supervision, restart policy, resource bounds, and cleanup receipts.
- `trellis-protocol-session-runtime` — Trellis-gated protocol manifests, endpoint projection, protocol-message envelopes, session state, and dataspace-backed interpreters.
- `raft-control-plane-registry` — first Raft-backed strongly consistent control-plane registry for protocol/artifact/policy/capability pointers and receipt indexes.
- `job-dag-iroh-worker-execution` — remote-shaped job worker requests/results over remote dataspace/Iroh using target sync, admission, execution receipts, and recorded replay logs.
- `dataspace-delivery-idempotency` — scoped operation ids, dedup windows, retry receipts, and replay protection for remote/local dataspace, services, protocols, and job workers.
- `secrets-redaction-encrypted-refs` — usable confidentiality rail with secret refs, redaction markers, encrypted refs, reveal/decrypt receipts, and commitment-based replay.
- `plugin-host-lifecycle-runtime` — artifact-backed plugin install/permission/lifecycle/hostcall/health/upgrade receipts over existing executor/effect boundaries.
- `coordination-services-control-plane` — concrete Raft-backed coordination services for locks/fencing, queues, semaphores, rate limits, elections, barriers, and service registry assertions.
- `operator-dogfood-node-workflow` — end-to-end local node dogfood workflow with canonical checkpoints, reports, repro bundles, and release gate receipts.

Implementation should proceed as vertical slices. The current first slice has a pure in-process runtime kernel under `src/runtime/` with canonical Preserves runtime values, deterministic dataspace sets for messages/assertions/observers, minimal begin/commit/rollback turn boundaries, and a pure admission gate model. The harness drives that kernel for fresh local native actors, records canonical policy-gate/capability-gate/budget-gate/admission-decision/trace/effect/report evidence, rolls denied turns back before commit, suppresses denied ambient effects, and preserves compatibility with the original string-shaped payload shorthand while accepting arbitrary Preserves values for message bodies and exact-value observe/assert/retract payloads. Evidence-bearing execution now requires explicit `<budget-v1 ...>`, `<actor-registry-v1 ...>`, and `<capabilities-v1 ...>` fixtures: omitted budgets/registries/capabilities remain parseable for migration/diagnostics but cannot execute or satisfy pass-evidence gates. Actor registries bind ids to executor kinds, reject inferred actors, and fail closed for unsupported Steel/Wasm/adapter/remote-proxy kinds rather than falling back to native execution. Admission composes static policy with the explicit capability context: capability fixtures deny by default when no grant matches, and admission decision events carry authority evidence bound to the capability context and matching grant/ref or denial. Report validation now fails closed on default resource policy, inferred actor registries, missing/duplicate/tampered admission evidence, missing/stale/tampered executor hostcall boundary evidence, missing/stale/tampered Nickel/Basalt policy preflight evidence, missing/stale/tampered Basalt/UCAN capability preflight evidence, missing/stale/tampered Nickel/Basalt resource preflight evidence, capability authority mismatches, committed actions after denial, and effect records after denied effects. Native harness actors now emit canonical `<executor-preflight-v1 ...>` evidence plus `<actor-input-v1 ...>`, `<hostcall-request-v1 ...>`, `<hostcall-decision-v1 ...>`, and `<actor-output-v1 ...>` envelopes that bind actor id, step/turn, policy ref, capability ref, budget ref, admission evidence, allowed hostcalls, sandbox refs, and runtime trace refs. Reviewed Steel hostcall actors may now declare `<steel-executor-v1 ...>` fixtures that produce bound `<steel-review-receipt-v1 ...>` source/callable/allowed-hostcall evidence and execute in a reviewed Steel VM with `<steel-execution-receipt-v1 ...>` input/output evidence; Steel sources containing forbidden ambient IO tokens or undeclared hostcall use fail closed before side effects. Reviewed Wasm hostcall actors may declare `<wasm-executor-v1 ...>` fixtures that produce bound `<wasm-inspection-receipt-v1 ...>` module/import/WIT/allowed-hostcall evidence; invalid modules, ambient/WASI imports, missing fixtures, undeclared hostcall use, missing operation exports, or mismatched Wasmtime hostcall execution fail closed before side effects. Admitted core Wasm steps instantiate with no WASI, deterministic fuel/memory limits, canonical `<wasm-execution-receipt-v1 ...>` evidence, and `molten.wasm.abi.v1` Preserves byte input/output refs when exported. Executor preflights also bind conformance suite refs for the allowed hostcall profile, with native/Steel/Wasm cross-kind tests over identical Preserves inputs. Adapter and remote-proxy actors can run only with explicit executable preflight fixtures and deterministic/verified transcript profiles; missing fixtures remain fail-closed. Pass-evidence gate receipts include explicit-budget/no-default-resource-policy/resource-policy preflight/Nickel resource export/Basalt resource receipt/budget usage binding, actor-registry/no-inferred-actors/executor-boundary/executor-conformance/Wasm-execution/executor-hostcall-boundary, policy-preflight, Nickel policy source/export normalization, Basalt policy gate/preflight receipt binding, Steel predicate review, capability context/grants/Basalt authority receipt/UCAN proofset/grant-ref binding, hostcall admission/replay, admission, and deterministic replay checks. Report repro exports are sealed pass artifacts: `refs.preserves` embeds a report gate receipt, seal metadata, and refs for report/suite/effect/policy/capability/budget evidence; bundle gates recompute the report receipt exactly before emitting a new `repro-bundle` receipt. `molten test repro verify` emits canonical verification receipts, and `molten test repro unpack` materializes only verified sealed bundles. Sealed bundle export and verification now include redaction policy/gate evidence and reject sensitive Preserves marker records until explicit redaction/encryption support exists. Failure repro bundles remain diagnostic-only.
