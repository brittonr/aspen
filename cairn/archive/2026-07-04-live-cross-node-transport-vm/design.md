# Design: live cross-node transport VM scenario

The scenario extends the VM check with a live sender and receiver workflow. The receiver starts a live listener and emits a bound ticket. The sender imports admitted peer and authority evidence, sends a control request over the live transport, and waits for receive, ingress, queue, dispatch, reconcile, ack, and protocol-gate evidence.

The NixOS test driver may copy artifacts only after the live exchange has completed, so node-local evidence can be exported into the final manifest. It must not copy the request or response in a way that substitutes for the live transport under test.

The pure validation core checks receipt bindings: expected sender, expected receiver, topic, operation id, peer admission, authority grant, send transport receipt, receive transport receipt, queue receipt, control receipt, reconcile receipt, ack, and protocol-gate receipt. It rejects missing or stale bindings before a VM pass claim is accepted.

Negative fixtures mutate expected peer, expected node, ticket ref, receive receipt, and protocol-gate input. Logs remain diagnostic-only and cannot repair missing canonical receipts.
