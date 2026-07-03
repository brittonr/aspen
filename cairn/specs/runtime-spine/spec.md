# Runtime Spine Specification

## Purpose

Defines the `runtime-spine` capability.

## Requirements

### Requirement: Synit and SAM are non-normative references
r[molten.runtime_spine.synit_reference_boundary] Molten MUST treat Synit and the Syndicated Actor Model as non-normative design references for reactive dataspace semantics, assertions, retractions, object capabilities, service assertions, and tracing. Molten MUST NOT claim Synit wire protocol, sturdyref, PID1, service-manager, OID, service-schema, or configuration-scripting compatibility.

#### Scenario: Documentation cites Synit without compatibility claim
- GIVEN Molten design material cites Synit or the Syndicated Actor Model
- WHEN the material describes an adopted pattern
- THEN it states the Molten-specific envelope, policy, evidence, transport, storage, and configuration boundaries
- AND it does not claim Synit compatibility.

### Requirement: Actor turns are atomic
r[molten.runtime_spine.turn_semantics] Molten MUST process local actor events in turns where an actor receives one event, accumulates pending actions, applies deterministic validation and policy gates, and commits or discards pending actions as a unit. Pending messages, assertions, observations, retractions, and effect intents MUST remain invisible until commit.

#### Scenario: Successful turn commits pending actions
- GIVEN an actor turn that stages an assertion and a message
- WHEN all admission and transition checks pass
- THEN the runtime makes the assertion and message visible in the committed turn result.

#### Scenario: Failed turn rolls back pending actions
- GIVEN an actor turn stages pending actions and is denied or fails before commit
- WHEN the runtime rolls the turn back
- THEN no pending assertions, retractions, messages, or effect intents become committed runtime state.

### Requirement: Assertion lifetimes are owner-scoped
r[molten.runtime_spine.assertion_lifetimes] Molten dataspace assertions MUST be owned by an actor, session, facet, or admitted live reference. Owner cleanup, termination, revocation, or session close MUST retract owner-scoped assertions and observers before they remain visible as live state. Duplicate canonical assertions MAY be represented by owner sets or deterministic assertion refs, but visibility MUST depend on at least one live owner.

#### Scenario: Actor cleanup retracts owned assertions
- GIVEN an actor has asserted service presence into a dataspace
- WHEN the actor scope is cleaned up
- THEN the runtime retracts the actor's live assertions and observers.

#### Scenario: Duplicate assertion remains until last owner retracts
- GIVEN two owners assert the same canonical Preserves value into the same dataspace
- WHEN one owner retracts or terminates
- THEN the assertion remains visible while another live owner still maintains it.

### Requirement: Observe subscriptions are explicit assertions
r[molten.runtime_spine.observe_patterns] Molten MUST support explicit `Observe`-style subscription records or DTOs over the implemented Preserves pattern subset. An observer MUST receive matching current assertions, future matching assertions, and matching retractions until the subscription is retracted or the observer scope is cleaned up.

#### Scenario: Observe receives existing and future assertions
- GIVEN a dataspace contains an assertion matching an observer's pattern
- WHEN an actor registers an `Observe` subscription for that pattern
- THEN the observer receives the existing assertion and later future matching assertions.

#### Scenario: Observe retraction stops delivery
- GIVEN an active observer has a live subscription
- WHEN the observer scope or subscription is retracted
- THEN the dataspace stops delivering future matches scoped to that subscription.

### Requirement: Preserves pattern matching is deterministic and bounded
r[molten.runtime_spine.preserves_patterns] Molten MUST define a bounded deterministic Preserves pattern subset for dataspace routing and policy-visible matching. The completed initial subset includes exact canonical value matching and wildcard binding with deterministic binding order; richer record, array, dictionary, conjunction, negation, or extensible compound matching MAY be added only by future admitted extensions.

#### Scenario: Pattern match produces stable bindings
- GIVEN the same implemented Preserves pattern and candidate value on two nodes
- WHEN each node evaluates the match
- THEN both nodes produce the same success or failure result and the same ordered binding sequence.

#### Scenario: Unsupported compound pattern denies
- GIVEN a pattern outside the admitted bounded subset
- WHEN routing or policy-visible matching evaluates it
- THEN the match is denied or rejected before it controls side effects.

### Requirement: Capabilities attenuate dataspace and message authority
r[molten.runtime_spine.capability_attenuation] Molten capabilities MUST attenuate authority over messages, assertions, subscriptions, and reference introduction through Molten policy/authority gates before delivery or publication. The completed scope supports scoped allow/deny authority contexts and live refs; rewrite/filter transforms require explicit future rule evidence before they can alter delivered values.

#### Scenario: Attenuation denies disallowed assertion
- GIVEN a live dataspace reference is scoped to an admitted capability
- WHEN an actor attempts to publish or observe outside that scope
- THEN the runtime denies before the assertion or subscription becomes visible.

#### Scenario: Rewrite requires explicit rule evidence
- GIVEN an actor requests message or assertion rewriting through attenuation
- WHEN no admitted rewrite rule evidence is present
- THEN Molten does not infer a rewrite from the capability alone.

### Requirement: Gatekeeper resolution emits live refs
r[molten.runtime_spine.gatekeeper_resolver] Molten MUST provide a gatekeeper resolver pattern that converts admitted long-lived credentials, UCANs, tickets, invites, or authority contexts into live scoped references with attenuation, expiry or revocation conditions, and evidence refs.

#### Scenario: Credential resolves to live scoped reference
- GIVEN a valid authority context that grants scoped access to a resource
- WHEN an actor submits it to the gatekeeper resolver
- THEN the resolver returns a live reference scoped to the admitted capability, attenuation, expiry, and receipt evidence.

#### Scenario: Revoked credential denies resolution
- GIVEN a revocation applies to a credential, context, delegation, key, or capability
- WHEN gatekeeper resolution runs
- THEN the resolver denies or invalidates the live reference and records diagnostic evidence.

### Requirement: Live references have cleanup semantics
r[molten.runtime_spine.reference_lifetimes] Molten live references to local actors, dataspaces, protocol sessions, consensus resources, blob capabilities, and host resources MUST have explicit lifetime, revocation, and cleanup semantics. Cleanup MUST retract dependent assertions, subscriptions, pending operations, or handles where implemented.

#### Scenario: Session close cleans live references
- GIVEN a transport or protocol session introduced live references
- WHEN the session closes or is revoked
- THEN references scoped only to that session become invalid and dependent assertions, subscriptions, or pending operations are cleaned up.

### Requirement: Service dependencies are dataspace evidence
r[molten.runtime_spine.service_dependency_assertions] Molten MUST represent service demand, readiness, dependency, failure, completion, restart, shutdown, and exposed service references as canonical service runtime or supervision evidence. Demand-driven startup MUST wait for dependency readiness and emit receipt-backed diagnostics for missing, denied, or cyclic dependencies.

#### Scenario: Dependency delays service start
- GIVEN a service demand depends on another service readiness assertion
- WHEN the dependency is not ready
- THEN the runtime withholds startup and emits wait or deny diagnostics without asserting readiness.

#### Scenario: Service readiness publishes state
- GIVEN a demanded service starts with satisfied dependencies
- WHEN readiness checks pass
- THEN the runtime emits a readiness assertion and lifecycle/status evidence.

### Requirement: Supervision is logical, not OS parentage
r[molten.runtime_spine.supervision_tree] Molten MUST model logical supervision relationships independently from OS process parentage or adapter-specific process trees. Supervision evidence MUST bind failure markers, lifecycle receipts, monitor notifications, restart decisions, cleanup receipts, and diagnostics without granting service authority by itself.

#### Scenario: Supervised adapter process is logical child
- GIVEN an adapter process has an OS parent unrelated to Molten's logical supervisor
- WHEN Molten emits supervision evidence
- THEN the service appears under its logical supervisor in Molten evidence regardless of OS parentage.

### Requirement: Demand drives startup and shutdown
r[molten.runtime_spine.demand_driven_startup] Molten MUST use explicit service demand and dependency evidence to start, keep alive, restart, or shut down services without relying on hardcoded service graphs. Shutdown and cleanup remain receipt-backed and policy/resource-gated.

#### Scenario: Removing demand allows shutdown
- GIVEN a service has no remaining demand or reverse-dependency evidence
- WHEN shutdown is admitted
- THEN the runtime may stop the service and retract or clean up service state evidence.

### Requirement: Interaction tracing is canonical evidence
r[molten.runtime_spine.interaction_tracing] Molten MUST represent committed turns, actor lifecycle events, assertions, retractions, messages, policy decisions, runtime predicate receipts, service turn contexts, replay divergence records, choreography transitions, consensus events, and associated receipt refs as canonical Preserves evidence where those events cross runtime or audit boundaries.

#### Scenario: Turn emits trace context
- GIVEN a local runtime or service turn commits an assertion or message
- WHEN trace or report evidence is emitted
- THEN the evidence identifies actor or service context, committed actions, state refs, and receipt or policy evidence refs.

#### Scenario: Protocol and consensus events are traceable
- GIVEN a choreography endpoint transition or Raft-backed consensus commit is exposed to runtime tracing
- WHEN the event is recorded
- THEN the trace evidence identifies protocol/session or consensus group/term/index metadata with associated refs.

### Requirement: Trace inspection is evidence-only
r[molten.runtime_spine.trace_rendering] Molten SHOULD expose inspection, summary, replay, or export surfaces for canonical trace/report records. Rendered summaries MUST remain non-normative views and MUST NOT replace canonical receipts, policy gates, or replay validation.

#### Scenario: Operator inspects filtered trace evidence
- GIVEN runtime or service trace/report evidence exists
- WHEN an operator filters or renders it by actor, service, protocol, or consensus metadata
- THEN the output is derived from canonical evidence and does not grant authority.

### Requirement: SAM runtime tests cover implemented surfaces
r[molten.runtime_spine.sam_integration_tests] Molten MUST include tests for implemented SAM-style surfaces, including turn rollback, assertion cleanup, Observe delivery and retraction behavior, authority attenuation denial, gatekeeper resolution, service dependency startup, supervision cleanup, and trace/report emission.

#### Scenario: Assertion lifecycle integration test
- GIVEN an observer and asserting actor in a local dataspace
- WHEN the actor asserts and then its scope is cleaned up
- THEN tests show the assertion visibility and cleanup evidence follow the owner lifecycle.

### Requirement: SAM runtime properties are bounded
r[molten.runtime_spine.sam_property_tests] Molten SHOULD use bounded Hegel/property tests for implemented assertion, retraction, subscription, owner-lifetime, service dependency, and runtime predicate invariants.

#### Scenario: Generated assertion ownership preserves visibility invariant
- GIVEN a generated bounded sequence of assertion owners, duplicate assertions, retractions, and owner cleanup
- WHEN the runtime predicate model evaluates visibility
- THEN an assertion is visible exactly when at least one live owner still maintains that canonical assertion.

### Requirement: Canonical retention pins and classes
r[molten.retention.pin_model] Molten MUST represent retention pins as canonical records that bind object ref, object kind, pin source, reason, owner, expiry, policy refs, and evidence refs.

#### Scenario: Pin binds object and owner
- GIVEN an object ref and an owner with retention authority
- WHEN the object is pinned
- THEN the pin record binds the object, owner, source, policy refs, and evidence refs

r[molten.retention.classes] Molten MUST classify retained objects as ephemeral cache, debug trace, replay snapshot, audit receipt, durable value, public artifact, private secret ref, upgrade rollback, or legal hold.

#### Scenario: Unknown class is rejected
- GIVEN a retention request naming an unsupported class
- WHEN a pin or GC decision is evaluated
- THEN the request fails closed before changing retention state

r[molten.retention.no_name_gc] Molten MUST NOT treat mutable names, aliases, tags, or channels as sufficient proof that content is safe to delete.

#### Scenario: Name absence is not GC evidence
- GIVEN an object with no current human-readable name
- WHEN GC eligibility is evaluated
- THEN eligibility still depends on the reference index, pins, policy refs, and evidence refs

r[molten.retention.receipts] Molten MUST emit canonical receipts for pin, unpin, retain, eligibility, delete, tombstone, redact, compact, and denial decisions.

#### Scenario: Decision receipt records diagnostics
- GIVEN a retention decision that is denied
- WHEN the receipt is emitted
- THEN the receipt records the action, object ref, reference index ref, and denial diagnostics

### Requirement: Reference-index based GC eligibility
r[molten.retention.reference_index] Molten MUST build a bounded reference index covering active sessions, artifacts, blobs, receipts, snapshots, transcripts, docs, policies, upgrades, storage refs, remote cache refs, and evaluation cache refs before GC decisions.

#### Scenario: Index binds active pins
- GIVEN an object with active retention pins
- WHEN the reference index is built
- THEN the index lists those pin refs and binds the target object ref

r[molten.retention.gc_eligibility] Molten MUST deny GC unless the reference index proves no active or retained dependency remains.

#### Scenario: Incomplete proof denies deletion
- GIVEN an incomplete reference proof for an object
- WHEN deletion is requested
- THEN deletion is denied before any object is removed or tombstoned

r[molten.retention.operator_holds] Molten MUST support operator, legal, and compliance holds as explicit authority-bound retention pins.

#### Scenario: Legal hold blocks destructive action
- GIVEN an object under a legal hold retention class or pin source
- WHEN a destructive retention action is requested
- THEN the action is denied until the hold is cleared by explicit authority

r[molten.retention.cache_pins] Molten MUST distinguish remote sync cache pins and evaluation cache pins from durable pins.

#### Scenario: Cache pin is indexed separately
- GIVEN a remote cache or evaluation cache pin
- WHEN the reference index is built
- THEN the pin source remains visible in the index and decision receipt diagnostics

### Requirement: Auditable deletion, tombstone, redaction, and compaction
r[molten.retention.tombstones] Molten MUST represent deleted or redacted content with canonical tombstone records that expose audit metadata without leaking private content.

#### Scenario: Private redaction leaves public tombstone
- GIVEN a private secret ref eligible for redaction
- WHEN redaction is admitted
- THEN a tombstone records the object kind, class, action, policy refs, and evidence refs without plaintext

r[molten.retention.compaction] Molten MUST define compaction decisions as retention receipts that preserve audit semantics and deny compaction for classes where summaries would hide required evidence.

#### Scenario: Private secret compaction is denied
- GIVEN a private secret ref
- WHEN compaction is requested
- THEN compaction is denied unless policy explicitly provides a safe redaction path

r[molten.retention.secret_redaction_hook] Molten MUST coordinate private secret retention with redaction and reveal policy so secret refs are not physically removed without tombstone evidence.

#### Scenario: Secret redaction uses retention evidence
- GIVEN an encrypted ref from a private repro bundle
- WHEN it is redacted or tombstoned
- THEN the retention receipt binds policy and evidence refs for the redaction decision

r[molten.retention.remote_gc] Molten MUST consider remote replica and cache refs before local deletion without assuming global deletion authority.

#### Scenario: Remote refs block incomplete proof
- GIVEN remote cache refs that have not been reconciled
- WHEN GC eligibility is evaluated with incomplete proof
- THEN deletion is denied with a remote-cache diagnostic

### Requirement: Retention tests and invariants
r[molten.retention.pin_tests] Molten MUST test that pinned artifacts, blobs, receipts, and private refs cannot be deleted.

#### Scenario: Pinned object deletion test
- GIVEN an object with an active retention pin
- WHEN deletion is requested
- THEN the retention receipt denies deletion

r[molten.retention.eligibility_tests] Molten MUST test that objects become eligible only after pins and retained refs are cleared.

#### Scenario: Unpin then tombstone
- GIVEN a pinned object
- WHEN the pin is removed by authority and no retained refs remain
- THEN a tombstone action can pass

r[molten.retention.tombstone_tests] Molten MUST test that replay and audit summaries can explain tombstoned or redacted content.

#### Scenario: Tombstone summary avoids plaintext
- GIVEN a redacted private ref tombstone
- WHEN it is summarized
- THEN the summary explains the action without revealing private content

r[molten.retention.property_tests] Molten MUST test no-dangling-retained-ref and deny-on-incomplete-proof invariants within bounded generated cases.

#### Scenario: Generated retained refs deny deletion
- GIVEN generated finite retained-ref sets
- WHEN GC eligibility is evaluated
- THEN non-empty retained refs or incomplete proofs deny deletion

### Requirement: Retention-gated destructive paths
r[molten.retention.ledger_gc_gate] Molten MUST gate evidence-ledger garbage collection through passing retention receipts before removing ledger content.

#### Scenario: Ledger GC denied before removal
- GIVEN a ledger artifact with an active retention pin
- WHEN ledger GC evaluates the artifact for removal
- THEN GC emits denial evidence and does not remove the artifact

r[molten.retention.chunk_gc_gate] Molten MUST gate chunk-store manifest and chunk removal through passing retention receipts before removing files or writing tombstone receipts.

#### Scenario: Chunk GC denied before tombstone
- GIVEN an unpinned chunk-store manifest that is still retention-pinned
- WHEN chunk GC evaluates the manifest for removal
- THEN no manifest or chunk is removed and the GC receipt binds the denying retention receipt

r[molten.retention.eval_cache_tombstone_gate] Molten MUST gate evaluation-cache invalidation tombstones through passing retention receipts before writing tombstone entries.

#### Scenario: Cache tombstone denied before mutation
- GIVEN an evaluation-cache key with active retention evidence
- WHEN invalidation selects the key
- THEN the tombstone is not written unless retention eligibility passes

r[molten.retention.secret_cleanup_gate] Molten MUST require secret cleanup receipts to bind actual passing retention receipts for the cleaned secret and tombstone.

#### Scenario: Secret cleanup rejects stale retention evidence
- GIVEN a secret cleanup request with missing or mismatched retention evidence
- WHEN the cleanup receipt is built
- THEN cleanup is denied and diagnostics identify the retention mismatch

### Requirement: Subsystem retention evidence
r[molten.retention.subsystem_receipt_refs] Molten MUST expose retention receipt refs in ledger GC, chunk GC, cache invalidation, and secret cleanup receipts without treating them as authority grants.

#### Scenario: Subsystem receipt binds retention refs
- GIVEN a destructive subsystem decision that evaluated retention
- WHEN the subsystem receipt is emitted
- THEN the receipt lists the retention receipt refs that informed the decision

r[molten.retention.destructive_gate_tests] Molten MUST test pass and fail-closed retention-gated destructive paths for ledger GC, chunk GC, cache tombstones, and secret cleanup.

#### Scenario: Denials leave content intact
- GIVEN bounded generated or fixture destructive candidates with incomplete or denied retention decisions
- WHEN the subsystem attempts cleanup
- THEN tests verify content remains intact and denial receipts are auditable

### Requirement: Explicit destructive retention evidence
r[molten.retention.destructive_evidence_inputs] Molten MUST require destructive subsystem retention evaluations to accept explicit requester, policy, authority, evidence, retained-reference, remote-reference, and reference-index completeness inputs rather than relying on mutable names or synthesized trust.

#### Scenario: Destructive caller supplies evidence inputs
- GIVEN a ledger GC, chunk GC, or cache invalidation candidate
- WHEN the subsystem evaluates retention eligibility
- THEN the retention evaluation binds the explicit requester, policy refs, evidence refs, retained refs, remote refs, and reference-index completeness supplied by the caller

r[molten.retention.apply_requires_authority] Molten MUST deny apply-mode destructive candidates when requester, policy, authority, or supporting evidence refs are missing.

#### Scenario: Missing authority denies before removal
- GIVEN an apply-mode destructive candidate without delete authority evidence
- WHEN the subsystem attempts removal or tombstoning
- THEN the operation emits denial evidence and does not remove or tombstone the object

r[molten.retention.reference_index_plumbing] Molten MUST pass retained refs, remote refs, and reference-index completeness through destructive subsystem retention checks so incomplete proofs fail closed.

#### Scenario: Remote uncertainty blocks apply
- GIVEN a destructive candidate with unresolved remote cache refs or an incomplete reference index
- WHEN apply-mode GC or invalidation evaluates the candidate
- THEN deletion or tombstoning is denied before mutation

### Requirement: Destructive retention evidence receipts
r[molten.retention.cli_evidence_flags] Molten MUST expose operator-facing CLI flags for destructive retention requester, policy, authority, evidence, retained, remote, and reference-index completeness inputs.

#### Scenario: CLI surfaces missing evidence
- GIVEN a destructive CLI command without required retention evidence flags
- WHEN candidates are selected for apply-mode mutation
- THEN the command emits a denial receipt and reports the missing evidence diagnostics

r[molten.retention.evidence_summary_receipts] Molten MUST bind retention evidence summaries in subsystem GC and invalidation receipts without treating those summaries as authority grants.

#### Scenario: Receipt records evidence summary
- GIVEN a destructive subsystem decision
- WHEN the subsystem receipt is emitted
- THEN it records the retention receipt refs and the retention evidence inputs that informed the decision

r[molten.retention.destructive_evidence_tests] Molten MUST test fail-closed destructive retention evidence behavior for missing authority, missing policy, missing evidence, incomplete indexes, retained refs, and remote uncertainty.

#### Scenario: Evidence tests leave content intact
- GIVEN destructive candidates with incomplete or missing retention evidence
- WHEN subsystem cleanup runs
- THEN tests verify denial receipts are auditable and selected content remains intact

### Requirement: Retention evidence admission model
r[molten.retention.evidence_admission_model] Molten MUST represent destructive retention policy, authority, supporting evidence, reference-index, and remote-GC inputs as typed local admission receipts with canonical ref binding.

