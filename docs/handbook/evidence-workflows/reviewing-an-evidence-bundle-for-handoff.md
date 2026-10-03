# Reviewing an evidence bundle for handoff

Mode: Review checklist

Use this checklist before handing canonical receipts, provenance records, and optional scoped chain evidence to another operator or reviewer. “Bundle” here means the proposed collection of review artifacts, not a newly defined archive format or a promise that one universal verifier understands every artifact. The [Handbook](../README.md) links related procedures. The [proof workflow](../../proof-workflow.md) supplies the governing claim/assumption/negative-evidence checklist; the [technical companion](../../technical/foundations/evidence-and-authority-separation.md) supplies theory rather than additional admission authority.

This checklist is based on inspected code and checked-in tests. It does not report a runtime validation of your bundle, and no commands were executed for this batch. Mark each acceptance question with actual evidence, a bounded exemption, or a blocker. Do not replace missing evidence with an optimistic check mark.

## Claim and subject acceptance

- [ ] **Can the recipient state the exact claim in one sentence?** Record the subject ref, operation, profile, and intended consuming gate. “All evidence passed” is not sufficiently scoped.
- [ ] **Is the subject distinct from the evidence object?** Inventory artifact, provenance record, evaluation receipt, build record, chain link, and append receipt refs separately. A wrapper's hash is not the payload's hash.
- [ ] **Is the evidence scope explicit?** Label pure-core tests, local fixtures, shell effects, and live-adapter observations. A synthetic reviewed provenance record must be labeled synthetic even when its canonical ref is well formed.
- [ ] **Are non-claims visible?** State that the collection alone grants no authority, policy, resource, transport, retention, execution, or source-gate trust. Do not infer production readiness or exactly-once effects from receipt presence.

Acceptance evidence should be the canonical records plus a short inventory mapping each one to the claim it supports. Rendered summaries are navigation aids, not replacements for that inventory.

## Identity and materialization acceptance

- [ ] **Can each required ref resolve to the intended bytes?** Ledger readback parses canonical bytes and recomputes the requested hash. Preserve evidence of that readback from the owning workflow rather than relying only on a content-looking filename.
- [ ] **Are required child objects available to the recipient?** A provenance ref sequence and a chain payload descriptor name objects; neither automatically supplies those objects. Mark intentionally excluded or confidential dependencies and explain the resulting verification limit.
- [ ] **Is the import/export distinction understood?** Ordinary ledger import returns an import receipt value but does not recursively store it. Chain append explicitly stores its append receipt. A handoff must include the receipts it actually claims to preserve.
- [ ] **Were failures preserved?** Retain malformed/missing-ref diagnostics and any evidence of hash mismatch. Do not erase a damaged ledger or overwrite a deny receipt to produce a cleaner package.

The inspected ledger tests cover malformed refs, missing content, and tampered materialized bytes. They are suitable source pointers for expected behavior, not fresh verification results for this handoff.

## Provenance acceptance

- [ ] **Does the selected record name the requested artifact?** Compare the record's `artifact` with the actual consumer subject; record hashes and directory names cannot answer this question.
- [ ] **Does the operation/profile match the real consumer?** Ordinary local-test and node-control thresholds differ. Sensitive operations use explicit names in the threshold function; a convenient alternative spelling is not equivalent evidence.
- [ ] **Is a reproducible-verified claim bound end to end?** Require a passing build-verification receipt whose expected and actual refs both equal the subject and whose build-record ref is bound by the provenance record. Resolve the build record and preserve the independent evidence that established the actual ref.
- [ ] **Has the verifier's limit been stated?** `verify_build` compares a caller-supplied actual ref with a build record. It does not perform a build. The strong threshold also accepts policy-trusted, while the extra build-binding branch is specific to reproducible-verified; review the actual path rather than a broad threshold label.
- [ ] **Were decision and diagnostics inspected in canonical evidence?** The provenance CLI can complete successfully while writing a deny receipt. Its short summary does not render every field.

## Continuity acceptance, when a chain is included

- [ ] **Is the complete scope tuple recorded?** Include scope, id, and epoch, plus the requested anchor/head. A chain in another tuple cannot fill a gap in this one.
- [ ] **Are payload, link, predicate, and append receipt refs distinguished?** The checked-in append fixture reads these as separate objects and derives heads from the indexed links.
- [ ] **Are forks and gaps dispositioned by the owning policy?** The default verifier rejects unexpected forks. Diagnostic retention preserves observations but does not supply fork choice or global ordering.
- [ ] **Are verification effects disclosed?** The library segment verifier stores predicate/verification evidence. Do not describe running it against a production ledger as a purely read-only check.

## Worked handoff rejection

Consider a collection containing a reproducible-verified provenance record for artifact A and a passing build-verification receipt for A. The receipt references build record B, but the provenance record binds only C. Both objects may parse and the artifact strings may agree; the handoff still lacks the required binding. The correct review outcome is “blocked on a matching bound build verification,” with A, B, C, and the deny diagnostics preserved.

Do not edit the provenance record to insert B merely to close the checklist. Ask the producer to establish which build record belongs to the claim and produce the corresponding evidence through the owning workflow. If corrected evidence arrives, retain the earlier denied collection as history and identify the new canonical refs.

## Final acceptance record

Before handoff, record who reviewed the collection, the exact inputs and unresolved dependencies, executed verification versus source-only observations, negative evidence, confidentiality limits, and the receiving subsystem's remaining gates. Documentation-only work may use the explicit exemption permitted by the proof workflow. Neither a receipt export nor this checklist promotes evidence into current authority.

## Sources

- [Handbook](../README.md)
- [Proof workflow and exemptions](../../proof-workflow.md)
- [Evidence and authority separation](../../technical/foundations/evidence-and-authority-separation.md)
- [Ledger import/readback semantics](../../../src/ledger/parts/mod/p000/body.rs)
- [Ledger malformed/missing/tamper tests](../../../src/ledger/parts/mod/tests/m000/p000/body.rs)
- [Provenance evaluation](../../../src/provenance/parts/mod/p001/body.rs)
- [Build binding and operation thresholds](../../../src/provenance/parts/mod/p002/body.rs)
- [Chain append fixture](../../../src/evidence/parts/chain/tests/m000/p000/body.rs)
- [Chain verification and evidence writes](../../../src/evidence/parts/chain/p003/body.rs)
