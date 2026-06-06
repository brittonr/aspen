# Node Runtime Delta: Live Send Retry and Idempotency UX

### Requirement: Live send operation ids are guardable
r[molten.node_control_live_send_retry_idempotency.spec.operation_id_guard] Node-control live send MUST support an optional expected operation id guard derived from the canonical live ingress envelope. If the supplied operation id does not match the derived operation ref, live send MUST emit a deny send receipt before opening the live transport.

#### Scenario: Operation mismatch denies before transport
- GIVEN a receiver ticket, request, peer id, and sequence
- WHEN live send is invoked with a different expected operation id
- THEN the final send receipt decision is deny
- AND diagnostics identify the operation-id mismatch.

### Requirement: Retry receipts are canonical and bounded
r[molten.node_control_live_send_retry_idempotency.spec.retry_receipts] Node-control live send retry attempts MUST be bounded and failed join or publish attempts MUST emit canonical `node-control-live-send-retry-receipt-v1` receipts binding attempt number, maximum attempts, receiver ticket, envelope ref, operation ref, and diagnostics.

#### Scenario: Failed attempts produce retry evidence
- GIVEN a live ticket with endpoint address evidence
- WHEN live send cannot join or publish within the bounded attempts
- THEN retry receipts are emitted for failed attempts
- AND the final send receipt decision is deny.

### Requirement: Duplicate send receipts suppress repeat broadcasts
r[molten.node_control_live_send_retry_idempotency.spec.duplicate_receipts] State-root-bound node-control live send MUST detect a prior passing send receipt for the derived envelope before transport, emit a canonical `node-control-live-send-duplicate-receipt-v1`, and suppress another live broadcast.

#### Scenario: Duplicate send reuses prior receipt
- GIVEN a prior passing live send receipt for an envelope in the state root
- WHEN the same request, ticket, peer, sequence, and evidence refs are sent again
- THEN a duplicate-send receipt is emitted
- AND the prior send receipt is returned without another transport publish.

### Requirement: Fail-closed diagnostics are explicit
r[molten.node_control_live_send_retry_idempotency.spec.fail_closed_diagnostics] Live send MUST fail closed with canonical diagnostics for missing ticket addresses, unsupported endpoint address forms, operation-id mismatches, join timeouts, join failures, publish failures, and stale duplicate receipt paths.

#### Scenario: Diagnostics bind failure reason
- GIVEN a malformed, offline, or mismatched live send input
- WHEN live send evaluates the input
- THEN the emitted receipt includes the concrete failure diagnostic
- AND no side effect is treated as authorized by transport evidence.

### Requirement: Retry and duplicate evidence is not authority
r[molten.node_control_live_send_retry_idempotency.spec.transport_non_authority] Live send retry receipts and duplicate-send receipts MUST NOT satisfy peer bootstrap, operation authority, policy/resource, delivery-idempotency, or payload provenance gates.

#### Scenario: Auxiliary evidence does not authorize enqueue
- GIVEN retry or duplicate-send receipts
- WHEN receiver ingress evaluates bootstrap and authority gates
- THEN those auxiliary receipts do not satisfy admission
- AND missing bootstrap or authority still denies before enqueue.

### Requirement: CLI exposes retry and duplicate UX
r[molten.node_control_live_send_retry_idempotency.spec.cli_ux] The CLI MUST expose live-send options for expected operation id, bounded maximum attempts, retry receipt export, and duplicate receipt export.

#### Scenario: CLI writes guarded send evidence
- GIVEN a node-control request and receiver ticket
- WHEN `control-ingress-live-send` is invoked with retry or duplicate options
- THEN canonical receipts are written to the requested outputs
- AND stdout reports the derived operation id and auxiliary receipt counts.

### Requirement: Tests cover retry/idempotency paths
r[molten.node_control_live_send_retry_idempotency.spec.tests] Automated tests MUST cover duplicate send suppression and operation-id mismatch denial without skipping live-send validation.

#### Scenario: Tests exercise new evidence
- GIVEN the Molten test suite
- WHEN live-send tests run
- THEN duplicate suppression is verified
- AND operation-id mismatch denial is verified.