#### Scenario: Admission receipt binds canonical ref
- GIVEN a destructive retention evidence admission value
- WHEN Molten parses the supplied admission ref
- THEN the ref matches the canonical admission value and the receipt kind is one of policy, authority, supporting-evidence, reference-index, or remote-GC

r[molten.retention.evidence_scope_binding] Molten MUST require destructive retention admission receipts to bind requester, object ref, object kind, retention class, and action before they can authorize or support mutation.

#### Scenario: Mismatched evidence denies deletion
- GIVEN an admission receipt for a different requester, object, class, or action
- WHEN ledger GC, chunk GC, or cache invalidation evaluates a candidate
- THEN the destructive operation is denied before removal or tombstone mutation

### Requirement: Destructive admission gates
r[molten.retention.destructive_admission_gate] Molten MUST gate destructive ledger GC, chunk GC, and cache invalidation on admitted policy, authority, supporting evidence, reference-index, and remote-GC receipts rather than on syntactic refs alone.

#### Scenario: Forged refs fail closed
- GIVEN syntactically valid refs that do not resolve to passing local admission receipts
- WHEN apply-mode destruction evaluates candidates
- THEN deletion or tombstoning is denied and content remains readable

r[molten.retention.admission_receipt_diagnostics] Molten MUST surface admitted retention refs and admission diagnostics in destructive subsystem receipts without treating policy or support evidence as authority grants.

#### Scenario: Receipt records admission result
- GIVEN a destructive subsystem decision
- WHEN the subsystem emits a receipt
- THEN the receipt lists retention receipt refs, admitted evidence refs, and diagnostics for missing, stale, revoked, mismatched, retained, incomplete-index, or remote-uncertain evidence

r[molten.retention.admission_tests] Molten MUST test destructive retention evidence admission for forged refs, wrong requester, wrong action, wrong object or class, missing reference-index proof, retained refs, unresolved remote refs, and passing admitted evidence.

#### Scenario: Admission tests prove fail-closed mutation
- GIVEN destructive candidates with incomplete or mismatched admission evidence
- WHEN subsystem cleanup runs
- THEN tests verify denial receipts are auditable and selected content remains intact

### Requirement: Retention GC dry-run plans
r[molten.retention.gc_plan_dry_run_ux] Molten MUST expose canonical retention GC dry-run plans that bind a destructive candidate, subsystem, action, requester, computed reference index, explicit destructive evidence inputs, policy gate, authority gate, supporting-evidence gate, reference-index gate, remote-GC gate, imported remote-clearance gate, local retention diagnostics, and final dry-run decision before any destructive mutation.

#### Scenario: Plan lists every destructive gate before mutation
- GIVEN a destructive retention candidate with explicit policy, authority, supporting evidence, reference-index, remote-GC, and remote-clearance inputs
- WHEN an operator requests a retention GC plan
- THEN Molten emits a `retention-gc-plan-v1` artifact that lists each gate and diagnostics without writing retention receipts, tombstones, or deleting content

#### Scenario: Plan evidence is not deletion authority
- GIVEN a passing retention GC plan artifact
- WHEN a destructive subsystem later attempts deletion, tombstoning, redaction, or compaction
- THEN the subsystem MUST still run normal retention admission and receipt generation, and MUST NOT treat the plan as authority, policy, resource, provenance, transport, execution, source-gate, or remote-GC clearance trust

### Requirement: Retention GC apply from plan
r[molten.retention.gc_apply_from_plan_ux] Molten MUST expose a retention GC apply workflow that requires a stored dry-run plan ref, recomputes the plan from its embedded candidate and destructive evidence immediately before mutation, denies on drift or failed admission before writing destructive retention receipts or tombstones, and emits a canonical apply receipt linking the original plan, recomputed plan, admitted evidence refs, retention receipt ref, and tombstone ref.

#### Scenario: Apply requires unchanged current plan
- GIVEN a passing `retention-gc-plan-v1` artifact and no retention state drift
- WHEN an operator applies retention GC with that plan ref
- THEN Molten recomputes the plan, observes the same plan ref, runs normal destructive admission and retention evaluation, and emits `retention-gc-apply-v1` evidence binding the plan, admission refs, retention receipt, and tombstone refs

#### Scenario: Drift denies before mutation
- GIVEN a `retention-gc-plan-v1` artifact and a later pin, retained dependency, stale admission, or changed remote clearance state
- WHEN an operator applies retention GC with the old plan ref
- THEN Molten emits a denial `retention-gc-apply-v1` receipt, records drift diagnostics, and does not write destructive retention receipts or tombstones

#### Scenario: Plan is not authority at apply time
- GIVEN a passing dry-run plan artifact
- WHEN the apply workflow evaluates destructive retention
- THEN Molten MUST still run normal policy, authority, supporting-evidence, reference-index, remote-GC, and imported remote-clearance admission and MUST NOT treat the plan itself as authority, policy, resource, provenance, transport, execution, source-gate, or remote-GC clearance trust

### Requirement: Retention GC execution gates
r[molten.retention.gc_execution_gates] Molten MUST require a matching passing retention GC apply receipt before non-dry-run destructive subsystem mutation and MUST emit canonical per-candidate execution gate evidence without treating plans, apply receipts, or execution gate receipts as authority, policy, resource, provenance, transport, execution, source-gate, or remote-GC clearance trust.

#### Scenario: Matching apply gates physical mutation
- GIVEN a passing retention GC plan and apply receipt for a ledger, chunk, or cache candidate
- WHEN a non-dry-run subsystem GC or invalidation operation attempts physical mutation for that candidate
- THEN Molten verifies the apply scope, plan binding, retention receipt, and tombstone refs, emits `retention-gc-execute-v1`, and only mutates after normal destructive admission and retention evaluation still pass

#### Scenario: Missing or wrong apply denies before mutation
- GIVEN a destructive subsystem candidate with no apply ref or an apply ref for a different object, class, action, or subsystem
- WHEN a non-dry-run GC or invalidation operation runs
- THEN Molten emits denial diagnostics and leaves the selected content or cache entry readable

#### Scenario: Fresh retention drift after apply still blocks execution
- GIVEN a passing apply receipt followed by a new pin, retained dependency, stale admission, or remote clearance change
- WHEN subsystem execution evaluates the candidate
- THEN Molten denies before physical mutation even if the apply receipt itself remains parseable

### Requirement: Retention GC audit UX
r[molten.retention.gc_audit_ux] Molten MUST expose a read-only retention GC audit workflow that starts from a stored execution gate ref, follows the bound plan, apply, retention receipt, and tombstone refs, emits canonical audit evidence with consistency diagnostics, and never treats the audit artifact as authority, policy, resource, provenance, transport, execution, source-gate, remote-GC clearance, or deletion trust.

#### Scenario: Audit shows passing destructive chain
- GIVEN a passing retention GC plan, apply receipt, execution gate, retention receipt, and tombstone for a destructive subsystem candidate
- WHEN an operator audits the execution gate ref
- THEN Molten emits `retention-gc-audit-v1` evidence that lists the plan, apply, execution, retention receipt, and tombstone refs with a passing audit decision

#### Scenario: Audit denies inconsistent chain
- GIVEN an execution gate whose linked apply, plan, receipt, or tombstone is missing, denied, or scope-mismatched
- WHEN an operator audits the execution gate ref
- THEN Molten emits denial diagnostics and does not mutate retained content or create deletion authority

#### Scenario: Audit remains explanatory evidence
- GIVEN a passing `retention-gc-audit-v1` artifact
- WHEN a destructive subsystem later attempts deletion, tombstoning, redaction, compaction, or invalidation
- THEN the subsystem MUST still require matching apply and execution gates plus normal destructive admission and MUST NOT treat the audit artifact as authority or clearance

### Requirement: Retention GC audit catalog search
r[molten.retention.gc_audit_catalog_search] Molten MUST classify retention GC plan, apply, execute, and audit artifacts in read-only catalog and MCP search results by stage, decision, subsystem, object ref, retention class, and chain refs while preserving normal retention deletion gates as the only destructive authority path.

#### Scenario: Catalog finds retention GC chains by scope
- GIVEN retention GC plan, apply, execute, and audit artifacts for a destructive candidate
- WHEN an operator searches the local catalog by object ref, subsystem, execution ref, or ledger kind
- THEN Molten returns the matching artifacts with retention GC classifications for plan, apply, execute, audit, and linked refs

#### Scenario: MCP search is read-only discovery
- GIVEN retention GC audit artifacts imported into the local ledger
- WHEN an MCP client calls the read-only `search_retention_gc` tool with stage, object, subsystem, decision, plan, apply, or execution filters
- THEN Molten returns catalog search evidence without mutating retention state or granting deletion authority

#### Scenario: Catalog discovery remains explanatory evidence
- GIVEN a passing catalog or MCP result for a retention GC audit chain
- WHEN a destructive subsystem later attempts deletion, tombstoning, redaction, compaction, or invalidation
- THEN the subsystem MUST still require matching plan/apply/execution gates plus normal destructive admission and MUST NOT treat catalog or MCP discovery as authority, policy, resource, provenance, transport, execution, source-gate, remote-GC clearance, or deletion trust

### Requirement: Retention candidate explain UX
r[molten.retention.candidate_explain_ux] Molten MUST expose a read-only retention candidate explain workflow that starts from an object ref, optionally narrows by object kind, retention class, action, and subsystem, and emits canonical evidence listing local pins, evidence admissions, remote clearances/imports, retention GC plans, applies, executions, audits, retention receipts, and tombstones without granting deletion authority.

#### Scenario: Explain lists known local evidence before destructive commands
- GIVEN a retention object with local pins, destructive evidence admissions, remote clearances, retention GC plan/apply/execute/audit artifacts, retention receipts, and tombstones
- WHEN an operator explains the candidate by object ref and optional scope filters
- THEN Molten emits `retention-candidate-explain-v1` evidence listing the matching refs and diagnostics without deleting, tombstoning, redacting, compacting, or invalidating content

#### Scenario: Explain is not a destructive gate
- GIVEN a passing `retention-candidate-explain-v1` artifact
- WHEN a destructive subsystem later attempts deletion, tombstoning, redaction, compaction, or invalidation
- THEN the subsystem MUST still require matching plan/apply/execution gates plus normal destructive admission and MUST NOT treat explain evidence as authority, policy, resource, provenance, transport, execution, source-gate, remote-GC clearance, remote-clearance-import, or deletion trust

#### Scenario: Audit artifacts become discoverable evidence
- GIVEN an operator has emitted a retention GC audit for an execution gate
- WHEN a later explain command scans the same retention root for that object
- THEN the explain artifact lists the known audit ref alongside the plan, apply, execute, retention receipt, and tombstone refs it explains

### Requirement: Retention candidate bundle export
r[molten.retention.candidate_bundle_export] Molten MUST expose a read-only retention candidate bundle export workflow that packages a supplied explain artifact, a canonical bundle manifest, and referenced local retention GC plan/apply/execute/audit, retention receipt, and tombstone artifacts without granting deletion authority.

#### Scenario: Bundle packages local explain evidence for handoff
- GIVEN a `retention-candidate-explain-v1` artifact that references local plan, apply, execute, audit, retention receipt, and tombstone artifacts
- WHEN an operator exports a retention candidate bundle
- THEN Molten writes `explain.preserves`, `bundle.preserves`, and grouped local artifact files for each readable referenced artifact

#### Scenario: Bundle reports missing local artifacts
- GIVEN an explain artifact that references a plan, apply, execute, audit, retention receipt, or tombstone artifact missing from the local retention root
- WHEN an operator exports a retention candidate bundle
- THEN Molten emits bundle diagnostics for the missing artifact and does not mint replacement evidence

#### Scenario: Bundle remains review evidence only
- GIVEN a passing `retention-candidate-bundle-v1` artifact
- WHEN a destructive subsystem later attempts deletion, tombstoning, redaction, compaction, or invalidation
- THEN the subsystem MUST still require matching plan/apply/execution gates plus normal destructive admission and MUST NOT treat bundle evidence as authority, policy, resource, provenance, transport, execution, source-gate, remote-GC clearance, remote-clearance-import, or deletion trust

### Requirement: Retention candidate bundle verification
r[molten.retention.candidate_bundle_verify] Molten MUST expose a read-only retention candidate bundle verification workflow that validates an exported bundle's manifest, explain artifact, and packaged local retention artifacts without granting deletion authority.

#### Scenario: Verification passes for an intact exported bundle
- GIVEN an exported retention candidate bundle whose `bundle.preserves`, `explain.preserves`, and grouped artifact files match their canonical refs and expected artifact kinds
- WHEN an operator verifies the bundle
- THEN Molten emits `retention-candidate-bundle-verify-v1` evidence with decision `pass`, the bundle ref, explain ref, listed artifact refs, observed file refs, and no diagnostics

#### Scenario: Verification diagnoses tampered or missing packaged artifacts
- GIVEN an exported retention candidate bundle with a missing, tampered, duplicate, unlisted, or unreferenced packaged artifact file
- WHEN an operator verifies the bundle
- THEN Molten emits `retention-candidate-bundle-verify-v1` evidence with decision `deny` and diagnostics identifying the inconsistent bundle evidence

#### Scenario: Verification remains review evidence only
- GIVEN a passing `retention-candidate-bundle-verify-v1` artifact
- WHEN a destructive subsystem later attempts deletion, tombstoning, redaction, compaction, or invalidation
- THEN the subsystem MUST still require matching plan/apply/execution gates plus normal destructive admission and MUST NOT treat verification evidence as authority, policy, resource, provenance, transport, execution, source-gate, remote-GC clearance, remote-clearance-import, import, or deletion trust

### Requirement: Retention bundle export profiles
r[molten.retention.bundle_export_profiles] Molten MUST expose profile-controlled retention candidate bundle export evidence that distinguishes internal full-fidelity bundles from public deny-sensitive and diagnostic redacted-review handoffs without granting deletion authority.

#### Scenario: Public profile denies sensitive handoff
- GIVEN a retention candidate bundle whose explain artifact or packaged local artifacts contain sensitive markers such as private-secret retention class or encrypted-ref object kind
- WHEN an operator exports the bundle with the public profile
- THEN Molten emits `retention-candidate-bundle-profile-v1` evidence with decision `deny`, marker refs, and diagnostics identifying that public handoff is not safe

#### Scenario: Diagnostic profile writes redacted review view
- GIVEN a retention candidate bundle with sensitive markers
- WHEN an operator exports the bundle with the diagnostic profile
- THEN Molten emits `retention-candidate-bundle-profile-v1` evidence with decision `pass`, marker refs, diagnostic-only loss classification, and redacted review copies that replace sensitive marker tokens

#### Scenario: Profiles remain review evidence only
- GIVEN a passing `retention-candidate-bundle-profile-v1` artifact or diagnostic redacted review view
- WHEN a destructive subsystem later attempts deletion, tombstoning, redaction, compaction, or invalidation
- THEN the subsystem MUST still require matching plan/apply/execution gates plus normal destructive admission and MUST NOT treat profile evidence or redacted views as authority, policy, resource, provenance, transport, execution, source-gate, remote-GC clearance, remote-clearance-import, verification, import, or deletion trust

### Requirement: Remote retention clearance receipts
r[molten.retention.remote_gc_clearance_receipts] Molten MUST represent per-remote destructive retention clearance as canonical receipts that bind peer ref, requester ref, object ref, object kind, retention class, action, remote ref, policy ref, authority ref, freshness, revocation refs, retained remote refs, diagnostics, and checks.

#### Scenario: Clearance binds remote scope
- GIVEN a remote GC clearance receipt
- WHEN Molten parses or admits the receipt for destructive retention
- THEN the receipt ref is canonical and its peer, requester, object, class, action, policy, authority, freshness, revocation, and retained-ref fields are checked before it can support local mutation

r[molten.retention.remote_gc_all_remotes] Molten MUST require every configured or known remote ref supplied to destructive retention evidence to have a current passing clearance before deletion, tombstoning, redaction, or compaction.

#### Scenario: Partial remote clearance denies
- GIVEN destructive retention evidence naming multiple remote refs
- WHEN only a subset has matching current clearance receipts
- THEN Molten denies the destructive operation before local mutation and reports the missing remote refs

### Requirement: Remote reconciliation destructive gate
r[molten.retention.remote_gc_reconciliation_gate] Molten MUST gate ledger GC, chunk GC, and eval-cache invalidation on reconciled per-remote clearance in addition to local policy, authority, supporting evidence, reference-index, and remote-GC admissions.

#### Scenario: Remote uncertainty blocks apply
- GIVEN a destructive ledger, chunk, or cache candidate with stale, revoked, forged, wrong-scope, or retained-remote clearance evidence
- WHEN apply-mode cleanup evaluates the candidate
- THEN the subsystem emits denial evidence and leaves selected content readable

r[molten.retention.remote_gc_diagnostics] Molten MUST surface per-peer and per-remote clearance diagnostics in destructive subsystem receipts without treating clearance receipts as authority, policy, resource, provenance, transport, execution, or source-gate trust.

#### Scenario: Receipt records clearance diagnostics
- GIVEN destructive retention admission with remote refs
- WHEN the subsystem emits its GC or invalidation receipt
- THEN the receipt diagnostics identify missing, stale, revoked, wrong-peer, wrong-object, wrong-action, retained, or forged remote clearance evidence

r[molten.retention.remote_gc_cli] Molten MUST expose operator-facing CLI support for creating remote clearance receipts and supplying remote clearance refs to destructive retention commands.

#### Scenario: CLI supplies remote clearance
- GIVEN an operator has per-remote clearance receipts
- WHEN the operator runs destructive ledger, chunk, or cache cleanup with remote clearance refs
- THEN Molten binds those refs into destructive retention evidence before admission

r[molten.retention.remote_gc_reconciliation_tests] Molten MUST test remote GC reconciliation for partial remote sets, stale or revoked clearance, wrong peer, wrong object or action, retained remote refs, forged refs, and an all-clear pass.

#### Scenario: Tests prove fail-closed remote reconciliation
- GIVEN destructive candidates with incomplete or mismatched remote clearance evidence
- WHEN subsystem cleanup runs
- THEN tests verify denial receipts are auditable and selected content remains intact

### Requirement: Remote clearance request and response artifacts
r[molten.retention.remote_clearance_request_response] Molten MUST represent peer-produced remote retention clearance workflows as canonical request and response artifacts that bind requester, peer, object, retention class, action, remote ref, policy, authority, supporting evidence, clearance value, diagnostics, and checks.

#### Scenario: Peer response binds request and clearance
- GIVEN a requester asks a peer for destructive remote GC clearance
- WHEN the peer emits a clearance response
- THEN the response binds the exact request ref and embedded clearance ref before the requester may import it

### Requirement: Remote clearance import gate
r[molten.retention.remote_clearance_import_gate] Molten MUST fail closed when importing remote clearance responses unless the request, response, and embedded clearance are current, passing, untampered, and scope-matching for the expected peer, remote ref, object, class, action, policy, and authority.

#### Scenario: Import stores only passing clearance
- GIVEN a response with stale, revoked, retained, wrong-peer, wrong-remote, wrong-request, or tampered clearance evidence
- WHEN the requester imports the response
- THEN Molten emits a denial receipt, does not store the clearance locally, and destructive admission still lacks clearance

r[molten.retention.remote_clearance_workflow_diagnostics] Molten MUST surface remote clearance workflow diagnostics without treating request, response, clearance, or import receipts as authority, policy, resource, provenance, transport, execution, or source-gate trust.

#### Scenario: Workflow diagnostics remain evidence-only
- GIVEN an imported remote clearance response
- WHEN the import receipt is rendered or supplied to destructive retention flows
- THEN diagnostics identify the clearance workflow decision while local authority and policy admissions remain separate requirements

### Requirement: Remote clearance workflow CLI and tests
r[molten.retention.remote_clearance_workflow_cli] Molten MUST expose CLI commands for building remote clearance requests, producing peer responses, importing responses, and showing workflow artifacts.

#### Scenario: Operator imports peer clearance
- GIVEN an operator has a request and a peer response
- WHEN the operator runs the import command
- THEN Molten writes an import receipt and stores the embedded clearance only if all workflow bindings pass

r[molten.retention.remote_clearance_workflow_tests] Molten MUST test pass import, retained or stale peer denial, wrong request or peer denial, tampered response denial, and destructive admission using imported peer clearance.

#### Scenario: Tests prove workflow fail-closed behavior
- GIVEN destructive cleanup depends on remote clearance produced through the workflow
- WHEN incomplete or mismatched workflow evidence is supplied
- THEN tests verify denial receipts are auditable and selected content remains intact

### Requirement: Remote clearance live transport receipts
r[molten.retention.remote_clearance_live_transport] Molten MUST carry `retention-remote-gc-clearance-request-v1` and `retention-remote-gc-clearance-response-v1` artifacts through node-control live workflow receipts that bind request ref, response ref, requester, peer, remote ref, object ref, action, and diagnostics.

#### Scenario: Live workflow binds request and response
- GIVEN a requester sends a remote retention clearance request over live node-control workflow transport
- WHEN the peer responds
- THEN Molten records live workflow evidence that binds the exact request and response refs without treating transport delivery as deletion authority

### Requirement: Remote clearance live import gate
r[molten.retention.remote_clearance_live_import_gate] Molten MUST still import live remote clearance responses through `retention-remote-gc-clearance-import-v1` before destructive retention admission may use the embedded peer clearance.

