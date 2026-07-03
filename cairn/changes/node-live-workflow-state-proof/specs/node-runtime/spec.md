## ADDED Requirements

### Requirement: Node live workflow lifecycle is ordered
r[molten.node_live_workflow_state_proof.ordered_lifecycle] Molten MUST prove that node live workflow bundle evidence advances only in the order bundle export, verify or gate, apply, optional send, receiver ingress or queue evidence, reconcile, ack, and import or protocol gate.

#### Scenario: Out-of-order apply denies
- GIVEN a live workflow bundle apply receipt without a matching passing bundle gate receipt
- WHEN Molten evaluates the workflow as enqueue or dispatch evidence
- THEN the workflow decision is `deny`
- AND no ingress, queue, dispatch, or import side effect is admitted from that apply receipt.

### Requirement: Node live workflow evidence binds operation identity
r[molten.node_live_workflow_state_proof.operation_binding] Molten MUST prove that live workflow reconcile, ack, import, and protocol-gate evidence bind the same bundle ref, request ref, operation ref, envelope ref, and expected receiver evidence before accepting a completed workflow.

#### Scenario: Ack for wrong operation denies
- GIVEN a passing reconcile receipt for one operation ref
- WHEN an ack bundle carries a different operation ref or request ref
- THEN the protocol gate or import receipt decision is `deny`
- AND diagnostics identify the mismatched workflow binding.

### Requirement: Live transport remains non-authorizing
r[molten.node_live_workflow_state_proof.transport_evidence_only] Molten MUST prove that live transport, neighbor, listener, send, and receive receipts do not replace peer admission, authority grant, provenance, policy, resource, source-gate, or operation evidence.

#### Scenario: Transport-only evidence cannot enqueue
- GIVEN a live send receipt and no imported peer admission or authority grant evidence
- WHEN node control evaluates ingress admission
- THEN the request is denied before enqueue
- AND diagnostics state which non-transport evidence is missing.
