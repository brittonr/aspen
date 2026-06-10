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