#### Scenario: Import remains the deletion-safety gate
- GIVEN a live response from a peer
- WHEN the response is retained, stale, revoked, wrong-scope, or tampered
- THEN Molten emits denial evidence, does not store the embedded clearance locally, and destructive admission lacks remote clearance

### Requirement: Remote clearance live CLI and diagnostics
r[molten.retention.remote_clearance_live_cli] Molten MUST expose deterministic CLI support for exercising remote clearance request/respond/import over live loopback transport.

#### Scenario: Operator runs loopback live clearance
- GIVEN an operator has retention policy, authority, requester, peer, object, and remote refs
- WHEN the operator runs the live loopback clearance command
- THEN Molten emits request, response, import, and live workflow receipts that can be inspected by ref

r[molten.retention.remote_clearance_live_diagnostics] Molten MUST surface live transport diagnostics for missing, retained, stale, revoked, wrong-peer, wrong-request, wrong-remote, and tampered response evidence without treating live receipts as authority, policy, resource, provenance, execution, source-gate, or remote-GC clearance trust.

#### Scenario: Live diagnostics remain evidence-only
- GIVEN a live clearance workflow denial
- WHEN the live receipt is rendered or supplied with destructive evidence
- THEN diagnostics identify the transport and clearance failure while local authority and policy admissions remain separate requirements

### Requirement: Remote clearance live multi-host request send
r[molten.retention.remote_clearance_live_multihost_request] Molten MUST expose a requester-side live command that stores a canonical `retention-remote-gc-clearance-request-v1` artifact and sends a node-control live ingress request bound to the request ref, requester node, peer node, topic, sequence, authority, policy, resource, peer-bootstrap, and evidence refs.

#### Scenario: Requester sends clearance request evidence
- GIVEN a requester has a remote clearance request scope and a peer live ticket
- WHEN the requester runs the live request-send command
- THEN Molten writes the request artifact, node-control request artifact, and send receipt without treating the send receipt as clearance or authority

### Requirement: Remote clearance live multi-host response send
r[molten.retention.remote_clearance_live_multihost_response] Molten MUST expose a peer-side live command that reads a request artifact, stores a canonical `retention-remote-gc-clearance-response-v1` artifact, and sends a node-control live ingress request bound to the response ref and original request ref back to the requester.

#### Scenario: Peer sends clearance response evidence
- GIVEN a peer has received a remote clearance request artifact
- WHEN the peer runs the live response-send command
- THEN Molten writes the response artifact, node-control request artifact, and send receipt without storing requester-side clearance

### Requirement: Remote clearance live multi-host import workflow
r[molten.retention.remote_clearance_live_multihost_import] Molten MUST expose a requester-side import workflow command that imports a peer response through `retention-remote-gc-clearance-import-v1` and stores `retention-remote-gc-clearance-live-workflow-v1` evidence binding request, response, import, send, receive, and ingress refs.

#### Scenario: Requester imports live peer response
- GIVEN a requester has the original request, peer response, node-control send receipts, receive receipts, and ingress refs
- WHEN the requester runs the live import workflow command
- THEN Molten emits an import receipt and live workflow receipt that bind all provided evidence refs

### Requirement: Remote clearance live multi-host import gate
r[molten.retention.remote_clearance_live_multihost_import_gate] Molten MUST keep `retention-remote-gc-clearance-import-v1` as the only live multi-host step that stores embedded peer clearance for destructive admission.

#### Scenario: Live transport evidence is not deletion clearance
- GIVEN live request and response send receipts exist without a passing import receipt
- WHEN destructive retention admission evaluates remote peer clearance
- THEN Molten denies because live transport receipts are not accepted as remote clearance

### Requirement: Remote clearance live multi-host diagnostics
r[molten.retention.remote_clearance_live_multihost_diagnostics] Molten MUST surface multi-host live diagnostics for missing or denied send, receive, ingress, wrong-peer, wrong-request, wrong-remote, retained, stale, revoked, and tampered-response evidence without treating live transport receipts as authority, policy, resource, provenance, execution, source-gate, or remote-GC trust.

#### Scenario: Denied transport fails closed
- GIVEN a live send receipt is denied or lacks a matching transport receipt
- WHEN the requester assembles the live workflow
- THEN Molten records transport diagnostics and the live workflow fails closed unless the import and evidence bindings pass

### Requirement: Remote clearance live multi-host tests
r[molten.retention.remote_clearance_live_multihost_tests] Molten MUST test request send, response send, final workflow assembly, denied transport evidence, and destructive admission through imported peer clearance.

#### Scenario: Tests prove multi-host safety boundary
- GIVEN multi-host live remote-clearance tests execute
- WHEN live evidence is complete or incomplete
- THEN passing cases require imported peer clearance and incomplete transport records fail closed with diagnostics

### Requirement: Remote clearance live two-node harness
r[molten.retention.remote_clearance_live_two_node_harness] Molten MUST test the live multi-host remote-clearance happy path with two local node roots, bound live tickets, real node-control live send receipts, real receive receipts, real ingress refs, final import-workflow evidence, and destructive admission through imported peer clearance.

#### Scenario: Two-node live clearance succeeds through imported clearance
- GIVEN requester and peer node roots have bound live tickets, peer-admission evidence, and authority grants for the `gate` live ingress operation
- WHEN the requester sends a clearance request to the peer, the peer receives it, the peer sends a response to the requester, and the requester receives it
- THEN Molten imports the peer response through `retention-remote-gc-clearance-import-v1`, stores a passing live workflow bound to real send/receive/ingress refs, and passes destructive admission only through the imported peer clearance

### Requirement: Remote clearance live tests
r[molten.retention.remote_clearance_live_tests] Molten MUST test passing live loopback import, retained or stale peer denial, wrong peer or request denial, tampered response denial, and destructive admission using imported live clearance.

#### Scenario: Tests prove live transport fail-closed behavior
- GIVEN destructive cleanup depends on live peer clearance
- WHEN live transport, request, response, or import evidence is incomplete or mismatched
- THEN tests verify denial receipts are auditable and selected content remains intact

### Requirement: Operator dogfood retention GC workflow
r[molten.operator_dogfood_retention_gc.workflow] Molten MUST exercise retention GC deletion-safety rails in the local operator dogfood workflow by emitting canonical retention evidence admissions, remote-GC clearance evidence, dry-run plan, apply, execution gate, audit, explain, bundle export/profile/verify, and catalog/MCP discovery artifacts under the explicit dogfood state root.

#### Scenario: Dogfood records retention GC chain
- GIVEN a clean local dogfood state root
- WHEN the operator dogfood workflow runs
- THEN the workflow emits mandatory recorded or deterministic steps for retention GC plan, apply, execute, audit, review bundle verification, and read-only catalog discovery

r[molten.operator_dogfood_retention_gc.release_gate] Molten MUST bind retention GC dogfood refs into operator checkpoints, dogfood reports, release evidence, and local ledger/catalog imports so release review can inspect the deletion-safety chain.

#### Scenario: Release evidence includes retention review artifacts
- GIVEN a passing local dogfood run
- WHEN the dogfood report and release gate receipt are inspected
- THEN they include retention GC step receipts, bundle verification evidence, catalog discovery receipts, and imported retention ledger artifacts

r[molten.operator_dogfood_retention_gc.evidence_only] Molten MUST treat operator dogfood retention GC artifacts as evidence-only release diagnostics that do not replace retention admission, plan, apply, execution, remote clearance, tombstone, policy, authority, provenance, resource, transport, source-gate, remote-GC, or destructive subsystem gates.

#### Scenario: Dogfood evidence does not grant deletion authority
- GIVEN a passing dogfood retention GC workflow
- WHEN a destructive subsystem later attempts deletion, tombstoning, compaction, redaction, ledger GC, chunk GC, or cache invalidation
- THEN the subsystem still requires matching normal retention gates and MUST NOT treat dogfood report, release gate, bundle verify, audit, explain, or catalog search receipts as deletion authority

r[molten.operator_dogfood_retention_gc.tests] Molten MUST test the dogfood retention GC workflow with passing local fixtures and fail-closed coverage for missing or denied mandatory retention evidence.

#### Scenario: Tests cover dogfood retention evidence
- GIVEN the local dogfood test harness
- WHEN the dogfood workflow is executed in tests
- THEN tests assert the retention GC steps, bundle verify evidence, catalog discovery evidence, and pass report are present

### Requirement: Coordination apply reports are canonical
r[molten.coordination_control_plane_ux.apply_report] Molten MUST emit canonical coordination apply reports that bind the manifest ref, final state ref, receipt refs, assertion refs, and supporting evidence refs.

#### Scenario: Batch apply report binds evidence
- GIVEN coordination request artifacts are applied through the control-plane runtime
- WHEN Molten writes the batch report
- THEN the report records the final coordination state ref
- AND the report binds every coordination receipt and supporting evidence ref.

### Requirement: Coordination show remains read-only
r[molten.coordination_control_plane_ux.readonly_show] Molten MUST summarize coordination artifacts without mutating coordination state or importing authority.

#### Scenario: Operator summarizes generated artifacts
- GIVEN a manifest, request, receipt, token, state snapshot, assertion, or apply report artifact
- WHEN the operator runs `molten test coordination show`
- THEN Molten prints a read-only summary
- AND no control-plane mutation is performed.

### Requirement: Manifest and request CLI emits canonical records
r[molten.coordination_control_plane_ux.manifest_request_cli] Molten MUST provide CLI commands that generate canonical coordination service manifests and requests with explicit operation-id, authority, policy, and resource refs.

#### Scenario: Request generation binds explicit evidence
- GIVEN an operator supplies service, operation, key, client session, operation id, authority refs, policy refs, resource refs, and an optional payload file
- WHEN `molten test coordination request` runs
- THEN it emits a canonical `coordination-request-v1`
- AND the request binds the supplied evidence refs without granting authority by itself.

### Requirement: Batch apply uses the control-plane runtime
r[molten.coordination_control_plane_ux.apply_batch_cli] Molten MUST apply coordination request files through the admitted control-plane runtime and MUST NOT mutate coordination state through ordinary actor messages or direct state edits.

#### Scenario: Queue request commits through apply
- GIVEN a coordination manifest and queue enqueue request artifact
- WHEN `molten test coordination apply` runs
- THEN the request is applied through the coordination control-plane state machine
- AND the output directory contains a coordination receipt and state evidence.

### Requirement: Duplicate operation ids replay without a second mutation
r[molten.coordination_control_plane_ux.idempotent_replay] Molten MUST preserve coordination operation-id idempotency when the CLI applies duplicate request artifacts in a batch.

#### Scenario: Duplicate request returns prior receipt
- GIVEN the same mutating coordination request appears twice in a batch
- WHEN the batch is applied
- THEN the second application returns the prior receipt ref
- AND the final state is not advanced a second time.

### Requirement: Coordination CLI behavior is tested
r[molten.coordination_control_plane_ux.cli_tests] Molten SHOULD cover coordination manifest, request, apply, show, and duplicate replay behavior in automated tests.

#### Scenario: CLI test exercises duplicate replay
- GIVEN the CLI test suite runs
- WHEN it applies the same coordination request twice
- THEN the apply report contains matching receipt refs
- AND the test observes a successful batch decision.

### Requirement: Coordination UX is documented
r[molten.coordination_control_plane_ux.docs] Molten SHOULD document the coordination control-plane UX and state that its receipts are evidence only.

#### Scenario: Operator reads the documentation
- GIVEN an operator reviews the Molten README or architecture notes
- WHEN they inspect coordination control-plane commands
- THEN the docs describe manifest, request, apply, and show usage
- AND the docs clarify that CLI artifacts do not grant authority, policy, resource, transport, or provenance trust.

### Requirement: Blob-ref job submissions
r[molten.blob_ref_jobs.payload_model] Molten MUST expose canonical job submission artifacts that reference executable and input content by artifact, blob, or chunk refs instead of embedding large bytes.
r[molten.blob_ref_jobs.no_inline_large_bytes] Molten MUST reject ref-backed job submissions that embed large executable, input, output, log, or dataset bytes instead of content refs.

#### Scenario: Content-ref-only submission
- GIVEN a job id, operation id, executable ref, input refs, size/format hints, authority context ref, policy refs, provenance refs, effect refs, and output mode
- WHEN an operator creates a ref-backed job submission
- THEN Molten MUST emit a canonical `job-ref-submission-v1` artifact that records those refs and the checks `content-refs-only` and `no-inline-large-bytes`

#### Scenario: Inline large content is denied
- GIVEN a ref-backed job submission that contains inline executable, input, dataset, output, or log bytes
- WHEN Molten parses or admits the submission
- THEN Molten MUST deny it before execution and require content refs for large content

### Requirement: Blob-ref worker fetch and verification
r[molten.blob_ref_jobs.local_worker] Molten MUST provide a deterministic local worker that fetches refs from a local chunk/blob store, verifies them, runs the declared handler profile, and stores outputs by ref.
r[molten.blob_ref_jobs.content_verification] Molten MUST verify executable/input chunk manifests or blob hashes before execution.
r[molten.blob_ref_jobs.provenance_policy] Molten MUST require explicit provenance and effect/policy refs for executable refs before treating the run as admitted execution evidence.
r[molten.blob_ref_jobs.retention_pins] Molten MUST pin executable, input, and output refs while a job is active and emit cleanup evidence when active pins are released.

#### Scenario: Verified local worker execution
- GIVEN a valid ref-backed job submission and a local chunk store containing the executable and input manifests
- WHEN the deterministic local worker executes the job
- THEN Molten MUST read the manifests, verify them before execution, pin active content refs, run the declared handler profile, store result bytes as content refs, and emit status and receipt artifacts for the run

#### Scenario: Missing or tampered content ref
- GIVEN a ref-backed job submission whose executable or input ref cannot be fetched or verified
- WHEN the deterministic local worker attempts execution
- THEN Molten MUST deny the run before invoking the handler and emit diagnostics plus a canonical denial receipt

### Requirement: Blob-ref job status and receipt evidence
r[molten.blob_ref_jobs.status_assertions] Molten MUST expose job status assertions for queued, fetching, running, complete, failed, cancelled, and result-ready states.
r[molten.blob_ref_jobs.receipts] Molten MUST emit receipts that bind submission, fetch, verification, admission, execution, result, cleanup, and denial evidence.
r[molten.blob_ref_jobs.replay_integration] Molten MUST include job refs, fetch receipts, verification receipts, and handler profile identity in deterministic replay identity.

#### Scenario: Status lifecycle evidence
- GIVEN a ref-backed job execution
- WHEN the worker progresses through queued, fetching, running, result-ready, complete, failed, or cancelled states
- THEN Molten MUST emit canonical `job-ref-status-v1` evidence records that bind the submission ref, operation id, output refs, and checks for the state

#### Scenario: Receipt replay identity
- GIVEN a ref-backed job execution receipt
- WHEN Molten summarizes, stores, or replays the receipt
- THEN the receipt MUST include the submission ref, job id, operation id, executable ref, input refs, status refs, fetch refs, verification refs, pin refs, cleanup refs, output ref, handler profile outcome, diagnostics, and pass/fail checks needed to reproduce the decision

### Requirement: Blob-ref job DAG integration
r[molten.blob_ref_jobs.job_dag_integration] Molten SHOULD integrate ref-backed job submissions with existing job DAG and delivery evidence surfaces without granting implicit authority.
r[molten.blob_ref_jobs.local_tests] Molten MUST test submitting, fetching, verifying, running, and completing a local ref-backed job.
r[molten.blob_ref_jobs.property_tests] Molten MUST include property coverage for no-inline-large-bytes, content verification before execution, and pin lifecycle invariants.

#### Scenario: CLI and ledger integration
- GIVEN a ref-backed job submission and execution receipt
- WHEN an operator uses the job CLI status or receipt commands
- THEN Molten SHOULD display the ref-backed receipt alongside existing job DAG receipts and ledger classifications

#### Scenario: Evidence only
- GIVEN a passing ref-backed job receipt
- WHEN another runtime operation evaluates authority, provenance, policy, resource, or transport admission
- THEN the ref-backed receipt MUST be treated as execution evidence only and MUST NOT grant authority, provenance, policy, resource, transport, or source-gate trust by itself

### Requirement: Services start from demand and admitted authority
r[molten.sam_service_supervision.spec.demand_start] A service MUST start only when a matching demand assertion exists and startup is admitted by explicit authority, policy, resource, and effect-handle evidence.

#### Scenario: Demand starts service
- GIVEN a service manifest and a demand assertion for that service
- AND authority/resource evidence admits startup
- WHEN the service runtime evaluates demand
- THEN it emits a service lifecycle receipt with decision `pass`
- AND publishes owned readiness or startup assertions through the dataspace

#### Scenario: Missing authority denies startup
- GIVEN a demand assertion for a service
- AND no authority context admitting service startup
- WHEN the runtime evaluates demand
- THEN startup is denied before actor execution
- AND no readiness assertion is committed

### Requirement: Supervision is logical and deterministic
r[molten.sam_service_supervision.spec.supervision] Service links, monitors, restart decisions, and failure propagation MUST be expressed as canonical dataspace/evidence records and MUST replay deterministically.

#### Scenario: Failure notifies monitor
- GIVEN a monitored service fails during a turn
- WHEN the failure commits
- THEN monitor assertions/events are emitted in deterministic order
- AND the lifecycle receipt binds the failure and monitor refs

#### Scenario: Restart budget is exhausted
- GIVEN a service restart policy with a bounded restart rate
- WHEN failures exceed the bound
- THEN the runtime emits a deny receipt
- AND publishes a final failed/stopped assertion instead of restarting indefinitely

#### Scenario: Supervision gate receipt is evidence only
- GIVEN a canonical service supervision report
- WHEN an operator gates the report
- THEN the runtime emits a service supervision gate receipt binding report, suite, restart, monitor, status, and cleanup evidence
- AND the receipt decision is derived by deterministic replay
- AND the receipt is not authority, provenance, resource, policy, or transport trust evidence

### Requirement: Cleanup retracts owned state
r[molten.sam_service_supervision.spec.cleanup] Service termination, failure, shutdown, or authority revocation MUST retract service-owned assertions, observers, live refs, and pending effect intents.

#### Scenario: Revocation cleans assertions
- GIVEN a running service with owned readiness and exposed-reference assertions
- WHEN its authority context is revoked
- THEN cleanup retracts those assertions
- AND emits a cleanup receipt binding the revoked authority and retraction refs

### Requirement: Shared canonical content-ref parsing
r[molten.runtime_spine.canonical_content_refs.shape] Molten MUST validate BLAKE3 content refs through a shared parser that accepts only `blake3:<64 lowercase hex chars>` for canonical content refs unless another algorithm is explicitly modeled.

#### Scenario: Malformed content ref is rejected
- GIVEN a ref that is empty, truncated, non-hex, uppercase, path-like, or uses an unsupported algorithm
- WHEN a runtime, node-control, evidence, storage, protocol, or catalog boundary parses the ref
- THEN Molten rejects the ref before using it as identity evidence

#### Scenario: Canonical value computes a content ref
- GIVEN a canonical Preserves value
- WHEN Molten computes the value identity
- THEN the resulting content ref is the BLAKE3 hash of the canonical Preserves bytes formatted by the shared ref helper

### Requirement: Canonical refs are constructed by shared helpers
r[molten.runtime_spine.canonical_content_refs.helper_construction] Molten MUST construct canonical BLAKE3 content refs through shared content-ref helpers for raw bytes, computed BLAKE3 hash values, and validated lowercase hex inputs rather than duplicating `blake3:` string formatting at subsystem boundaries.

#### Scenario: Byte and hash helpers produce canonical refs
- GIVEN raw artifact, receipt, blob, or transport bytes
- WHEN Molten computes a canonical content ref for those bytes
- THEN the ref is formatted by the shared helper as `blake3:<64 lowercase hex chars>`
- AND callers do not hand-concatenate the scheme prefix.

#### Scenario: Hex helper validates before formatting
- GIVEN a lowercase 64-character BLAKE3 hex digest
- WHEN Molten reconstructs a canonical content ref from the digest
- THEN the shared helper validates the digest length and character set before returning the ref.

### Requirement: Filename readback validates refs
r[molten.runtime_spine.canonical_content_refs.filename_readback] Molten MUST convert ledger, chunk-store, ingress, and evidence filenames back into content refs only through validated hex/readback helpers and MUST fail closed for malformed names.

#### Scenario: Malformed filename does not synthesize a ref
- GIVEN a local materialized filename with a `blake3_` prefix but a malformed, uppercase, path-like, truncated, or overlong digest
- WHEN a store scans materialized content
- THEN Molten rejects or ignores the filename as malformed
- AND does not synthesize a plausible canonical ref from unchecked string concatenation.

#### Scenario: Valid filename remains identity-only
- GIVEN a valid materialized filename that converts to a canonical ref
- WHEN a protected operation requires local content
- THEN Molten still recomputes the ref from the stored bytes or canonical value before side effects.

### Requirement: Transitional aliases are scoped evidence only
r[molten.runtime_spine.canonical_content_refs.scoped_aliases] Molten MAY emit explicitly scoped alternate hash aliases such as Octet `b3:` evidence refs only for integrations that require them, but MUST NOT accept those aliases as canonical runtime content refs unless a future algorithm/model explicitly admits them.

#### Scenario: Octet alias is derived from canonical evidence
- GIVEN an Octet diagnostic artifact that records a `b3:` fingerprint alias
- WHEN Molten emits the alias
- THEN the alias is derived from a validated canonical hash helper path or equivalent checked bytes hashing
- AND the alias remains Octet evidence, not runtime content identity.

