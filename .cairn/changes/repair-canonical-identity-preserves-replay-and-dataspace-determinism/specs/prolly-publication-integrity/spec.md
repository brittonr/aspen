# Prolly publication integrity deltas

## ADDED Requirements

### Requirement: Prolly Map rebalancing enforces max_node_bytes alongside max_fanout
r[molten.prolly.rebalance_enforces_max_node_bytes] `rebalance_last_group` MUST verify that any merged or borrowed internal node remains within `profile.max_node_bytes` as well as `profile.max_fanout`, so byte-bounded splits never trigger `NodeSizeLimitExceeded`.

#### Scenario: Byte-split internal groups do not overflow on rebalance
- GIVEN two adjacent internal groups split because `candidate_size > profile.max_node_bytes` while their combined fanout fits `max_fanout`
- WHEN `rebalance_last_group` runs
- THEN it MUST NOT merge them into a node exceeding `profile.max_node_bytes`.
