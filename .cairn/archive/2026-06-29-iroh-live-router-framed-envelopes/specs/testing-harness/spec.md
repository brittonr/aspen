## ADDED Requirements

### Requirement: Multi-node VM framed stream coverage
r[molten.testing.nixos_vm_multinode.framed_stream] Molten SHOULD extend the NixOS multi-node VM test to exercise at least one admitted framed Iroh bidirectional stream between VM nodes and bind the framed-stream receipts into the VM test-run evidence.

#### Scenario: VM test binds framed stream child receipt
- GIVEN two VM nodes with admitted peer, authority, policy, resource, and router registration evidence
- WHEN a canonical Preserves envelope crosses a framed Iroh stream between the nodes
- THEN the VM test-run receipt includes child refs for router admission, stream session, framed-envelope pass receipt, and downstream node-control or protocol-session admission
- AND the receipt states that live stream observations are non-replayable unless separately recorded.

#### Scenario: VM denial covers unsupported ALPN or malformed frame
- GIVEN a VM test attempts an unsupported ALPN connection or sends a malformed framed envelope
- WHEN the framed stream path evaluates the attempt
- THEN Molten emits deny evidence before state mutation
- AND the VM test binds the denial as diagnostic coverage rather than transport-derived authority.