### Requirement: Subsystems avoid ad-hoc ref formatting
r[molten.runtime_spine.canonical_content_refs.no_ad_hoc_formatting] Molten subsystems SHOULD NOT hand-build canonical `blake3:` refs or strip/replace the canonical prefix outside the shared Preserves rail helper boundary.

#### Scenario: Ref construction cleanup preserves gate separation
- GIVEN a subsystem migrated from ad-hoc `blake3:` formatting to shared helper construction
- WHEN it parses, stores, or validates refs
- THEN parse failures and diagnostics come from the shared helper
- AND existing authority, policy, provenance, source-gate, retention, resource, transport, and replay gates remain separate from content-ref shape.

### Requirement: Cleanup validation evidence is recorded
r[molten.runtime_spine.canonical_content_refs.cleanup_tests] Molten MUST validate canonical content-ref cleanup with focused malformed-ref/readback tests and source gates before treating the cleanup as complete.

#### Scenario: Cleanup validation passes
- GIVEN content-ref helper cleanup across ledger, chunk-store, remote dataspace, Iroh exchange, Octet evidence, and related synthetic refs
- WHEN validation runs
- THEN focused content-ref tests, affected subsystem tests, clippy, full tests, and Octet gates pass or emit explicit denial evidence.

### Requirement: Content addressing is evidence, not trust
r[molten.runtime_spine.canonical_content_refs.not_trust] Molten MUST NOT treat a well-shaped content ref as authority, policy, provenance, source-gate, retention, resource, or transport trust.

#### Scenario: Plausible ref lacks authority
- GIVEN a request with a syntactically valid payload ref but no authority or policy evidence
- WHEN the request is admitted
- THEN Molten denies the request through the authority or policy gate despite the valid ref shape

#### Scenario: Transport ref remains evidence-only
- GIVEN a live transport receipt that binds a syntactically valid envelope ref
- WHEN downstream dispatch evaluates the request
- THEN dispatch still depends on node-control authority, resource, idempotency, provenance, and source-gate receipts

### Requirement: Materialized ref readback
r[molten.runtime_spine.canonical_content_refs.materialized_readback] Molten MUST distinguish ref shape validation from local materialization and MUST recompute refs from local bytes or canonical values when an operation claims the content is present locally.

#### Scenario: Missing materialized content denies
- GIVEN a well-shaped ref that is not present in the claimed local ledger, chunk store, ingress store, or runtime journal
- WHEN an operation requires local materialized content
- THEN Molten emits denial diagnostics or a denial receipt instead of accepting the ref string

#### Scenario: Tampered materialized content denies
- GIVEN local bytes stored under a claimed ref
- WHEN recomputing the canonical or domain-separated BLAKE3 ref yields a different ref
- THEN Molten rejects the content and records a stale or tampered-ref diagnostic before side effects

### Requirement: Node-control refs use canonical discipline
r[molten.runtime_spine.canonical_content_refs.node_control] Molten MUST parse node-control request refs, payload refs, ingress envelope refs, live transport receipt refs, and subreceipt refs with the shared content-ref discipline.

#### Scenario: Node-control rejects short fixture refs
- GIVEN a node-control request whose payload ref is `blake3:fixture`
- WHEN the request is parsed or admitted outside test-only fixture construction
- THEN Molten rejects the request as malformed before dispatch

#### Scenario: Live ingress binds materialized envelope identity
- GIVEN canonical live ingress bytes received from transport
- WHEN Molten stores the ingress envelope locally
- THEN the live receipt binds the envelope ref recomputed from canonical bytes and does not treat transport delivery as authority

### Requirement: Runtime values expose canonical refs
r[molten.runtime_spine.canonical_content_refs.runtime_values] Molten SHOULD expose canonical refs for runtime values, messages, assertions, observations, events, turn journals, and state snapshots wherever those records cross a runtime, replay, harness, evidence, or storage boundary.

#### Scenario: Turn journal refs are stable under replay
- GIVEN a deterministic runtime run and its replay under the same inputs
- WHEN turn journals and state snapshots are emitted
- THEN their canonical refs are identical between the original run and replay

#### Scenario: Runtime value ref avoids debug-format identity
- GIVEN two equal runtime values with the same canonical Preserves bytes
- WHEN their refs are computed
- THEN the refs match even if Rust debug formatting or allocation layout differs

### Requirement: Migration coverage for content-ref discipline
r[molten.runtime_spine.canonical_content_refs.negative_tests] Molten MUST test malformed refs, wrong-length refs, unsupported algorithms, valid-shaped missing content, and tampered local bytes for migrated boundaries.

#### Scenario: Negative ref matrix fails closed
- GIVEN a migrated boundary that accepts content refs
- WHEN tests supply malformed, missing, or tampered refs
- THEN the boundary fails closed and emits structured diagnostics without mutating protected state

r[molten.runtime_spine.canonical_content_refs.migration] Molten SHOULD migrate artifact registry, catalog, coordination, protocol session, service runtime, transcripts, provenance, redaction, secrets, and job DAG validators to the shared ref helper in bounded slices.

#### Scenario: Migrated module removes ad-hoc prefix checks
- GIVEN a module that previously accepted refs with ad-hoc `blake3:` prefix checks
- WHEN the module is migrated
- THEN parse failures and diagnostics come from the shared content-ref helper while the module preserves its separate policy and authority gates

### Requirement: Trellis-backed assertion visibility
r[molten.trellis_runtime.assertion_visibility] The system SHOULD provide Trellis-backed predicates for dataspace assertion ownership, duplicate assertion deduplication, visibility, and automatic retraction.

#### Scenario: Assertion visible iff live owner maintains it
r[molten.trellis_runtime.assertion_visibility.live_owner]
- GIVEN a bounded model of assertion owners, assertion handles, owner liveness, and retractions
- WHEN Molten evaluates whether a canonical assertion is visible
- THEN the predicate reports visible exactly when at least one live admitted owner still maintains that assertion

#### Scenario: Duplicate assertion retracts only after final owner
r[molten.trellis_runtime.assertion_visibility.dedup]
- GIVEN multiple live owners asserting the same canonical value
- WHEN one but not all owners retract or terminate
- THEN the predicate preserves observer-level visibility until the final live owner withdraws the assertion

### Requirement: Trellis-backed turn commit and rollback
r[molten.trellis_runtime.turn_commit_rollback] The system SHOULD provide Trellis-backed predicates for pending-action invisibility, atomic turn commit, and rollback when a turn fails or is denied.

#### Scenario: Failed turn leaves committed state unchanged
r[molten.trellis_runtime.turn_commit_rollback.failed]
- GIVEN a prior state summary and a bounded set of pending actions
- WHEN the turn outcome is failed or denied
- THEN the predicate reports that committed actor, vat, and dataspace state remain unchanged

#### Scenario: Successful turn applies pending actions atomically
r[molten.trellis_runtime.turn_commit_rollback.commit]
- GIVEN a prior state summary, admitted pending actions, and a successful turn outcome
- WHEN the turn commits
- THEN the predicate reports that all admitted pending actions become visible together as the next committed state

### Requirement: Bounded Preserves pattern predicate subset
r[molten.trellis_runtime.preserves_pattern_subset] The system SHOULD define a bounded Trellis-friendly Preserves pattern and value subset with deterministic matching and binding order for routing and policy-visible matching predicates.

#### Scenario: Pattern match is deterministic
r[molten.trellis_runtime.preserves_pattern_subset.deterministic]
- GIVEN a bounded Preserves pattern and bounded Preserves value model
- WHEN two nodes evaluate the match
- THEN both nodes produce the same success or failure result and the same ordered bindings

### Requirement: Trellis-backed Observe delivery
r[molten.trellis_runtime.observe_delivery] The system SHOULD provide Trellis-backed predicates for Observe delivery of matching current assertions, future assertions, and matching retraction propagation.

#### Scenario: New Observe receives matching current set
r[molten.trellis_runtime.observe_delivery.current]
- GIVEN a current dataspace assertion set and a new Observe subscription with a matching pattern
- WHEN the Observe predicate evaluates initial delivery
- THEN it identifies exactly the current visible assertions that match the pattern

#### Scenario: Retraction propagates to matching observers
r[molten.trellis_runtime.observe_delivery.retraction]
- GIVEN a visible assertion delivered to an observer through an Observe subscription
- WHEN the assertion is no longer visible
- THEN the predicate identifies the corresponding observer retraction that must be emitted

### Requirement: Trellis-backed promise state machine
r[molten.trellis_runtime.promise_state] The system SHOULD provide Trellis-backed predicates for promise/vow states including pending, resolved, broken, cancelled, timed out, and causal failure propagation.

#### Scenario: Promise has one terminal result
r[molten.trellis_runtime.promise_state.terminal]
- GIVEN a pending promise and a generated sequence of resolution, failure, cancellation, or timeout events
- WHEN the predicate admits a terminal transition
- THEN no later conflicting terminal result is admitted for the same promise

### Requirement: Trellis-backed promise pipelining
r[molten.trellis_runtime.promise_pipeline] The system SHOULD provide Trellis-backed predicates for bounded promise pipelining, including queue bounds, forwarding order, and cleanup after failure.

#### Scenario: Resolved pipeline forwards in order
r[molten.trellis_runtime.promise_pipeline.order]
- GIVEN a bounded queue of pipelined calls and a promise that resolves to a reference
- WHEN the pipeline is admitted for forwarding
- THEN queued calls are forwarded in original order subject to policy admission

#### Scenario: Broken pipeline fails queued calls
r[molten.trellis_runtime.promise_pipeline.broken]
- GIVEN queued pipelined calls and a promise that breaks
- WHEN the failure predicate evaluates the queue
- THEN all queued calls fail causally and no target side effects are admitted

### Requirement: Trellis-backed revocation cleanup
r[molten.trellis_runtime.revocation_cleanup] The system SHOULD provide Trellis-backed predicates for revoked references denying future use and cleaning dependent assertions, subscriptions, pending calls, and child references.

#### Scenario: Revoked proxy invalidates dependents
r[molten.trellis_runtime.revocation_cleanup.proxy]
- GIVEN a proxy reference with dependent assertions, Observe subscriptions, pending calls, and child references
- WHEN the proxy is revoked
- THEN the predicate identifies future-use denial and the dependent cleanup actions required by policy

### Requirement: Trellis-backed actormap transactions
r[molten.trellis_runtime.actormap_transaction] The system SHOULD provide Trellis-backed predicates for actormap delta commit/rollback, spawned object visibility, and removed object invalidation.

#### Scenario: Aborted actormap delta is invisible
r[molten.trellis_runtime.actormap_transaction.abort]
- GIVEN a prior actormap and a generated turn delta with spawned, updated, and removed objects
- WHEN the turn aborts
- THEN the predicate reports that the next committed actormap equals the prior committed actormap

#### Scenario: Removed object cannot be near-called after commit
r[molten.trellis_runtime.actormap_transaction.removed]
- GIVEN an object removed by a committed actormap delta
- WHEN a later turn attempts a near call to that object id
- THEN the predicate denies the near call because the object is no longer live in the actormap

### Requirement: Trellis-backed near/far reference admission
r[molten.trellis_runtime.near_far_refs] The system SHOULD provide Trellis-backed predicates admitting synchronous calls only for live same-vat near references and requiring asynchronous semantics for far references.

#### Scenario: Cross-vat synchronous call is denied
r[molten.trellis_runtime.near_far_refs.cross_vat]
- GIVEN a caller vat id and a target reference descriptor for a different vat or session
- WHEN the caller requests synchronous invocation
- THEN the predicate denies synchronous near-call admission and requires far-call semantics

### Requirement: Trellis-backed snapshot authority subset
r[molten.trellis_runtime.snapshot_authority] The system SHOULD provide Trellis-backed predicates ensuring object snapshot authority claims are subsets of authority already held or explicitly admitted by restore policy.

#### Scenario: Snapshot cannot mint authority
r[molten.trellis_runtime.snapshot_authority.no_mint]
- GIVEN a held-authority set and a snapshot portrait claiming authority outside that set without admitted restore grant
- WHEN the snapshot authority predicate evaluates the portrait
- THEN the predicate rejects the extra authority claim before snapshot admission or restore

### Requirement: Trellis-backed service dependency admission
r[molten.trellis_runtime.service_dependencies] The system SHOULD provide Trellis-backed predicates for service dependency startup, readiness, failure, force-run, restart, reverse dependency, and shutdown admission.

#### Scenario: Dependency readiness gates startup
r[molten.trellis_runtime.service_dependencies.ready_gate]
- GIVEN a service demand assertion and a dependency assertion requiring another service state to be ready
- WHEN the required dependency state is absent and the service is not force-run
- THEN the predicate denies startup readiness for the dependent service

### Requirement: Runtime predicate receipt naming
r[molten.trellis_runtime.predicate_receipts] Runtime applications of Trellis-backed predicates SHOULD emit receipt/evidence identifiers that name the predicate, input summary, decision, and related actor/session/reference state.

#### Scenario: Predicate decision is receipt-addressable
r[molten.trellis_runtime.predicate_receipts.addressable]
- GIVEN a runtime admission decision based on a Trellis-backed predicate
- WHEN the runtime emits evidence for the decision
- THEN the receipt or trace record identifies the predicate name, bounded input summary, decision, and affected runtime state references

### Requirement: Trellis runtime predicate integration tests
r[molten.trellis_runtime.integration_tests] The system SHOULD include integration tests showing Molten runtime admission calls Trellis-backed predicates for assertion visibility, turn commit/rollback, patterns, promises, and revocation.

#### Scenario: Runtime uses predicate before commit
r[molten.trellis_runtime.integration_tests.before_commit]
- GIVEN a runtime turn that would publish assertions and update object state
- WHEN the runtime reaches the admission boundary
- THEN tests show the relevant Trellis-backed predicate is consulted before the turn commits

### Requirement: Trellis runtime predicate property tests
r[molten.trellis_runtime.property_tests] The system SHOULD use Hegel property tests over bounded models for assertion owners, turn deltas, pattern matches, promise pipelines, revocation graphs, snapshots, and service dependencies.

#### Scenario: Generated failed turns preserve committed state
r[molten.trellis_runtime.property_tests.failed_turns]
- GIVEN generated prior state summaries and generated failed turn deltas
- WHEN the bounded model evaluates rollback
- THEN the committed state after the failed turn equals the prior committed state

### Requirement: Goblins/OCapN reference boundary
r[molten.runtime_spine.goblins_reference_boundary] The system MUST treat Spritely Goblins and OCapN/CapTP as non-normative design references for vat/object execution, object capabilities, promises, persistence, debugging, and distributed references, and MUST NOT claim Guile Goblins, Racket Goblins, OCapN, or CapTP compatibility in the first implementation.

#### Scenario: Documentation cites Goblins without compatibility claim
r[molten.runtime_spine.goblins_reference_boundary.no_compat]
- GIVEN Molten design material that cites Goblins or OCapN/CapTP
- WHEN the material describes an adopted runtime pattern
- THEN it states the Molten-specific Preserves envelope, policy, evidence, transport, storage, and execution boundaries rather than claiming implementation or wire compatibility

### Requirement: Vat model with near and far references
r[molten.runtime_spine.vat_model] The runtime MUST define vats as optional internal object territories hosted by SAM-style actors or services, and MUST distinguish near references, which may be called synchronously within the same vat turn, from far references, which cross vat, actor, process, machine, transport, persistence, or sandbox boundaries and must be called asynchronously.

#### Scenario: Near object call is synchronous inside a turn
r[molten.runtime_spine.vat_model.near_call]
- GIVEN two objects hosted by the same vat during one actor turn
- WHEN one object calls the other through a near reference
- THEN the call executes synchronously within the same transactional turn

#### Scenario: Far object call returns promise
r[molten.runtime_spine.vat_model.far_call]
- GIVEN an object reference to another vat or remote peer
- WHEN an actor invokes that reference
- THEN the runtime treats the call as asynchronous and returns a promise or vow rather than blocking for a synchronous result

### Requirement: Transactional actormap
r[molten.runtime_spine.transactional_actormap] Each vat MUST maintain a transactional actormap for object behavior/state so object state changes, object spawn/remove operations, and pending outbound actions commit only if the enclosing turn succeeds and admission passes.

#### Scenario: Actormap delta commits on successful turn
r[molten.runtime_spine.transactional_actormap.commit]
- GIVEN a turn that updates an object state, spawns a new object, and queues an outbound message
- WHEN the turn completes and admission passes
- THEN the actormap delta and queued outbound message become visible as committed runtime state

#### Scenario: Actormap delta rolls back on failed turn
r[molten.runtime_spine.transactional_actormap.rollback]
- GIVEN a turn that updates local object state and then raises an uncaught error
- WHEN the turn aborts
- THEN the object state and queued outbound actions remain as they were before the turn began

### Requirement: Object references are capabilities
r[molten.runtime_spine.object_capability_refs] Object references MUST be treated as capability-bearing authority, and authority transfer MUST occur by explicit reference passing, object creation, resolver output, admitted snapshot restore, or other policy-admitted endowment.

#### Scenario: Missing reference denies use
r[molten.runtime_spine.object_capability_refs.missing_ref]
- GIVEN an object that has not been given a reference to a protected object or service
- WHEN it attempts to use that protected object or service
- THEN the runtime provides no ambient path to that authority

#### Scenario: Reference crossing boundary has Preserves descriptor
r[molten.runtime_spine.object_capability_refs.preserves_descriptor]
- GIVEN an object reference passed through a dataspace assertion, message, protocol payload, or transport envelope
- WHEN the reference crosses the runtime boundary
- THEN the reference is represented by a canonical Preserves descriptor with scope, attenuation, and evidence sufficient for admission

### Requirement: No ambient object authority
r[molten.runtime_spine.no_ambient_object_authority] Newly created objects MUST start without ambient filesystem, network, clock, process, dataspace, store, blob, consensus, choreography, or host-resource authority unless those authorities are explicitly endowed by capability-bearing references and admitted policy.

#### Scenario: New object cannot access clock without capability
r[molten.runtime_spine.no_ambient_object_authority.clock]
- GIVEN a newly spawned object without a clock capability
- WHEN it attempts to observe wall-clock time
- THEN the runtime denies the operation or requires an explicit admitted clock reference

### Requirement: Promise and vow results for far calls
r[molten.runtime_spine.promise_vows] Far-object calls MUST return promise or vow results that represent pending success, failure, cancellation, timeout, or causal failure propagation without blocking the caller's current turn.

#### Scenario: Far call resolves successfully
r[molten.runtime_spine.promise_vows.resolve]
- GIVEN a far-object call that completes successfully on the target vat
- WHEN the result is delivered to the caller
- THEN the corresponding promise resolves with the canonical result value or reference descriptor

#### Scenario: Far call failure propagates to promise
r[molten.runtime_spine.promise_vows.failure]
- GIVEN a far-object call whose target turn aborts or whose transport/session fails
- WHEN the caller observes the promise
- THEN the promise is broken with causal failure information rather than silently succeeding

### Requirement: Bounded promise pipelining
r[molten.runtime_spine.promise_pipelining] The runtime MUST support bounded promise pipelining, allowing calls to be queued against unresolved future references while enforcing queue length, lifetime, payload size, authority scope, and policy visibility limits.

#### Scenario: Pipelined call forwards after promise resolves
r[molten.runtime_spine.promise_pipelining.forward]
- GIVEN a promise that is expected to resolve to an object reference
- WHEN an actor queues a pipelined call against that promise and the promise resolves successfully
- THEN the queued call is forwarded in order to the resolved reference subject to policy admission

#### Scenario: Broken promise breaks pipelined calls
r[molten.runtime_spine.promise_pipelining.break]
- GIVEN pipelined calls queued against a promise
- WHEN the promise breaks before resolving to a reference
- THEN the queued calls fail with causal failure propagation and do not perform target side effects

#### Scenario: Pipeline bound denies excess queue
r[molten.runtime_spine.promise_pipelining.bounds]
- GIVEN a promise pipeline that has reached its configured queue or lifetime bound
- WHEN another pipelined call is requested
- THEN the runtime rejects or delays the request before unbounded memory or authority growth occurs

### Requirement: Revocable and attenuated proxies
r[molten.runtime_spine.revocable_proxies] The runtime MUST support proxy references that can narrow authority, enforce policy, log use, transform payloads, or revoke access, and revocation MUST clean up dependent assertions, subscriptions, pending calls, and live references where applicable.

#### Scenario: Revocation invalidates proxy
r[molten.runtime_spine.revocable_proxies.revoke]
- GIVEN a live proxy reference used to assert a subscription and queue far calls
- WHEN the proxy is revoked
- THEN further use is denied and dependent subscriptions or pending calls are retracted, cancelled, or failed according to the proxy policy

#### Scenario: Attenuated proxy narrows authority
r[molten.runtime_spine.revocable_proxies.attenuate]
- GIVEN a proxy that allows only a subset of methods or assertion patterns
- WHEN a caller attempts a disallowed operation through the proxy
- THEN the runtime denies the operation before it reaches the underlying reference

### Requirement: Rights amplification with sealers or branded tokens
r[molten.runtime_spine.rights_amplification] The runtime MUST support a sealer/unsealer or branded-token pattern for rights amplification, allowing authorized objects to prove private relationships or recover sealed authority without relying on ambient identity checks.

#### Scenario: Authorized unsealer reveals sealed authority
r[molten.runtime_spine.rights_amplification.unseal]
- GIVEN a sealed value created by a private sealer and an object holding the corresponding unsealer
- WHEN the object unseals the value
- THEN it recovers only the sealed authority or data and can record the brand/provenance as evidence

