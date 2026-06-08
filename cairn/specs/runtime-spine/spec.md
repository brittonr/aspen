# Runtime Spine Specification

## Purpose

Defines the `runtime-spine` capability.

## Requirements

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
