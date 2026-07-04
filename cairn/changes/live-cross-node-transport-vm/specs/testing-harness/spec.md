## ADDED Requirements

### Requirement: NixOS VM evidence includes true cross-node live transport
r[molten.testing.nixos_vm.cross_node_live_transport] Molten SHOULD include a NixOS VM scenario where a sender node delivers a control request to a receiver node through the admitted live transport path before any test-driver artifact export is used for evidence collection.

#### Scenario: Sender reaches receiver through live transport
- GIVEN a VM topology with admitted sender and receiver nodes, a receiver live listener, a bound ticket, peer admission evidence, and authority evidence
- WHEN the sender submits a control request through the live transport path
- THEN the VM evidence binds send, receive, ingress, queue, dispatch, reconcile, ack, and protocol-gate receipts
- AND artifact copying is used only after the live exchange for evidence export and review.

#### Scenario: Live transport scope is explicit
- GIVEN a passing cross-node live transport VM run
- WHEN the run receipt is inspected
- THEN it states that the evidence is scoped to the NixOS VM topology
- AND it does not grant authority, policy, provenance, resource, source-gate, retention, deployment, or production-readiness trust by itself.

### Requirement: Live transport VM gate rejects stale or log-only evidence
r[molten.testing.nixos_vm.live_transport_negative_gate] Molten MUST reject cross-node live transport VM pass claims when the expected peer, expected node, ticket, receive receipt, protocol gate, or receipt chain is missing, stale, mismatched, or represented only by logs.

#### Scenario: Wrong peer or stale ticket denies
- GIVEN a cross-node live transport bundle with a wrong expected peer or stale ticket ref
- WHEN the VM transport gate evaluates the bundle
- THEN the gate emits deny evidence before accepting pass evidence
- AND diagnostics identify the stale or mismatched binding.

#### Scenario: Logs cannot replace receive receipt
- GIVEN diagnostic logs showing apparent live delivery but no canonical receive transport receipt
- WHEN the VM transport gate evaluates the run
- THEN the gate denies the pass claim because logs are diagnostic-only.