#### Scenario: Wrong unsealer cannot amplify rights
r[molten.runtime_spine.rights_amplification.wrong_unsealer]
- GIVEN a sealed value and an unrelated unsealer
- WHEN an object attempts to unseal the value
- THEN the runtime rejects the operation and grants no additional authority

### Requirement: Distributed reference lifetimes
r[molten.runtime_spine.distributed_ref_lifetimes] Far references MUST have explicit session, handoff, bootstrap, and lifetime or garbage-tracking rules so remote resources can be released and stale references can be denied.

#### Scenario: Session-scoped reference expires on disconnect
r[molten.runtime_spine.distributed_ref_lifetimes.disconnect]
- GIVEN a far reference whose descriptor is scoped to a transport session
- WHEN the session disconnects without admitted handoff or persistence
- THEN the reference becomes invalid and dependent pending calls fail or are retracted

#### Scenario: Handoff creates new admitted scope
r[molten.runtime_spine.distributed_ref_lifetimes.handoff]
- GIVEN a far reference that must outlive its current session
- WHEN an admitted handoff or bootstrap protocol grants a replacement descriptor
- THEN the new descriptor carries its own scope, attenuation, expiry, and evidence references

### Requirement: Safe object serialization
r[molten.runtime_spine.safe_object_serialization] Vat and object serialization MUST preserve object state and authority graphs, and objects that provide self-portraits or snapshot recipes MUST be able to describe only state and authority they already hold.

#### Scenario: Snapshot preserves authority graph
r[molten.runtime_spine.safe_object_serialization.authority_graph]
- GIVEN a vat containing objects with references to each other and to external resources
- WHEN the vat is snapshotted
- THEN the snapshot records object state and reference graph descriptors without introducing new authority

#### Scenario: Malicious portrait cannot claim new authority
r[molten.runtime_spine.safe_object_serialization.no_escalation]
- GIVEN an object snapshot portrait that claims a reference the object did not hold
- WHEN the serializer validates the portrait
- THEN the claimed reference is rejected or excluded before snapshot admission

### Requirement: Object upgrade recipes
r[molten.runtime_spine.object_upgrade] Restored object snapshots MUST use explicit behavior/schema versions and admitted upgrade recipes when object representations change across Molten versions.

#### Scenario: Snapshot restore applies admitted upgrade
r[molten.runtime_spine.object_upgrade.apply]
- GIVEN a snapshot containing an older object schema version and an admitted upgrade recipe
- WHEN the vat is restored
- THEN the runtime applies the recipe deterministically and records upgrade evidence

#### Scenario: Missing upgrade rejects incompatible snapshot
r[molten.runtime_spine.object_upgrade.missing]
- GIVEN a snapshot with an unsupported object schema version and no admitted upgrade recipe
- WHEN the runtime attempts to restore it
- THEN restore is rejected before the object becomes live

### Requirement: Time-travel distributed debugging hooks
r[molten.runtime_spine.time_travel_debugging] The runtime MUST expose trace, snapshot, and replay hooks sufficient to reconstruct object state at prior turns, inspect causality, and correlate object events with dataspace, choreography, consensus, policy, and receipt events subject to debugging authority.

#### Scenario: Debugger reconstructs prior turn state
r[molten.runtime_spine.time_travel_debugging.reconstruct]
- GIVEN admitted snapshots and turn trace records for a vat
- WHEN an authorized debugger selects a prior turn id
- THEN the runtime can reconstruct or present the object state and pending causal events for that point in execution

#### Scenario: Debugging respects authority
r[molten.runtime_spine.time_travel_debugging.authority]
- GIVEN a trace containing secret object state or references
- WHEN a caller lacks the required debugging capability
- THEN the inspection surface redacts or denies access to protected state and references

### Requirement: Authority graph inspection
r[molten.runtime_spine.authority_graph_inspection] The runtime SHOULD expose an authority-aware inspection surface for object reference graphs, proxy chains, attenuations, revocations, and snapshot descriptors.

#### Scenario: Operator inspects attenuated reference graph
r[molten.runtime_spine.authority_graph_inspection.inspect]
- GIVEN an authorized operator inspecting a vat
- WHEN the operator requests the reference graph
- THEN the runtime reports objects, references, proxy boundaries, attenuations, revocation state, and evidence references subject to redaction policy

### Requirement: Portable encrypted storage
r[molten.runtime_spine.portable_encrypted_storage] Content, snapshots, large payloads, and document artifacts SHOULD use provider-independent storage principles: content addressing, encryption before storage, chunking, mutable containers built from immutable chunks, and read/write authority represented as explicit capabilities.

#### Scenario: Blob provider cannot read encrypted content
r[molten.runtime_spine.portable_encrypted_storage.encrypted]
- GIVEN a snapshot or large payload stored through a blob adapter
- WHEN the blob provider stores the chunks
- THEN the provider sees only encrypted chunks and metadata that does not include plaintext without a read capability

#### Scenario: Content ref is network independent
r[molten.runtime_spine.portable_encrypted_storage.network_independent]
- GIVEN an immutable encrypted content artifact addressed by hash
- WHEN the artifact is fetched from Iroh blobs, local store, or another admitted provider
- THEN the same content reference and integrity checks apply regardless of provider location

### Requirement: Vat integration tests
r[molten.runtime_spine.vat_integration_tests] The system MUST include integration tests for near synchronous calls, far asynchronous calls, actormap rollback, pending action commit, reference passing, proxy revocation, and promise failure propagation.

#### Scenario: Far call and revocation integration test
r[molten.runtime_spine.vat_integration_tests.far_revoke]
- GIVEN two vats connected through a local far-reference adapter
- WHEN one vat calls through a proxy and the proxy is later revoked
- THEN the first call follows normal promise resolution rules and later calls fail due to revocation

### Requirement: Snapshot integration tests
r[molten.runtime_spine.snapshot_integration_tests] The system MUST include integration tests for object snapshot/restore, authority preservation, denied authority escalation, and version upgrade recipes.

#### Scenario: Restored vat preserves allowed references only
r[molten.runtime_spine.snapshot_integration_tests.restore]
- GIVEN a vat snapshot with an object reference graph and an attempted unauthorized extra reference
- WHEN the snapshot is restored
- THEN admitted references are restored and unauthorized authority is denied or excluded

### Requirement: Promise pipeline property tests
r[molten.runtime_spine.promise_property_tests] The system MUST use Hegel property-based tests for bounded promise pipelines, resolution and failure ordering, queue cleanup, and causal failure propagation within supported bounds.

#### Scenario: Generated pipeline preserves order or fails causally
r[molten.runtime_spine.promise_property_tests.pipeline_order]
- GIVEN a generated bounded promise pipeline and generated resolution or failure event
- WHEN the model processes the pipeline
- THEN forwarded calls preserve queue order on success and all queued calls fail causally on promise break

### Requirement: Actormap property tests
r[molten.runtime_spine.actormap_property_tests] The system MUST use Hegel property-based tests for generated actormap turn deltas to verify commit and rollback invariants.

#### Scenario: Generated failed turn preserves prior actormap
r[molten.runtime_spine.actormap_property_tests.rollback]
- GIVEN a generated actormap state and generated turn delta that aborts
- WHEN the model rolls the turn back
- THEN the resulting actormap equals the prior committed state

### Requirement: Remote dataspace envelopes
r[molten.iroh_sam_dataspace.envelope_dto] The system MUST represent remote SAM dataspace actions as canonical `remote-dataspace-envelope-v1` Preserves records carrying sender peer, sender actor, target peer, topic, operation, payload, content refs, capability refs, and evidence refs.

#### Scenario: Assertion envelope is canonical
r[molten.iroh_sam_dataspace.envelope_dto.assertion]
- GIVEN peer A actor `producer` wants to assert `<service-ready "db">` for peer B
- WHEN the remote dataspace adapter builds the envelope
- THEN the envelope operation is `assert`, its payload is the canonical Preserves assertion value, and its envelope ref is the Blake3 hash of the canonical Preserves bytes

### Requirement: Transport receipts for Iroh dataspace traffic
r[molten.iroh_sam_dataspace.transport_receipt_dto] The system MUST emit canonical `remote-dataspace-transport-receipt-v1` records for remote dataspace publish, deliver, and deny decisions.

#### Scenario: Publish receipt binds envelope and topic
r[molten.iroh_sam_dataspace.transport_receipt_dto.publish]
- GIVEN a valid remote dataspace envelope for topic `services`
- WHEN the Iroh transport adapter publishes the envelope
- THEN the transport receipt binds the envelope ref, transport name, source peer, target peer, topic, content refs, diagnostics, and checks

### Requirement: Transport identity is not authority
r[molten.iroh_sam_dataspace.transport_not_authority] The system MUST NOT treat Iroh endpoint identity, gossip topic membership, or blob possession as authority to act as an actor or mutate local dataspace state.

#### Scenario: Transport receipt is not enough for delivery admission
r[molten.iroh_sam_dataspace.transport_not_authority.not_enough]
- GIVEN a pass transport receipt for an envelope
- WHEN capability, policy, resource, or peer-bootstrap evidence is absent
- THEN the envelope MUST NOT be accepted as pass evidence for local side effects

### Requirement: Local Iroh-shaped deterministic adapter
r[molten.iroh_sam_dataspace.local_gossip_publish] The system MUST provide a deterministic local Iroh-shaped adapter for tests and repros that stores canonical envelope bytes under a local transport root and emits the same receipt shape as the live adapter.

#### Scenario: Local publish and deliver preserves envelope identity
r[molten.iroh_sam_dataspace.local_gossip_publish.roundtrip]
- GIVEN a canonical remote dataspace envelope
- WHEN it is published and delivered through the deterministic local Iroh-shaped adapter
- THEN the delivered envelope ref matches the published ref and the delivery receipt binds the same topic and peers

### Requirement: Content refs validate before delivery
r[molten.iroh_sam_dataspace.content_ref_validation] The system MUST validate declared remote dataspace content refs before delivering an envelope to local actors.

#### Scenario: Tampered content is rejected
r[molten.iroh_sam_dataspace.content_ref_validation.tampered]
- GIVEN a remote dataspace envelope that declares a blob content ref
- WHEN the local bytes for that content ref hash to a different value
- THEN delivery is denied before any actor observes the payload

### Requirement: Delivered envelopes apply through SAM turn semantics
r[molten.iroh_sam_dataspace.apply_assert_retract] Delivered remote assertion and retraction envelopes MUST apply through the local runtime turn boundary rather than mutating dataspace state directly.

#### Scenario: Remote assertion notifies local observer
r[molten.iroh_sam_dataspace.apply_assert_retract.observe]
- GIVEN peer B has a local observer for `<service-ready "db">`
- WHEN peer B delivers an admitted remote `assert` envelope from peer A actor `producer` with that payload
- THEN peer B records a normal assertion commit event owned by a remote actor identity and a normal assertion observed event for the local observer

### Requirement: Message and observe envelopes use the same runtime boundary
r[molten.iroh_sam_dataspace.apply_message_observe] Delivered remote message and observe envelopes MUST route through local message delivery and observer registration semantics.

#### Scenario: Remote observe registers an observer
r[molten.iroh_sam_dataspace.apply_message_observe.observe]
- GIVEN a delivered remote `observe` envelope with an exact Preserves pattern
- WHEN the envelope is applied locally after admission
- THEN the observer registration is represented as a normal runtime observe event under a remote actor identity

### Requirement: Recorded transport log for replay
r[molten.iroh_sam_dataspace.recorded_delivery_log] Evidence-bearing remote dataspace runs MUST either record the canonical transport delivery log for replay or be marked non-replayable and excluded from deterministic gates.

#### Scenario: Recorded replay does not consult live network
r[molten.iroh_sam_dataspace.recorded_delivery_log.replay]
- GIVEN a remote dataspace run with recorded envelope bytes, content refs, transport receipts, and admission receipts
- WHEN the run is replayed
- THEN replay uses the recorded transport log rather than live Iroh timing or peer availability

### Requirement: Live Iroh behind the same boundary
r[molten.iroh_sam_dataspace.live_iroh_gossip] Live `iroh-gossip` integration MUST use the same envelope, content-ref, admission, receipt, and replay boundaries as the deterministic local adapter.

#### Scenario: Live and local adapters share receipt shape
r[molten.iroh_sam_dataspace.live_iroh_gossip.same_shape]
- GIVEN the local adapter and live Iroh adapter publish equivalent envelopes
- WHEN their receipts are inspected
- THEN both receipts use `remote-dataspace-transport-receipt-v1` and differ only in transport/profile-specific refs allowed by policy

### Requirement: Demand startup is admitted before side effects
r[molten.sam_service_demand_runtime.spec.admitted_demand_start] A service MUST start from a canonical demand assertion only after dependency readiness and explicit authority, policy, resource, effect-handle, and source-gate evidence pass.

#### Scenario: Demand starts dependency after gates pass
- GIVEN a `service-demand-v1` assertion for service `svc:frontend`
- AND a canonical manifest for `svc:frontend` requiring `svc:backend`
- AND `svc:backend` has a ready status assertion
- AND startup authority, policy, resource, effect-handle, and strict source-gate evidence pass
- WHEN the service demand runtime evaluates demand
- THEN it commits a service lifecycle receipt with decision `pass`
- AND it publishes service-owned readiness/status assertions for `svc:frontend`

#### Scenario: Missing source gate denies before actor execution
- GIVEN a valid demand assertion and manifest
- AND all dependencies are ready
- BUT strict source-gate evidence is missing or denied
- WHEN the service demand runtime evaluates demand
- THEN startup denies before actor execution
- AND no readiness assertion is committed

### Requirement: Dependency readiness is deterministic and bounded
r[molten.sam_service_demand_runtime.spec.dependency_resolution] Service dependency readiness MUST be resolved from canonical service status/readiness refs within bounded graph limits, and unmet, stale, cyclic, or missing dependencies MUST produce deterministic wait or denial receipts.

#### Scenario: Unmet dependency waits
- GIVEN a demand assertion for a service whose required dependency has no ready status assertion
- WHEN demand evaluation runs
- THEN the runtime emits a dependency-wait lifecycle receipt
- AND it performs no actor start side effects

#### Scenario: Dependency cycle denies
- GIVEN service manifests whose `requires` relations form a cycle outside supported bounds
- WHEN dependency resolution runs
- THEN the runtime emits deterministic denial diagnostics
- AND no service in the cycle is started by that demand evaluation

### Requirement: Service-owned assertions are replay-bound
r[molten.sam_service_demand_runtime.spec.owned_assertion_replay] Readiness, degraded, failure, and stopped assertions emitted by service startup MUST be owned by the service and bound into replay identity with demand, dependency, authority, resource, scheduler, and effect-log refs.

#### Scenario: Readiness owner is bound
- GIVEN an admitted service startup emits a readiness assertion
- WHEN the lifecycle receipt is generated
- THEN the receipt binds the service id, manifest ref, demand ref, authority/resource/effect refs, and readiness assertion ref
- AND later cleanup can identify the assertion as service-owned

#### Scenario: Replay detects changed dependency status
- GIVEN a recorded service startup replay identity
- AND the dependency readiness ref changes before replay
- WHEN replay validates the service lifecycle
- THEN replay fails at the dependency decision
- AND reports deterministic first-divergence diagnostics

### Requirement: Service records are canonical evidence
r[molten.sam_service_records_ledger.spec.canonical_records] Service manifests, demands, statuses, supervisors, restart policies, lifecycle receipts, and cleanup receipts MUST be represented as canonical Preserves records with stable Blake3 refs before they are used as runtime evidence.

#### Scenario: Manifest ref is stable
- GIVEN two byte-identical `service-manifest-v1` records with the same authority, target, dependencies, provided assertions, policy, resource, and effect refs
- WHEN Molten canonicalizes each record
- THEN both records produce the same service manifest ref
- AND the ref can be used by later service lifecycle receipts

#### Scenario: Malformed record denies
- GIVEN a service record with an unknown schema tag or missing explicit owner authority
- WHEN Molten parses the record for service admission
- THEN parsing denies with deterministic diagnostics
- AND the record cannot satisfy service pass evidence

### Requirement: Service manifests carry explicit authority and resource boundaries
r[molten.sam_service_records_ledger.spec.explicit_boundaries] A `service-manifest-v1` MUST bind explicit owner authority, policy, resource, and effect profile refs; a service name alone MUST NOT grant startup or cleanup authority.

#### Scenario: Name-only service cannot start
- GIVEN a service manifest containing only a human-readable service id and target actor name
- WHEN the service runtime evaluates the manifest
- THEN the manifest is denied before demand startup
- AND no readiness or status assertion is committed

#### Scenario: Boundary refs are preserved
- GIVEN a service manifest with explicit authority, policy, resource, and effect profile refs
- WHEN Molten renders catalog or MCP summaries
- THEN the summaries include safe refs or redacted markers
- AND the underlying canonical record remains the normative evidence

### Requirement: Service artifacts are visible without leaking secrets
r[molten.sam_service_records_ledger.spec.catalog_redaction] Service manifests, status records, lifecycle receipts, and cleanup receipts MUST be classified in ledger/catalog/MCP views, and rendered summaries MUST redact hidden refs and secret/confidential markers by default.

#### Scenario: Service status is summarized safely
- GIVEN a `service-status-v1` with readiness refs and a hidden secret-bearing diagnostic payload
- WHEN the catalog renders the service status
- THEN the summary shows service id, state, dependency ids, and receipt refs
- AND the secret-bearing payload is replaced by a redaction marker

#### Scenario: Text summary is not pass evidence
- GIVEN a rendered service summary that says a service is ready
- WHEN a gate evaluates service readiness evidence
- THEN the summary alone is rejected
- AND the gate requires the canonical status or lifecycle receipt refs

### Requirement: Supervision is logical and receipt-backed
r[molten.sam_service_supervision_cleanup.spec.logical_supervision] Service links, monitors, failure propagation, and restart decisions MUST be represented as canonical logical records and receipts independent from OS process parentage.

#### Scenario: Failure notifies monitors deterministically
- GIVEN a running service with two monitor records
- WHEN the service commits a failure transition
- THEN Molten emits monitor notification refs in deterministic order
- AND the failure lifecycle receipt binds the monitor refs and notification refs

#### Scenario: OS parentage is not supervision evidence
- GIVEN an OS process tree or ambient parent pid without canonical service link records
- WHEN Molten evaluates service supervision evidence
- THEN the OS parentage data is rejected as pass evidence
- AND no restart or monitor decision is admitted from it

### Requirement: Restart policy is bounded and replayable
r[molten.sam_service_supervision_cleanup.spec.bounded_restart] Restart decisions MUST be bounded by explicit restart policy, authority state, logical resource budgets, and recorded lifecycle refs; unbounded restart loops MUST deny.

#### Scenario: Restart attempt passes within bounds
- GIVEN a failed service with a restart policy that allows another attempt
- AND authority/resource evidence remains valid
- WHEN restart evaluation runs
- THEN Molten emits a restart decision receipt with decision `pass`
- AND schedules startup through the demand runtime path

#### Scenario: Restart budget exhausted denies
- GIVEN a failed service whose restart attempts exceed the policy window
- WHEN restart evaluation runs
- THEN Molten emits a restart denial receipt
- AND publishes a final failed or stopped status instead of restarting indefinitely

### Requirement: Cleanup retracts only proven service-owned state
r[molten.sam_service_supervision_cleanup.spec.owned_cleanup] Service stop, failure, shutdown, or authority revocation MUST retract service-owned assertions, observers, live refs, exposed refs, and pending effect intents, and MUST NOT delete state whose ownership cannot be proven.

#### Scenario: Revocation cleans owned readiness
- GIVEN a running service with owned readiness and exposed-reference assertions
- AND its owner authority is revoked
- WHEN cleanup runs
- THEN Molten retracts the owned assertions
- AND emits a cleanup receipt binding the revoked authority and retraction refs

#### Scenario: Foreign state is not deleted
- GIVEN cleanup input that includes an assertion owned by another service
- WHEN cleanup validates ownership
- THEN cleanup denies deletion of the foreign assertion
- AND records deterministic diagnostics in the cleanup receipt

### Requirement: Cleanup evidence is replay and retention input
r[molten.sam_service_supervision_cleanup.spec.cleanup_replay_retention] Cleanup receipts MUST bind prior lifecycle, ownership, revocation, resource, and retraction refs so replay can detect cleanup divergence and retention/GC can consume cleanup evidence without bypassing retention policy.

#### Scenario: Replay detects missing retraction
- GIVEN a recorded cleanup receipt with three owned assertion retractions
- WHEN replay observes only two matching retractions
- THEN replay fails at cleanup verification
- AND reports the missing retraction ref

#### Scenario: Retention still gates deletion
- GIVEN a cleanup receipt proving service-owned assertion retraction
- WHEN retention/GC evaluates physical deletion eligibility
- THEN the cleanup receipt is treated as input evidence
- AND retention policy still decides whether deletion is admitted

### Requirement: Executors use canonical hostcall envelopes
r[molten.runtime.executor_hostcall_boundary.envelopes] Non-native executors MUST interact with the runtime only through canonical Preserves actor input, hostcall request/decision, and actor output envelopes.

#### Scenario: Steel actor requests a send hostcall
- GIVEN a Steel actor with valid executor preflight evidence
- WHEN it requests a send operation
- THEN the runtime records a canonical hostcall request
- AND admission binds the decision to policy, capability, budget, actor, and turn refs

