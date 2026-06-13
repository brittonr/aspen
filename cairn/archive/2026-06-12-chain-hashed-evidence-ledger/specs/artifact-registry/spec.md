# Artifact Registry Delta: chain-hashed evidence ledger

## ADDED Requirements

### Requirement: Chain links are immutable ledger artifacts
r[molten.evidence.chain_hashing.ledger_index] The local evidence ledger MUST store chain links as immutable canonical artifacts and derive chain indexes from stored link bytes and linked payload artifacts.

#### Scenario: Rebuilding indexes preserves chain heads
r[molten.evidence.chain_hashing.ledger_index.rebuild]
- GIVEN a ledger containing chain links, payload artifacts, append receipts, and checkpoints
- WHEN derived indexes are dropped and rebuilt from canonical content
- THEN chain scope/id/epoch listings, parent/child relationships, sequence lookups, payload lookups, anchors, checkpoints, and heads are reconstructed

#### Scenario: Indexed head is not authoritative without link bytes
r[molten.evidence.chain_hashing.ledger_index.head_requires_content]
- GIVEN a derived index entry claiming a chain head
- WHEN the corresponding canonical chain-link artifact is missing or hashes differently
- THEN the ledger rejects the head until the canonical link bytes are available and verified

### Requirement: Append receipts record head transitions
r[molten.evidence.chain_hashing.append_receipts] Ledger chain appends MUST emit canonical append receipts that bind head-before, head-after, appended link ref, payload ref, and continuity checks.

#### Scenario: Idempotent append of existing head
r[molten.evidence.chain_hashing.append_receipts.idempotent]
- GIVEN a chain head already points to a link ref
- WHEN the same canonical link is appended again with the same head-before and head-after
- THEN append is idempotent
- AND the append receipt names the existing link ref

#### Scenario: Unexpected stale head is denied
r[molten.evidence.chain_hashing.append_receipts.stale]
- GIVEN a chain head has advanced since a caller last observed it
- WHEN the caller appends a link against the stale head without an admitted fork or historical policy
- THEN append fails closed
- AND a denial receipt names the stale observed head and current head

### Requirement: Checkpoints are explicit artifacts
r[molten.evidence.chain_hashing.control_plane_checkpoints] Accepted control-plane chain heads SHOULD be represented by canonical checkpoint artifacts or receipts that name chain scope/id/epoch, prior checkpoint ref, new head ref, verified range, and policy/membership refs.

#### Scenario: Checkpoint descends from prior checkpoint
r[molten.evidence.chain_hashing.control_plane_checkpoints.descends]
- GIVEN a prior accepted checkpoint for a chain scope/id/epoch
- WHEN a new checkpoint is proposed
- THEN verification confirms the new head descends from the prior checkpoint head or explicitly records an admitted reconfiguration/epoch change

### Requirement: GC preserves anchored chains
r[molten.evidence.chain_hashing.anchor_policy] Retention and GC MUST preserve chain links and payload artifacts reachable from retained anchors, heads, checkpoints, or signed append/verify receipts.

#### Scenario: Pinned checkpoint preserves segment
r[molten.evidence.chain_hashing.anchor_policy.gc]
- GIVEN a retained checkpoint naming a chain head
- WHEN GC runs
- THEN the checkpoint, verified segment to the retained anchor, append/verify receipts, and required payload artifacts remain available
