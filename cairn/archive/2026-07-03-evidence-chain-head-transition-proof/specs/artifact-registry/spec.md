## ADDED Requirements

### Requirement: Evidence chain head transitions are continuous
r[molten.evidence_chain_state_machine_proof.head_transition_continuity] Molten MUST prove that evidence-chain append operations advance from head-before to head-after only when the appended link, payload ref, predicate receipt ref, and continuity checks are canonical and consistent.

#### Scenario: Valid append advances one head
- GIVEN a chain head and a canonical append link whose prior head matches the observed head
- WHEN Molten appends the link
- THEN the append receipt binds head-before, head-after, appended link ref, payload ref, and predicate receipt ref
- AND the resulting head equals the appended link ref.

### Requirement: Evidence chain gaps and forks deny
r[molten.evidence_chain_state_machine_proof.gap_fork_denial] Molten MUST prove that chain verification denies missing intermediate links, stale observed heads, forked heads, duplicate sequence conflicts, and tampered payload refs before accepting a chain segment as continuous evidence.

#### Scenario: Forked head denies verification
- GIVEN two append links that claim the same prior head for the same chain scope and epoch
- WHEN Molten verifies the chain segment
- THEN verification emits a denial receipt
- AND diagnostics identify the fork or duplicate head transition.

### Requirement: Evidence chain checkpoints and anchors preserve reachable evidence
r[molten.evidence_chain_state_machine_proof.checkpoint_anchor_preservation] Molten MUST prove checkpoints, retained heads, anchors, and signed append or verify receipts preserve every reachable chain link and payload artifact required to validate the retained chain segment.

#### Scenario: Retained checkpoint protects chain segment
- GIVEN a retained checkpoint for a verified chain segment
- WHEN retention or garbage collection evaluates reachable evidence
- THEN the checkpoint, verified links, payload artifacts, append receipts, and verify receipts remain available
- AND unanchored unrelated artifacts may still be removed according to retention policy.