#### Scenario: Reviewed Steel preflight binds source, callable, and hostcalls
- GIVEN a Steel actor with a reviewed source/callable fixture
- WHEN executor preflight evidence is emitted
- THEN it includes a Steel review receipt binding the source ref, callable name, and allowed hostcalls
- AND replay/validation rejects stale, missing, or tampered Steel review receipts

#### Scenario: Steel undeclared hostcall is rejected before effects
- GIVEN a Steel actor whose reviewed fixture allows only a subset of hostcalls
- WHEN a suite step requests an undeclared hostcall
- THEN execution fails closed before side effects occur

#### Scenario: Wasm preflight binds module, imports, WIT, and hostcalls
- GIVEN a Wasm actor with an explicit module/WIT/allowed-hostcall fixture
- WHEN executor preflight evidence is emitted
- THEN it includes a Wasm inspection receipt binding the module ref, inspected imports, WIT ref, and allowed hostcalls
- AND invalid modules, stale receipts, unlisted imports, or ambient/WASI imports are rejected before side effects occur

#### Scenario: Wasm undeclared hostcall is rejected before effects
- GIVEN a Wasm actor whose preflight allows only a subset of hostcalls
- WHEN a suite step requests an undeclared hostcall
- THEN execution fails closed before side effects occur

#### Scenario: Reviewed Wasm hostcall actor executes under Wasmtime
- GIVEN a reviewed Wasm actor with valid module/WIT/allowed-hostcall preflight evidence
- WHEN an admitted hostcall step runs
- THEN the harness instantiates the core module with Wasmtime without WASI
- AND only `molten:hostcall/*` imports declared by preflight are linked
- AND the actor must export the operation entrypoint used for that hostcall
- AND execution is bounded by deterministic fuel and memory limits
- AND a canonical Wasm execution receipt is recorded before runtime state changes

#### Scenario: Ambient IO attempt is rejected
- GIVEN a non-native executor attempts filesystem, network, clock, random, or process access outside declared hostcalls
- WHEN execution runs
- THEN the runtime fails closed and records an executor-boundary diagnostic

### Requirement: Executor preflight is mandatory
r[molten.runtime.executor_hostcall_boundary.shell_admission] Steel, Wasm, adapter-backed, and remote-proxy actor kinds MUST remain fail-closed until executor preflight receipts validate.

#### Scenario: Unsupported executor kind remains blocked
- GIVEN an actor registry containing a Wasm actor without Wasm preflight evidence
- WHEN a suite runs
- THEN execution is rejected before side effects occur

#### Scenario: Stale preflight receipt is rejected
- GIVEN an actor module changed after preflight
- WHEN execution runs with the stale preflight receipt
- THEN execution fails closed before the actor can emit hostcalls

### Requirement: Replay validates hostcalls
r[molten.runtime.executor_hostcall_boundary.conformance] Replay MUST compare hostcall requests, decisions, and outputs for non-native actors exactly.

#### Scenario: Cross-kind conformance profile binds identical hostcalls
- GIVEN native, reviewed Steel, and reviewed Wasm actors that request the same hostcall operations over identical Preserves inputs
- WHEN executor preflight evidence is emitted
- THEN each actor binds the same executor conformance suite ref for the shared hostcall profile
- AND deterministic runs over the same actor id and inputs produce the same final runtime state across actor kinds

#### Scenario: Hostcall replay divergence is reported
- GIVEN a report whose hostcall decision was tampered
- WHEN replay runs
- THEN replay emits a hostcall-decision divergence diagnostic

### Requirement: Replay fixture identity binds deterministic inputs
r[molten.determinism.replay_fixture.identity] The deterministic replay fixture MUST define a canonical `deterministic-run-identity-v1` record that binds artifact refs, dependency-closure refs, initial-state refs, schema refs, policy refs, capability and revocation refs, handler-profile refs, seed or effect-log refs, runtime/tool version refs, and any scenario label that affects execution.

#### Scenario: Changed identity input is rejected
- GIVEN a recorded deterministic fixture identity
- WHEN replay is requested with a different artifact, profile, policy, seed, effect-log, initial-state, or version ref that affects execution
- THEN replay verification denies before accepting matching output evidence
- AND the denial identifies the changed identity boundary

### Requirement: Fixture record binds journals and effects
r[molten.determinism.replay_fixture.record] The deterministic replay fixture MUST emit a canonical `deterministic-fixture-record-v1` that binds the run identity, ordered turn journal refs, ordered effect-log refs, output refs, and final state ref needed to replay the run without ambient observations.

#### Scenario: Record contains enough evidence to replay
- GIVEN a bounded local deterministic fixture run
- WHEN the fixture record is produced
- THEN it contains or references the identity, turn journals, effect request/response pairs, outputs, receipts, and final state needed for verification

#### Scenario: Rendered output is not the replay oracle
- GIVEN a fixture record with human-readable rendering
- WHEN replay verification runs
- THEN verification compares canonical Preserves refs rather than trusting rendered text

### Requirement: Replay verifier compares semantic boundaries in order
r[molten.determinism.replay_fixture.verify] Replay fixture verification MUST emit `deterministic-replay-verify-v1` and MUST compare scheduler selection, input refs, effect request refs, effect response refs, policy-decision refs, committed action refs, receipt refs, output refs, and after-state refs in deterministic turn order.

#### Scenario: Matching replay passes
- GIVEN a recorded fixture and the same run identity inputs
- WHEN replay verification processes every recorded turn and effect response
- THEN the verify receipt passes and binds matching output and final-state refs

#### Scenario: Replay stops at the first mismatched boundary
- GIVEN a recorded fixture with a tampered turn, effect, receipt, output, or after-state ref
- WHEN replay verification reaches the first mismatch
- THEN verification stops before processing downstream differences
- AND emits a deny receipt that points to the first mismatched boundary

### Requirement: First divergence evidence is canonical and safe
r[molten.determinism.replay_fixture.first_divergence] Replay fixture verification MUST emit `deterministic-first-divergence-v1` evidence for the first mismatch, including divergence kind, turn id, actor/session/vat id when available, log position, handler-profile ref, expected canonical ref, actual canonical ref when safe, and redacted diagnostics for secret or capability-bearing boundaries.

#### Scenario: Effect response divergence is reported
- GIVEN a recorded fixture whose effect response is changed before verification
- WHEN replay compares the recorded and replayed response refs
- THEN verification reports an effect-response divergence with expected and actual refs when safe

#### Scenario: Sensitive divergence is redacted
- GIVEN a mismatch involving a secret or capability-bearing value
- WHEN first-divergence evidence is rendered or exported without reveal authority
- THEN diagnostics include safe commitments or redaction markers rather than plaintext secret or capability material

### Requirement: Replay profile denies live external effects
r[molten.determinism.replay_fixture.no_live_effects] Replay fixture verification MUST inject recorded effect responses and MUST deny live external clock, random, filesystem, network, environment, process, and storage observations that are not represented by the fixture effect log.

#### Scenario: Missing recorded effect response fails closed
- GIVEN replay execution reaches an effect request with no matching recorded request/response pair
- WHEN the replay handler profile handles the request
- THEN it denies the request before consulting the live external source
- AND records replay-denial evidence

### Requirement: Replay fixture CLI is evidence-oriented
r[molten.determinism.replay_fixture.cli] Molten SHOULD expose `molten test replay-fixture` commands for recording, verifying, tampering for negative tests, and showing canonical replay evidence without granting authority or bypassing normal gates.

#### Scenario: Fixture verify writes a receipt
- GIVEN a recorded replay fixture on disk
- WHEN an operator runs fixture verification with a receipt output path
- THEN Molten writes a canonical replay verification receipt
- AND the receipt is evidence only, not authority, policy admission, transport trust, provenance trust, source-gate trust, or release trust

### Requirement: Replay fixture tests cover pass and denial paths
r[molten.determinism.replay_fixture.tests] Molten SHOULD include tests for unchanged replay pass, changed identity input, changed effect response, changed policy or receipt boundary, live-effect denial under replay profile, and canonical readback of produced records.

#### Scenario: Tamper matrix catches first divergence
- GIVEN negative replay fixtures for identity, effect response, policy or receipt, output, and state-hash tampering
- WHEN the tests verify each fixture
- THEN each denial reports the expected first-divergence kind without accepting downstream matching refs as pass evidence

### Requirement: Vat replay binds generic verify evidence
r[molten.determinism.vat_generic_replay.bind_verify] The vat replay fixture SHOULD include generic `deterministic-replay-verify-v1` pass evidence in addition to vat-local replay receipts.

#### Scenario: Vat replay includes generic pass receipt
- GIVEN the vat replay fixture runs an unchanged replay scenario
- WHEN the fixture artifact is emitted
- THEN it includes a generic deterministic replay verification receipt with a pass decision
- AND the generic receipt ref is available in fixture diagnostics or embedded evidence

### Requirement: Vat replay binds generic first-divergence evidence
r[molten.determinism.vat_generic_replay.bind_divergence] The vat replay fixture SHOULD include generic first-divergence denial evidence for at least one mismatched boundary.

#### Scenario: Vat replay includes generic divergence receipt
- GIVEN the vat replay fixture includes a changed effect response or equivalent replay mismatch
- WHEN the fixture artifact is emitted
- THEN it includes a generic deterministic replay verification receipt with a deny decision
- AND it includes the corresponding `deterministic-first-divergence-v1` value when available

### Requirement: Vat-local replay evidence remains available
r[molten.determinism.vat_generic_replay.keep_vat_local] The vat replay fixture MUST preserve existing vat-local replay receipts while adding generic replay evidence.

#### Scenario: Existing vat replay tooling still sees vat receipts
- GIVEN existing tooling searches for `vat-replay-receipt-v1`
- WHEN the vat replay fixture is emitted after generic replay integration
- THEN the vat-local receipts remain present and canonical

### Requirement: Vat generic replay integration is tested
r[molten.determinism.vat_generic_replay.tests] Molten SHOULD test that vat replay fixture output contains generic pass, denial, and first-divergence records without treating those records as authority.

#### Scenario: Generic records are evidence-only
- GIVEN a vat replay fixture with generic replay verification evidence
- WHEN tests inspect the output
- THEN they find generic pass and denial evidence
- AND the fixture still states that replay evidence is evidence-only rather than authority or trust

### Requirement: Deterministic playback law
r[molten.determinism.central_law] Molten MUST define deterministic playback as a central runtime law: the same artifacts, dependency closure, initial state, schema refs, policy refs, handler profile, and seed or recorded effect log produce the same canonical traces, receipts, outputs, and final state hash.

#### Scenario: Same replay identity reproduces refs
- GIVEN a deterministic runtime run with fixed artifacts, initial state, handler profile, and seed or recorded effect log
- WHEN the run is replayed with the same identity inputs
- THEN the replay emits matching canonical trace refs, receipt refs, outputs, and final state hash

### Requirement: No ambient nondeterminism in deterministic profiles
r[molten.determinism.no_ambient_nondeterminism] Deterministic runtime profiles MUST NOT observe ambient clock, random, filesystem, network, environment, process, or OS scheduling inputs except through explicit recorded or deterministic effect responses.

#### Scenario: Ambient observation is denied
- GIVEN a deterministic or replay handler profile
- WHEN runtime execution attempts to observe an external source without an admitted effect response
- THEN execution is denied before semantic state changes and emits deterministic diagnostics

### Requirement: External observations cross effect boundaries
r[molten.determinism.effect_boundary] Runtime observations of clock, random, storage, blobs, network, process, filesystem, policy, and external services MUST cross canonical effect request and effect response boundaries before affecting deterministic state.

#### Scenario: Effect response enters replay identity
- GIVEN a runtime turn that needs a clock or random observation
- WHEN an admitted handler returns the observation
- THEN the canonical effect request and response refs are included in trace, receipt, and replay identity evidence

### Requirement: Replay identity binds all deterministic inputs
r[molten.determinism.identity_inputs] Deterministic replay identity MUST bind artifacts, dependency closure, initial state, schema refs, policy refs, capability state, handler profile, seed or effect-log hash, and runtime/tool versions where those inputs affect execution.

#### Scenario: Changed identity input diverges
- GIVEN a recorded deterministic run
- WHEN replay changes an input bound into replay identity
- THEN replay fails at the first changed boundary and reports expected and actual canonical refs

### Requirement: Total deterministic scheduler key
r[molten.determinism.scheduler] Local deterministic runtime scheduling MUST use a documented total canonical key and MUST NOT depend on map iteration order, thread races, or live arrival timing after events are admitted.

#### Scenario: Queue order is stable
- GIVEN two admitted events with canonical scheduler keys
- WHEN a deterministic profile selects the next actor turn
- THEN the event with the smaller canonical scheduler key is selected regardless of host scheduling behavior

### Requirement: Turn commit visibility
r[molten.determinism.turn_commit] Actors MUST process one event per turn and pending state, assertion, message, effect-intent, and evidence changes MUST become visible only after admitted commit.

#### Scenario: Denied turn rolls back pending changes
- GIVEN a turn with staged mutations and pending outbound actions
- WHEN admission or execution denies the turn
- THEN pending changes are discarded and the trace records a rollback or denial receipt

### Requirement: Logical clock handler
r[molten.determinism.logical_clock] Deterministic profiles MUST provide logical clock observations through explicit handler responses rather than ambient wall-clock reads.

#### Scenario: Logical time replays
- GIVEN a recorded logical clock effect response
- WHEN replay reaches the same clock request
- THEN replay injects the recorded logical time and denies any ambient wall-clock read

### Requirement: Seeded random handler
r[molten.determinism.seeded_random] Deterministic profiles MUST provide random bytes from explicit seed/config and request sequence or from recorded responses.

#### Scenario: Seeded random replays
- GIVEN a deterministic seed and random request sequence
- WHEN the same run is replayed
- THEN the random response refs match the recorded response refs

### Requirement: Deterministic chaos schedule
r[molten.determinism.chaos_schedule] Chaos profiles SHOULD represent faults, delays, drops, reorders, partitions, and resource limits as deterministic schedules bound into replay identity.

#### Scenario: Chaos fault is replayable
- GIVEN a chaos profile with a seeded fault schedule
- WHEN a delivery is delayed or dropped
- THEN the trace records the schedule position and replay reproduces the same fault decision

### Requirement: Turn journal evidence
r[molten.determinism.turn_journal] Deterministic runtime turns MUST emit canonical turn journal records with cause, scheduler key, input hash, before/after state hashes, effect refs, policy refs, committed actions, and receipt refs sufficient for replay comparison.

#### Scenario: Journal binds turn state
- GIVEN a committed deterministic actor turn
- WHEN the turn journal is emitted
- THEN it binds input, effect, policy, action, receipt, before-state, and after-state refs

### Requirement: Snapshot model
r[molten.determinism.snapshot_model] Replay MUST start from a canonical snapshot or authenticated snapshot refs covering runtime state, handler state, policy/capability state, dependency closure, and relevant storage or fixture refs.

#### Scenario: Snapshot seeds replay
- GIVEN a replay run with a snapshot ref
- WHEN replay initializes runtime state
- THEN state is derived from that snapshot and no additional authority is minted

### Requirement: State hashes
r[molten.determinism.state_hashes] Deterministic runtime profiles SHOULD compute state hashes from canonical snapshot representations or authenticated snapshot refs.

#### Scenario: State hash mismatch stops replay
- GIVEN a recorded after-state hash for a turn
- WHEN replay computes a different after-state hash
- THEN replay stops and reports a state-hash divergence

### Requirement: Trace privacy gates
r[molten.determinism.trace_privacy] Trace journals and snapshots that may contain secrets or capabilities MUST be subject to policy admission before export or rendering.

#### Scenario: Sensitive trace export is denied
- GIVEN a trace containing secret or capability-bearing refs
- WHEN an unauthorized export is requested
- THEN export denies or emits a redacted view without revealing protected content

### Requirement: Handler profiles
r[molten.determinism.handler_profiles] Molten MUST define pure, local, chaos, record, replay, and profiling handler profiles with canonical profile ids and config hashes.

#### Scenario: Profile id is evidence
- GIVEN a runtime report or receipt from a deterministic run
- WHEN the report is inspected
- THEN it includes the handler profile identity and enough config evidence to distinguish profile behavior

### Requirement: Record profile
r[molten.determinism.record_profile] Record profiles MUST record every admitted external effect response and relevant observation needed for later replay.

#### Scenario: Production observation is recorded
- GIVEN a record-profile run that calls a real adapter
- WHEN the adapter returns an observation
- THEN the canonical response evidence is stored in the effect log before affecting deterministic state

### Requirement: Replay profile
r[molten.determinism.replay_profile] Replay profiles MUST inject recorded effect responses, compare effect requests for exact match, and deny real external side effects.

#### Scenario: Replay does not consult outside world
- GIVEN a recorded effect log
- WHEN replay reaches an effect request
- THEN it compares the request ref, injects the recorded response, and does not call live external adapters

### Requirement: Replay algorithm
r[molten.determinism.replay_algorithm] Replay MUST verify input hashes, effect requests, effect responses, committed actions, receipts or traces, outputs, and after-state hashes turn by turn.

#### Scenario: Replay compares turn boundaries
- GIVEN a recorded deterministic run
- WHEN replay processes each turn
- THEN it checks the canonical refs at each semantic boundary before continuing to the next turn

### Requirement: First divergence diagnostics
r[molten.determinism.first_divergence] Replay SHOULD report the first divergent boundary with divergence kind, expected and actual canonical refs, handler profile, seed or log position, actor or turn id, and safe diagnostics.

#### Scenario: Input divergence is first
- GIVEN a recorded run and a replay with a changed input
- WHEN replay compares the turn input ref
- THEN replay stops at the input boundary and reports expected and actual input refs

### Requirement: Transcript integration
r[molten.determinism.transcript_integration] Executable transcripts MUST pin deterministic identity inputs and compare canonical trace, receipt, output, or diagnostic expectations.

#### Scenario: Transcript pins replay identity
- GIVEN an executable transcript for a deterministic runtime scenario
- WHEN the transcript is run as evidence
- THEN the report binds initial state, handler profile, seed or log hash, policy refs, and expected canonical outputs

### Requirement: Evaluation cache integration
r[molten.determinism.eval_cache_integration] Evaluation cache keys MUST include handler profile, seed/config, initial state hash, dependency closure, policy refs, and other deterministic identity inputs that affect results.

#### Scenario: Cache rejects changed profile
- GIVEN a cached deterministic result for one handler profile
- WHEN the same artifact runs under a different profile
- THEN the cache key differs or the entry is rejected

### Requirement: Remote sync replay integration
r[molten.determinism.remote_sync_integration] Remote artifact sync SHOULD record discovery, fetch, verification, and admission effects so replay can validate remote execution setup without live network dependence.

#### Scenario: Remote fetch is replayed from records
- GIVEN a recorded remote artifact fetch
- WHEN replay validates setup
- THEN it uses recorded fetch and verification evidence rather than live peer timing

### Requirement: Storage replay integration
r[molten.determinism.storage_integration] Typed storage replay SHOULD use fixture snapshots or recorded storage effect responses for deterministic reads and writes.

#### Scenario: Storage read is recorded
- GIVEN a production storage read in a record profile
- WHEN replay reaches the same read request
- THEN replay injects the recorded storage response and compares the request ref

### Requirement: Job DAG replay integration
r[molten.determinism.job_dag_integration] Distributed job DAG tests SHOULD use deterministic local, profiling, or chaos profiles and production incidents SHOULD be replayable from recorded effect logs where possible.

#### Scenario: Job replay binds handler profile
- GIVEN a job receipt with handler profile and effect-log refs
- WHEN replay validates the job
- THEN it checks profile identity and recorded effect refs before accepting matching output refs

### Requirement: Upgrade replay gate
r[molten.determinism.upgrade_gate] Upgrade sessions MAY require deterministic transcript or recorded playback success before cutover.

#### Scenario: Upgrade blocks on replay failure
- GIVEN an upgrade session with a required replay gate
- WHEN replay reports a divergence
- THEN the upgrade cutover is denied before mutation

### Requirement: Two-actor replay test
r[molten.determinism.two_actor_replay_test] Molten SHOULD include a local two-actor or two-object replay test proving identical artifacts, initial state, profile, and seed produce identical traces and final state hash.

#### Scenario: Vat replay fixture is stable
- GIVEN the local vat replay fixture with a fixed seed, profile, and initial object state
- WHEN the same two-object run is replayed
- THEN the fixture emits matching trace and final-state refs

### Requirement: Random and clock replay tests
r[molten.determinism.random_clock_replay_test] Molten SHOULD test that logical clock and seeded random handlers replay deterministically.

#### Scenario: Recorded clock response is stable
- GIVEN a deterministic clock or random response in the effect log
- WHEN replay runs with the same request sequence
- THEN the response refs match the recorded refs

### Requirement: Divergence tests
r[molten.determinism.divergence_tests] Molten SHOULD test first-divergence reporting for changed input, effect response, policy decision, and state hash boundaries.

#### Scenario: Changed response reports effect divergence
- GIVEN a recorded deterministic run
- WHEN replay uses a changed effect response
- THEN replay reports an effect-response divergence before comparing downstream state

### Requirement: No ambient tests
r[molten.determinism.no_ambient_tests] Molten SHOULD include tests or lints that reject ambient nondeterminism in deterministic core or runtime paths.

#### Scenario: Direct ambient read is rejected
- GIVEN code marked as deterministic runtime core
- WHEN it attempts to use ambient clock, random, filesystem, network, environment, process, or scheduler observations
- THEN tests or gates reject the change or require an explicit effect boundary

### Requirement: Determinism property tests
r[molten.determinism.property_tests] Molten SHOULD include property tests for replay identity, scheduler total order, trace hash stability, and snapshot authority preservation.

#### Scenario: Generated replay identity is stable
- GIVEN generated deterministic inputs within bounded limits
- WHEN the same identity is replayed
- THEN canonical trace and final-state refs remain stable

### Requirement: Remote dataspace CLI namespace
r[molten.remote_dataspace_harness_cli.remote_subcommand] The system MUST expose remote dataspace harness operations under `molten test remote`.

#### Scenario: CLI parses remote command
r[molten.remote_dataspace_harness_cli.remote_subcommand.parse]
- GIVEN a remote dataspace subcommand
- WHEN the CLI parses arguments
- THEN the selected handler receives typed command fields without using positional string dispatch

### Requirement: Envelope build command
r[molten.remote_dataspace_harness_cli.envelope_build] The system MUST provide a CLI command that builds canonical `remote-dataspace-envelope-v1` artifacts from explicit peer, actor, topic, operation, payload, content ref, capability ref, and evidence ref inputs.

#### Scenario: Build assertion envelope
r[molten.remote_dataspace_harness_cli.envelope_build.assert]
- GIVEN a payload file containing `<service-ready "db">`
- WHEN `molten test remote envelope build` is run for operation `assert`
- THEN the output file contains a canonical remote dataspace envelope whose ref is printed or available for later publish

### Requirement: Deterministic local publish/deliver commands
r[molten.remote_dataspace_harness_cli.publish_deliver_local] The system MUST expose CLI commands for deterministic local Iroh-shaped publish and deliver of remote dataspace envelopes.

#### Scenario: Publish then deliver locally
r[molten.remote_dataspace_harness_cli.publish_deliver_local.roundtrip]
- GIVEN a canonical remote dataspace envelope file
- WHEN it is published with `remote publish-local` and delivered with `remote deliver-local`
- THEN the delivered envelope ref matches the published envelope ref and transport receipts are emitted as canonical Preserves artifacts

### Requirement: Two-peer remote harness command
r[molten.remote_dataspace_harness_cli.run_two_peer] The system MUST provide a one-command deterministic two-peer remote dataspace scenario where peer A asserts `service.ready` and peer B observes it through the recorded delivery log.

#### Scenario: Two-peer run emits pass evidence
r[molten.remote_dataspace_harness_cli.run_two_peer.pass]
- GIVEN a transport root and output directory
- WHEN `remote run-two-peer` succeeds
- THEN it emits delivery log, admission receipt, gate receipt, and summary artifacts, and replay uses the recorded delivery log

### Requirement: Remote dataspace gate CLI
r[molten.remote_dataspace_harness_cli.gate_command] The system MUST provide a CLI command that creates a remote dataspace gate receipt only from replayable delivery logs, admission receipts, and turn-journal context refs.

#### Scenario: Non-replayable log is denied
r[molten.remote_dataspace_harness_cli.gate_command.non_replayable]
- GIVEN a non-replayable remote delivery log
- WHEN the gate command is run
- THEN it fails closed before emitting pass evidence

### Requirement: Remote service-ready example
r[molten.remote_dataspace_harness_cli.example_fixture] The system MUST include an example Preserves payload fixture for the remote service-ready scenario.

#### Scenario: Example parses
r[molten.remote_dataspace_harness_cli.example_fixture.parses]
- GIVEN `examples/remote-service-ready.preserves`
- WHEN it is parsed as Preserves
- THEN it yields the service-ready payload used by the CLI demonstration

### Requirement: Versioned envelope spine
r[molten.runtime_spine.envelope] The system MUST define a versioned envelope type with Serde DTO boundaries that carries sender identity, routable subject, Preserves body, blob references, capabilities, and evidence references.

#### Scenario: Native actors exchange an envelope
r[molten.runtime_spine.envelope.native_exchange]
- GIVEN two native Molten actors in the same runtime
- WHEN one actor sends a valid envelope to a subject observed by the other actor
- THEN the receiving actor observes the same envelope fields after routing

### Requirement: Canonical Preserves boundary
r[molten.runtime_spine.canonical_preserves] The system MUST define Blake3 boundary hashes over canonical Preserves bytes rather than over incidental Rust memory layout or debug formatting.

#### Scenario: Equivalent envelope encodings hash identically
r[molten.runtime_spine.canonical_preserves.stable_hash]
- GIVEN two equivalent envelope values constructed through different Rust code paths
- WHEN each envelope is converted to canonical Preserves bytes
- THEN both canonical byte streams produce the same boundary hash

### Requirement: Preserves communication boundary
r[molten.runtime_spine.preserves_comms] Every Molten communication surface that crosses a runtime, trust, transport, execution, storage, policy, or evidence boundary MUST have a canonical Preserves representation, even when Rust structs or adapter-native types are used internally.

#### Scenario: Actor and dataspace messages use Preserves boundary
r[molten.runtime_spine.preserves_comms.dataspace]
- GIVEN a local actor envelope or dataspace assertion/message
- WHEN the message crosses the actor or dataspace adapter boundary
- THEN the communicated value is representable as canonical Preserves bytes for hashing, routing, policy admission, and evidence

#### Scenario: Protocol and consensus messages use Preserves boundary
r[molten.runtime_spine.preserves_comms.protocol_consensus]
- GIVEN a choreography protocol-message envelope or a Raft command/message envelope
- WHEN the message is routed locally, transported remotely, persisted, or admitted by policy
- THEN the protocol or consensus message is represented at the boundary by canonical Preserves bytes with stable identity

#### Scenario: Large payload uses Preserves reference
r[molten.runtime_spine.preserves_comms.large_payload]
- GIVEN a communication payload too large or unsuitable to carry inline
- WHEN the payload is sent through Molten
- THEN the envelope carries canonical Preserves metadata and content references while the large bytes may be stored or transported through a blob adapter

### Requirement: Pure core boundary
r[molten.runtime_spine.core_purity] The core envelope and validation layer MUST be deterministic and MUST NOT perform filesystem, network, process, clock, scripting, or runtime scheduling effects.

#### Scenario: Core validation runs without adapters
r[molten.runtime_spine.core_purity.no_adapters]
- GIVEN an envelope fixture and no runtime adapters
- WHEN core validation checks the fixture
- THEN validation returns only deterministic data derived from the fixture

### Requirement: Snafu error boundary
r[molten.runtime_spine.error_boundary] The system MUST use structured error types at core validation and adapter boundaries so callers can distinguish invalid input, denied operations, unavailable adapters, and persistence failures.

#### Scenario: Adapter failure is structured
r[molten.runtime_spine.error_boundary.adapter_failure]
- GIVEN a runtime adapter that cannot complete a requested side effect
- WHEN the adapter reports the failure
- THEN the caller receives a structured error category and source context rather than an unstructured string

### Requirement: Runtime reference boundaries
r[molten.runtime_spine.runtime_references] The system MUST document BEAM/OTP and Lunatic as non-normative design references for actor lifecycle, supervision, mailboxes, links/monitors, scheduling, and Wasm hostcall ergonomics, and MUST NOT claim BEAM distribution, OTP behavior, Erlang/Elixir API, or Lunatic API compatibility.

#### Scenario: Reference material does not become compatibility claim
r[molten.runtime_spine.runtime_references.non_compatibility]
- GIVEN runtime design material that cites BEAM/OTP or Lunatic
- WHEN Molten describes a borrowed runtime pattern
- THEN the material states the Molten-specific envelope, policy, evidence, and transport boundaries instead of claiming protocol or API compatibility

### Requirement: Local dataspace adapter
r[molten.runtime_spine.local_dataspace] The system MUST provide a local runtime adapter that routes envelopes through actor, assertion, subscription, and dataspace concepts without leaking those mechanisms into the pure core.

#### Scenario: Subscription receives matching envelope
r[molten.runtime_spine.local_dataspace.subscription]
- GIVEN a local actor subscribed to a subject pattern
- WHEN another local actor sends a matching envelope
- THEN the subscribed actor receives the envelope through the runtime adapter

### Requirement: Declarative startup configuration
r[molten.runtime_spine.config] The system MUST evaluate Nickel-authored configuration into typed startup configuration before runtime dispatch begins.

#### Scenario: Config declares an actor and subscription
r[molten.runtime_spine.config.actor_subscription]
- GIVEN a Nickel configuration that declares a native actor and a subscription
- WHEN Molten loads the configuration
- THEN the runtime starts with the declared actor and subscription represented as typed Rust config

### Requirement: Clap CLI surface
r[molten.runtime_spine.cli] The system MUST expose command-line operations through a Clap-based CLI surface.

#### Scenario: CLI parses config path
r[molten.runtime_spine.cli.config_path]
- GIVEN a user-provided runtime configuration path
- WHEN Molten parses CLI arguments
- THEN the selected command receives a typed configuration path value

### Requirement: Tracing observability
r[molten.runtime_spine.observability] The system MUST emit structured tracing spans or events at runtime and adapter boundaries.

#### Scenario: Adapter decision emits trace event
r[molten.runtime_spine.observability.adapter_decision]
- GIVEN an adapter admission decision
- WHEN the runtime records the decision
- THEN a structured tracing event identifies the adapter, decision, and envelope subject

### Requirement: Iroh remote bridge
r[molten.runtime_spine.remote_bridge] The system MUST bridge envelope-sized messages over Iroh gossip, large immutable payloads over Iroh blobs, and replicated mutable document/state surfaces over Iroh docs.

#### Scenario: Remote peer receives envelope and blob reference
r[molten.runtime_spine.remote_bridge.blob_reference]
- GIVEN two Molten peers joined to the same authorized topic
- WHEN one peer publishes an envelope with a content reference
- THEN the other peer receives the envelope over gossip and can fetch the referenced payload through the blob adapter

### Requirement: Blob reference bridge
r[molten.runtime_spine.blob_refs] The system MUST provide a blob adapter for content-addressed payload references carried by runtime envelopes, while keeping large payload bytes out of the canonical envelope body.

#### Scenario: Envelope declares external blob reference
r[molten.runtime_spine.blob_refs.declared]
- GIVEN an envelope carrying a canonical content reference for an external payload
- WHEN the blob adapter stores or fetches the payload
- THEN the adapter verifies the bytes against the declared reference before the payload is admitted

### Requirement: Iroh docs bridge
r[molten.runtime_spine.docs_bridge] The system MUST expose Iroh docs through a runtime adapter that records envelope-level evidence for application-visible document mutations.

#### Scenario: Document mutation emits evidence
r[molten.runtime_spine.docs_bridge.mutation_evidence]
- GIVEN a Molten actor with an admitted document mutation capability
- WHEN the actor applies a mutation through the Iroh docs adapter
- THEN the runtime records the document namespace, mutation reference, and admission evidence in an envelope or receipt

### Requirement: Remote content admission
r[molten.runtime_spine.remote_admission] The system MUST reject remote envelopes when declared blob references or canonical envelope hashes fail validation.

#### Scenario: Tampered blob is rejected
r[molten.runtime_spine.remote_admission.tampered_blob]
- GIVEN a remote envelope that declares a content reference
- WHEN the fetched payload does not match the declared reference
- THEN the runtime rejects the payload before delivering it to actors

### Requirement: Sandboxed Wasmtime actors
r[molten.runtime_spine.wasmtime_hostcalls] The system MUST expose sandboxed Wasmtime actor hostcalls for envelope send, subscription, blob read, and blob write while denying ambient filesystem and network access.

#### Scenario: Wasm actor sends through hostcall
r[molten.runtime_spine.wasmtime_hostcalls.send]
- GIVEN a Wasmtime actor with an admitted send capability
- WHEN the actor calls the send hostcall with a valid envelope
- THEN the runtime applies admission checks and routes the envelope only if admitted

### Requirement: Deny-by-default WASI capabilities
r[molten.runtime_spine.wasi_capabilities] The system MUST use Wasmtime-WASI only through explicit capability wiring and MUST deny ambient filesystem, clock, environment, and socket access by default.

#### Scenario: WASI filesystem access is denied without capability
r[molten.runtime_spine.wasi_capabilities.filesystem_denied]
- GIVEN a Wasmtime actor without an admitted filesystem capability
- WHEN the actor attempts to access a host filesystem path through WASI
- THEN the runtime denies the access before exposing host path contents

### Requirement: WIT component admission
r[molten.runtime_spine.wit_components] The system MUST support WIT/component bindings for typed actor interfaces and wasmparser-based module inspection before actor admission.

#### Scenario: Invalid component import is rejected
r[molten.runtime_spine.wit_components.invalid_import]
- GIVEN a Wasm component declaring an import outside the admitted hostcall surface
- WHEN the runtime inspects the component before admission
- THEN the component is rejected before instantiation

### Requirement: Trusted Steel orchestration
r[molten.runtime_spine.steel_orchestration] The system MUST expose Steel orchestration APIs that operate through the same envelope spine as native, remote, and Wasmtime actors.

#### Scenario: Steel script spawns and inspects actors
r[molten.runtime_spine.steel_orchestration.spawn_inspect]
- GIVEN a trusted Steel orchestration script
- WHEN the script spawns an actor and inspects runtime state
- THEN those operations use public runtime APIs and produce inspectable envelope or receipt evidence

### Requirement: Deny-by-default adapter effects
r[molten.runtime_spine.deny_by_default] The system MUST deny adapter side effects by default unless an explicit policy gate admits the requested operation.

#### Scenario: Missing capability denies send
r[molten.runtime_spine.deny_by_default.missing_capability]
- GIVEN an actor without a matching send capability
- WHEN the actor requests a send side effect
- THEN the runtime denies the side effect before any local or remote delivery occurs

### Requirement: Nickel and Steel contract selection
r[molten.runtime_spine.nickel_steel_contracts] The system MUST use Nickel contracts for static declarative policy, schema, resource, ability, adapter-option, and configuration gates, and MUST use Steel contracts only for explicitly reviewed dynamic predicates or trusted callables that cannot be represented as static Nickel data.

#### Scenario: Static policy uses Nickel contract
r[molten.runtime_spine.nickel_steel_contracts.static_nickel]
- GIVEN a runtime action governed by static resource prefixes, allowed abilities, or adapter options
- WHEN Molten evaluates the action before side effects
- THEN the admission path uses a Nickel-authored contract artifact and records the contract id and normalized source hash in evidence

#### Scenario: Dynamic predicate uses Steel contract
r[molten.runtime_spine.nickel_steel_contracts.dynamic_steel]
- GIVEN a runtime action that requires an explicitly reviewed dynamic predicate or trusted callable
- WHEN Molten evaluates the action before side effects
- THEN the admission path uses a Steel contract backend and records the backend, contract id, decision, and receipt reference in evidence

### Requirement: Basalt contract enforcement
r[molten.runtime_spine.basalt_contracts] The system MUST support Basalt-backed UCAN contract enforcement for capability-bearing runtime requests that are governed by Nickel policy artifacts or Steel contract backends.

#### Scenario: UCAN contract admits bounded request
r[molten.runtime_spine.basalt_contracts.admit]
- GIVEN a runtime request with a Basalt contract id, resource, ability, and matching UCAN capability grant
- WHEN the policy layer evaluates the request
- THEN the operation is admitted only for the matching resource and ability

### Requirement: Policy gate integration
r[molten.runtime_spine.policy_gate] The system MUST support bounded policy gates using Trellis predicates for capabilities, replay checks, leases, routing limits, and content integrity.

#### Scenario: Policy gate records admission decision
r[molten.runtime_spine.policy_gate.receipt]
- GIVEN an envelope that requests a gated operation
- WHEN the policy layer evaluates the operation
- THEN the runtime records whether the operation was admitted or rejected and which bounded predicate was applied

### Requirement: Cairn receipt validation
r[molten.runtime_spine.cairn_receipts] The system MUST validate action-envelope and lifecycle receipts through Cairn surfaces before treating them as runtime evidence.

#### Scenario: Invalid receipt is not evidence
r[molten.runtime_spine.cairn_receipts.invalid]
- GIVEN an envelope with an attached Cairn receipt reference
- WHEN the referenced receipt fails Cairn validation
- THEN the runtime excludes that receipt from admitted evidence

### Requirement: Octet/Valence evidence references
r[molten.runtime_spine.valence_evidence] The system MUST support Octet/Valence evidence references for function object, module, and provenance claims without treating those references as proof of semantic correctness.

#### Scenario: Function object evidence is bounded
r[molten.runtime_spine.valence_evidence.boundary]
- GIVEN an envelope that references function object evidence
- WHEN the runtime displays or evaluates the evidence reference
- THEN it reports the bounded evidence claim and does not claim general semantic equivalence

### Requirement: Redb local store adapter
r[molten.runtime_spine.redb_store] The system MUST support a Redb-backed adapter for durable local metadata, receipt indexes, replay caches, and content-reference bookkeeping while keeping filesystem effects out of the pure core.

#### Scenario: Store adapter records receipt index
r[molten.runtime_spine.redb_store.receipt_index]
- GIVEN an admitted runtime operation that emits a receipt reference
- WHEN the Redb store adapter persists the local index entry
- THEN later inspection can recover the receipt reference without re-running pure admission logic

### Requirement: Integration evidence
r[molten.runtime_spine.integration_evidence] The system MUST provide end-to-end evidence that runtime configuration, local routing, remote bridge handling, and policy admission preserve envelope boundaries across adapters.

#### Scenario: Configured route emits boundary evidence
r[molten.runtime_spine.integration_evidence.config_route]
- GIVEN a runtime configuration that starts a local actor, remote bridge, and policy gate
- WHEN an admitted envelope traverses those adapters
- THEN the emitted evidence links the configuration, local route, remote bridge, and policy decision without granting extra authority

### Requirement: Hegel property tests
r[molten.runtime_spine.property_tests] The system MUST use Hegel property-based tests for envelope, admission, and adapter invariants that are too broad for hand-written examples alone.

#### Scenario: Generated envelopes preserve canonical identity
r[molten.runtime_spine.property_tests.generated_envelopes]
- GIVEN a generated valid envelope
- WHEN the property test converts it through the supported DTO and canonical encoding boundaries
- THEN the envelope identity and canonical hash remain stable

### Requirement: Lifecycle state model
r[molten.lifecycle.state_model] Molten MUST define canonical lifecycle states and transition records for actors, services, vats, sessions, handlers, and jobs.

#### Scenario: Transition binds entity and state
- GIVEN a runtime entity with a current lifecycle state
- WHEN the entity records a lifecycle transition
- THEN the transition binds the entity kind, entity id, prior state, next state, action, cause, policy refs, resource refs, evidence refs, optional supervisor ref, and logical step

#### Scenario: Invalid transition is denied
- GIVEN a lifecycle transition that jumps across required intermediate states
- WHEN Molten evaluates the transition
- THEN the transition receipt is denied with diagnostics and no compatibility claim is made for BEAM, OTP, or Lunatic semantics

### Requirement: Lifecycle transition receipts
r[molten.lifecycle.transition_receipts] Molten MUST emit canonical receipts for spawn, start, ready, degraded, fail, restart, stop, cleanup, and supervisor lifecycle decisions.

#### Scenario: Receipt binds transition ref
- GIVEN a lifecycle transition value
- WHEN Molten emits a lifecycle receipt
- THEN the receipt binds the canonical transition ref, decision, diagnostics, and lifecycle evidence schema

#### Scenario: Supervisor decision is explicit evidence
- GIVEN a supervisor evaluates a child lifecycle transition
- WHEN the supervisor records its decision
- THEN Molten emits a lifecycle transition receipt instead of hiding the decision behind automatic restart behavior

### Requirement: Lifecycle prior-art boundary
r[molten.lifecycle.no_otp_compat] Molten MUST document BEAM, OTP, and Lunatic as prior art only and MUST NOT claim compatibility with their runtime, distribution, restart, mailbox, or supervision semantics.

#### Scenario: Lifecycle evidence states Molten semantics
- GIVEN lifecycle transition evidence
- WHEN the evidence is inspected
- THEN it identifies Molten-local lifecycle semantics and does not assert OTP or Lunatic compatibility

### Requirement: Lifecycle trace events
r[molten.lifecycle.trace_events] Molten MUST emit tracing events for lifecycle transitions with entity, cause, action, policy refs, transition ref, and logical step.

#### Scenario: Trace event binds cause and policy
- GIVEN a lifecycle transition admitted under policy
- WHEN Molten emits the trace event
- THEN the event binds the transition ref, cause, policy refs, action, and entity identity

### Requirement: Failed turn rollback evidence
r[molten.lifecycle.turn_failure] Molten MUST roll back pending turn actions and vat deltas on panic, denial, or validation failure and MUST emit canonical failure evidence for discarded work.

#### Scenario: Denied turn discards pending actions
- GIVEN a runtime turn with staged messages, assertions, observations, or vat deltas
- WHEN policy denial, panic, or validation failure aborts the turn
- THEN the after-rollback state matches the before state and failure evidence binds the discarded action refs, pending turn ref, policy refs, evidence refs, and any discarded vat delta refs

#### Scenario: Mutated rollback is denied
- GIVEN a failed turn receipt whose after-rollback state differs from the before state
- WHEN Molten validates the turn failure evidence
- THEN the receipt decision is denied with diagnostics instead of treating partial mutation as a successful rollback

### Requirement: Scope cleanup
r[molten.lifecycle.scope_cleanup] Molten MUST retract owned assertions, subscriptions, live references, and admitted resources when an actor, service, vat, session, handler, or job stops, crashes, loses authority, or disconnects.

#### Scenario: Stop retracts owned scope
- GIVEN an entity with owned runtime scope entries
- WHEN the entity stops, crashes, loses authority, or disconnects
- THEN cleanup evidence identifies the retracted assertions, subscriptions, live refs, and released resources

### Requirement: Idempotent cleanup
r[molten.lifecycle.cleanup_idempotent] Molten MUST make lifecycle cleanup idempotent and receipt-backed so repeated cleanup attempts do not reintroduce state or duplicate destructive side effects.

#### Scenario: Repeated cleanup is stable
- GIVEN an entity whose cleanup has already completed
- WHEN cleanup is requested again
- THEN Molten emits stable cleanup evidence and leaves runtime state unchanged

### Requirement: One-shot effect failure traces
r[molten.lifecycle.one_shot_effects] Molten MUST report irreversible one-shot effects explicitly in failure traces instead of implying that external effects were rolled back.

#### Scenario: Irreversible effect is disclosed
- GIVEN a failed turn after an irreversible external effect was requested or completed
- WHEN Molten emits failure evidence
- THEN the evidence distinguishes rolled-back local state from one-shot effects that require compensation or review

### Requirement: Links and monitors
r[molten.lifecycle.links_monitors] Molten MUST provide policy-controlled links and monitors for lifecycle failure propagation and observation.

#### Scenario: Monitor observes failure without authority escalation
- GIVEN a monitor authorized for a child entity
- WHEN the child fails
- THEN the monitor observes failure evidence without gaining child authority

### Requirement: Local supervisors
r[molten.lifecycle.supervisors] Molten MUST provide local supervisors with never, one-for-one, and bounded restart strategies.

#### Scenario: One-for-one supervisor restarts child
- GIVEN a one-for-one supervisor policy and a failed child
- WHEN restart admission passes
- THEN Molten records a supervisor decision and restarts only the failed child

### Requirement: Restart windows
r[molten.lifecycle.restart_windows] Molten MUST use logical-time restart windows and resource budgets to throttle restarts.

#### Scenario: Restart budget exhaustion denies restart
- GIVEN a child exceeding the configured restart budget within a logical-time window
- WHEN the supervisor evaluates restart
- THEN restart is denied with budget diagnostics

### Requirement: Service lifecycle assertions
r[molten.lifecycle.service_assertions] Molten MUST represent service demand, readiness, failure, dependency, exposed references, restart, and stop signals as dataspace assertions.

#### Scenario: Service readiness is a dataspace assertion
- GIVEN a service that reaches readiness
- WHEN it publishes lifecycle state
- THEN readiness is represented as a canonical dataspace assertion with lifecycle evidence refs

### Requirement: Failure rollback tests
r[molten.lifecycle.failure_tests] Molten MUST test that failed turns discard pending actions and emit failure receipts.

#### Scenario: Failed turn test observes no pending mutation
- GIVEN a test turn with staged actions
- WHEN denial or validation failure aborts the turn
- THEN the test observes unchanged state and a failure receipt binding discarded actions

### Requirement: Cleanup tests
r[molten.lifecycle.cleanup_tests] Molten MUST test that actor stop or crash retracts owned assertions and subscriptions.

#### Scenario: Cleanup test observes retractions
- GIVEN a test actor with owned assertions and subscriptions
- WHEN the actor stops or crashes
- THEN the test observes cleanup evidence and no leaked owned entries

### Requirement: Restart tests
r[molten.lifecycle.restart_tests] Molten MUST test deterministic supervisor restarts with bounded restart windows.

#### Scenario: Restart test respects window
- GIVEN a supervisor restart window fixture
- WHEN child failures are replayed deterministically
- THEN allowed and denied restarts match the configured logical-time budget

### Requirement: Lifecycle property tests
r[molten.lifecycle.property_tests] Molten MUST include Hegel property tests for cleanup idempotence, no leaked assertions, and restart bounds.

#### Scenario: Generated cleanup is idempotent
- GIVEN generated lifecycle cleanup inputs
- WHEN cleanup is applied repeatedly
- THEN final state is stable, assertions do not leak, and restart counts remain bounded

## Effect Handler Manifests

### Requirement: Effect manifests are canonical
r[molten.effects.manifest_model] Executable artifacts MUST describe admitted effects with canonical `effect-manifest-v1` records that bind artifact kind, artifact ref, executor kind, declared effect ids, operation names, schema refs, policy refs, evidence refs, and no-Unison-runtime-compatibility checks.

#### Scenario: Manifest identity is stable
- GIVEN an executable artifact and its declared effects
- WHEN Molten renders the effect manifest
- THEN the manifest has a stable canonical content ref
- AND records that Unison abilities are non-normative prior art only.

### Requirement: Effect ids are stable and declared
r[molten.effects.effect_ids] Declared effect ids MUST be lowercase deterministic ids bound to operation names and input/output schema refs, and duplicate effect-id/operation pairs MUST fail closed.

#### Scenario: Duplicate declared effect is rejected
- GIVEN a manifest containing the same effect id and operation twice
- WHEN Molten validates the manifest
- THEN validation denies the manifest before it can admit handler bindings.

### Requirement: Artifacts link effect manifests
r[molten.effects.artifact_link] Artifact records for executable Wasm, Steel, native, choreography, job, adapter, or remote-proxy code MUST link admitted effect manifests by content ref instead of relying on ambient runtime knowledge.

#### Scenario: Artifact effects field binds manifest ref
- GIVEN an executable artifact installed in the registry
- WHEN it declares effects
- THEN the artifact's canonical effects field points at the effect manifest ref
- AND the manifest itself binds back to the artifact ref.

### Requirement: Unison runtime compatibility is not claimed
r[molten.effects.no_unison_runtime] Molten MUST document Unison abilities/effects as prior art only and MUST NOT claim Unison syntax, type system, runtime, or generalized algebraic effect compatibility.

#### Scenario: Manifest records reference boundary
- GIVEN a Molten effect manifest inspired by Unison-style ability declarations
- WHEN the manifest is rendered
- THEN its checks record that Molten does not implement Unison runtime compatibility.

### Requirement: Handler profiles are explicit
r[molten.effects.handler_profiles] Molten MUST represent admitted handler profiles with canonical `handler-profile-v1` records for production, local, mock, chaos, profiling, and dry-run profiles, binding policy, capability, resource, handler binding, and evidence refs.

#### Scenario: Unsupported profile is rejected
- GIVEN an effect request naming an unsupported handler profile
- WHEN Molten parses the request or profile
- THEN validation fails before any effect handler is invoked.

### Requirement: Effect binding receipts gate requests
r[molten.effects.binding_receipts] Molten MUST emit canonical `effect-binding-receipt-v1` records that bind manifest ref, handler-profile ref, request ref, effect id, operation, decision, diagnostics, evidence refs, and deny-undeclared-effect checks.

#### Scenario: Declared effect receives pass receipt
- GIVEN a request whose artifact, effect id, operation, and handler profile match an admitted manifest and profile
- WHEN Molten admits the request
- THEN it emits a passing effect binding receipt.

#### Scenario: Undeclared effect receives deny receipt
- GIVEN a request for an effect id or operation absent from the artifact manifest
- WHEN Molten admits the request
- THEN it emits a deny receipt with diagnostics
- AND no handler side effect is authorized by the request shape.

### Requirement: Effect request and response envelopes are canonical
r[molten.effects.request_envelope] Effect requests and responses MUST use canonical `effect-request-v1` and `effect-response-v1` envelopes binding artifact refs, effect ids, handler profiles, input/output refs, capability refs, diagnostics, evidence refs, and decision checks.

#### Scenario: Request and response refs are stable
- GIVEN the same artifact ref, effect id, handler profile, input ref, capabilities, and evidence refs
- WHEN Molten renders the effect request and response envelopes
- THEN their canonical refs are stable and replayable.

### Requirement: Undeclared effects deny before side effects
r[molten.effects.deny_undeclared] Molten MUST reject effect requests whose effect id and operation pair is absent from the artifact's admitted effect manifest before exposing Wasmtime hostcalls, Steel APIs, adapter calls, or remote proxy operations.

#### Scenario: Hostcall is absent from manifest
- GIVEN an executable artifact with a manifest declaring only `dataspace.send`
- WHEN it requests `blob.get`
- THEN Molten emits a deny binding receipt before exposing the hostcall or adapter operation.

### Requirement: Wasmtime hostcalls require admitted effects
r[molten.effects.wasmtime_hostcall_gate] Wasmtime executor hostcalls MUST be exposed only when the hostcall request carries canonical effect manifest, handler profile, effect request, and passing binding receipt refs for the requested operation.

#### Scenario: Wasm hostcall carries binding proof
- GIVEN a Wasm actor whose allowed hostcall is declared in its admitted effect manifest
- WHEN the actor invokes the hostcall
- THEN the Wasm execution receipt records `effect-manifest-bound`, `effect-request-admitted`, and `declared-effect-id-required` checks.

### Requirement: Steel runtime APIs require admitted effects
r[molten.effects.steel_api_gate] Reviewed Steel executor APIs MUST require the same admitted effect request binding before returning hostcall responses, and MUST avoid ambient adapter access.

#### Scenario: Steel hostcall carries binding proof
- GIVEN a Steel actor whose allowed hostcall is declared in its admitted effect manifest
- WHEN the actor calls `molten-hostcall`
- THEN the Steel execution receipt records `effect-manifest-bound`, `effect-request-admitted`, and `declared-effect-id-required` checks.

### Requirement: Dataspace handlers are explicit
r[molten.effects.dataspace_handlers] Dataspace send and observe effects MUST use declared local or production handler bindings rather than ambient runtime access.

#### Scenario: Dataspace send uses handler binding
- GIVEN an actor declares a dataspace send effect
- WHEN Molten executes the effect through a local or production profile
- THEN the request is admitted through a handler binding before any message is delivered.

### Requirement: Blob handlers are explicit
r[molten.effects.blob_handlers] Blob get and blob put effects MUST use declared local or Iroh-backed handler bindings with canonical request and response refs.

#### Scenario: Blob get uses handler binding
- GIVEN an actor declares a blob get effect
- WHEN Molten executes the effect through an Iroh-backed profile
- THEN the blob request is admitted through a handler binding before any blob bytes are read.

### Requirement: Typed storage handlers are explicit
r[molten.effects.storage_handlers] Typed storage read and write effects MUST use declared local or Redb-backed handler bindings and MUST bind typed storage refs in effect evidence.

#### Scenario: Storage write uses handler binding
- GIVEN an actor declares a typed storage write effect
- WHEN Molten executes the write through a Redb-backed profile
- THEN the write is admitted through a handler binding before persisted state changes.

### Requirement: Time and random handlers deny by default
r[molten.effects.time_random_handlers] Clock and random effects MUST deny by default unless a deterministic local test handler or explicitly admitted production handler is bound.

#### Scenario: Clock lacks handler
- GIVEN an actor requests clock access without an admitted time handler
- WHEN Molten evaluates the request
- THEN the request is denied before any wall-clock value is exposed.

#### Scenario: Deterministic local clock and random handlers are receipted
- GIVEN an actor has admitted clock or random capability in a local deterministic harness run
- WHEN Molten produces the effect response
- THEN the observation includes a `time-random-handler-receipt-v1` binding the request ref, handler binding ref, response ref, and local deterministic profile.

### Requirement: Chaos handler profile is bounded
r[molten.effects.chaos_profile] Chaos handler profiles MUST bound deterministic fault, delay, reorder, and partition injection and record the applied chaos profile in effect evidence.

#### Scenario: Chaos delay is bounded
- GIVEN a chaos profile with a maximum deterministic delay
- WHEN a handler injects delay
- THEN the effect evidence records the bounded delay and replay uses the same value.

### Requirement: Profiling handler profile records effect metrics
r[molten.effects.profiling_profile] Profiling handler profiles MUST record effect counts, payload sizes, dependency fetches, and trace refs without granting additional effect authority.

#### Scenario: Profiling records counts
- GIVEN an actor runs under the profiling profile
- WHEN it executes admitted effects
- THEN Molten records effect counts and payload sizes as profiling evidence only.

### Requirement: Transcript tests pin handler traces
r[molten.effects.transcript_tests] Executable transcript tests MUST pin handler profiles and expected canonical traces or receipts for effect-handler behavior.

#### Scenario: Transcript pins receipt
- GIVEN a transcript fixture with a declared handler profile
- WHEN the fixture runs
- THEN the observed effect receipts match the pinned canonical trace.

### Requirement: Property tests cover handler substitution
r[molten.effects.property_tests] Property tests SHOULD cover deny-by-default behavior, handler substitution, and effect-request determinism across equivalent inputs.

#### Scenario: Equivalent requests are deterministic
- GIVEN two equivalent effect requests with identical refs and profile
- WHEN property tests render their envelopes
- THEN the canonical request refs are equal.

### Requirement: Lifecycle transition relation is finite and explicit
r[molten.lifecycle_state_machine_proof.transition_relation_table] Molten MUST expose the lifecycle transition relation as a bounded, reviewable finite relation so proof tests can enumerate every lifecycle source and target state without relying on adapter behavior.

#### Scenario: Allowed edge appears in the matrix
- GIVEN the lifecycle state enum and the lifecycle transition relation
- WHEN the lifecycle proof matrix enumerates all source and target states
- THEN every permitted lifecycle edge appears in the relation exactly once
- AND no unlisted lifecycle edge produces a passing transition receipt.

### Requirement: Lifecycle action-target matrix is exhaustive
r[molten.lifecycle_state_machine_proof.action_target_matrix] Molten MUST prove lifecycle receipt decisions across every lifecycle state, action, and target-state combination, and a receipt MUST pass only when the state edge is allowed and the action is valid for the target state or is an explicit supervisor decision.

#### Scenario: Mismatched action denies
- GIVEN an allowed lifecycle edge with an action that does not match the target state
- WHEN Molten evaluates the lifecycle transition receipt
- THEN the receipt decision is `deny`
- AND diagnostics identify the action-target mismatch.

### Requirement: Lifecycle graph reachability matches the specified state model
r[molten.lifecycle_state_machine_proof.reachability] Molten MUST prove that lifecycle states reachable from `declared` are reachable only through the specified lifecycle transition relation, and forbidden shortcuts MUST deny before producing passing lifecycle evidence.

#### Scenario: Startup path is reachable without shortcuts
- GIVEN a lifecycle entity in the `declared` state
- WHEN the lifecycle proof computes reachable states from the allowed transition relation
- THEN `spawning`, `starting`, and `ready` are reachable through their required intermediate states
- AND a direct `declared` to `ready` transition denies.

### Requirement: Lifecycle terminal and cleanup boundaries are closed
r[molten.lifecycle_state_machine_proof.terminal_cleanup] Molten MUST prove terminal and cleanup boundaries in the lifecycle graph: `cleaned` has no outgoing passing transition, `stopped` can only clean up, `failed` can only restart or clean up, and `restarting` can only return to starting or clean up.

#### Scenario: Cleaned state cannot exit
- GIVEN a lifecycle entity already in the `cleaned` state
- WHEN any lifecycle transition is evaluated from `cleaned`
- THEN the transition receipt decision is `deny`
- AND no outgoing lifecycle edge is accepted.

### Requirement: Lifecycle denial diagnostics are deterministic
r[molten.lifecycle_state_machine_proof.denial_diagnostics] Molten MUST emit bounded, deterministic lifecycle transition diagnostics for denied transition predicates, including invalid state edges and action-target mismatches.

#### Scenario: Invalid transition names the denied edge
- GIVEN a lifecycle transition that jumps across required intermediate states
- WHEN Molten evaluates the lifecycle transition receipt
- THEN the receipt decision is `deny`
- AND diagnostics identify the invalid source and target state edge.

#### Scenario: Multiple predicate failures stay stable
- GIVEN a lifecycle transition whose state edge is invalid and whose action does not match the target state
- WHEN Molten evaluates the transition more than once
- THEN the diagnostic strings appear in the same order each time
- AND the denial receipt ref is stable for the same canonical input.

### Requirement: Lifecycle denial receipts bind failed checks
r[molten.lifecycle_state_machine_proof.denial_receipt_binding] Molten MUST bind denial receipts to the canonical lifecycle transition ref, the `deny` decision, deterministic diagnostics, and lifecycle check names whenever transition input validation succeeds.

#### Scenario: Denial receipt remains proof evidence
- GIVEN a syntactically valid lifecycle transition that fails semantic transition checks
- WHEN Molten emits the lifecycle transition receipt
- THEN the receipt binds the transition ref and denial diagnostics
- AND the receipt MUST NOT be accepted as a passing lifecycle transition.

### Requirement: Lifecycle receipts are deterministic for identical inputs
r[molten.lifecycle_state_machine_proof.receipt_determinism] Molten MUST produce stable lifecycle transition refs, receipt refs, decisions, diagnostics, and canonical receipt values when the same lifecycle transition input is evaluated more than once.

#### Scenario: Repeated receipt generation is stable
- GIVEN a lifecycle transition input with canonical refs and a fixed logical step
- WHEN Molten constructs the transition record and receipt twice
- THEN both runs produce the same transition ref, receipt ref, decision, diagnostics, and canonical value.

### Requirement: Lifecycle receipts bind transition evidence
r[molten.lifecycle_state_machine_proof.receipt_evidence_binding] Molten MUST validate lifecycle receipt evidence by binding the receipt ref to the canonical receipt value, the transition ref to the canonical transition value, and the decision to the deterministic diagnostics for that transition.

#### Scenario: Tampered receipt is rejected
- GIVEN a lifecycle transition receipt whose decision, transition ref, diagnostics, or checks have been modified after receipt creation
- WHEN Molten validates the lifecycle receipt as proof evidence
- THEN validation denies the receipt
- AND diagnostics identify the binding that failed.

### Requirement: Runtime committed turn delta is exact
r[molten.runtime_state_machine_proof.turn_commit_delta] Molten MUST prove that a committed runtime turn publishes exactly the predicate-approved state delta for assertions, retractions, messages, observations, and recorded effect responses.

#### Scenario: Committed turn matches computed delta
- GIVEN a runtime snapshot and a pending turn with bounded actions
- WHEN Molten commits the turn through the runtime transition predicate
- THEN the after-state ref matches the pure transition result
- AND no unrecorded pending action becomes visible.

### Requirement: Runtime rollback leaves committed state unchanged
r[molten.runtime_state_machine_proof.turn_rollback_no_mutation] Molten MUST prove that denied, failed, or rolled-back runtime turns leave committed runtime state equal to the before snapshot.

#### Scenario: Denied turn preserves before snapshot
- GIVEN a pending runtime turn that is denied before commit
- WHEN Molten rolls the turn back
- THEN the resulting state ref equals the before-state ref
- AND pending assertions, retractions, messages, and effect intents are not committed.

### Requirement: Runtime turn predicate receipts bind transition evidence
r[molten.runtime_state_machine_proof.turn_predicate_receipts] Molten MUST bind runtime turn predicate receipts to before-state refs, turn inputs, after-state refs, outcomes, decisions, checks, and diagnostics.

#### Scenario: Stale commit receipt denies
- GIVEN a runtime turn receipt whose committed outcome does not match the after snapshot
- WHEN Molten validates the turn transition predicate
- THEN the receipt decision is `deny`
- AND diagnostics identify the transition mismatch.

### Requirement: Generated runtime turn traces preserve invariants
r[molten.runtime_state_machine_proof.generated_turn_traces] Molten SHOULD include bounded generated runtime turn traces that mix commit and rollback outcomes and assert the commit delta and rollback no-mutation laws after every step.

#### Scenario: Generated mixed trace stays deterministic
- GIVEN a generated bounded sequence of runtime turns
- WHEN the sequence is replayed from the same initial snapshot
- THEN the same committed state refs and predicate receipt refs are produced
- AND every denied turn leaves state unchanged.

### Requirement: Retention GC lifecycle proof binds plan apply execute audit
r[molten.retention_gc_lifecycle_proof.ordered_chain] Molten MUST prove that retention GC audit evidence follows a stored dry-run plan, matching recomputed plan, passing apply receipt, matching execution gate, retention receipt, and tombstone evidence where destructive actions require tombstones.

#### Scenario: Audit rejects broken chain
- GIVEN an execution gate whose apply ref does not match the audited plan
- WHEN Molten validates the retention GC audit chain
- THEN the audit or proof receipt decision is `deny`
- AND diagnostics identify the broken plan/apply/execute binding.

### Requirement: Retention GC denies drift before mutation
r[molten.retention_gc_lifecycle_proof.drift_no_mutation] Molten MUST prove that plan drift, denied recomputation, missing normal destructive admission, missing remote clearance import, or missing apply refs deny before deletion, tombstoning, redaction, cache invalidation, or compaction mutation.

#### Scenario: Plan drift leaves content unchanged
- GIVEN a stored GC plan whose recomputed plan ref differs
- WHEN `gc-apply-plan` or a destructive subsystem evaluates the candidate
- THEN the decision is `deny`
- AND before/after state or content refs show no destructive mutation occurred.

### Requirement: Retention GC execution scope is exact
r[molten.retention_gc_lifecycle_proof.execution_scope] Molten MUST prove that a passing execution gate is accepted only for the same subsystem, action, object ref, object kind, retention class, retention receipt, and tombstone refs bound by the apply receipt.

#### Scenario: Scope mismatch denies execution
- GIVEN a passing apply receipt for one object ref
- WHEN an execution gate is requested for another object ref or action
- THEN execution gate decision is `deny`
- AND the destructive subsystem does not remove or tombstone the requested object.
