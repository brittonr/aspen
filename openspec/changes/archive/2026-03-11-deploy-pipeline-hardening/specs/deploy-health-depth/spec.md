## ADDED Requirements

### Requirement: Raft log gap verification in health check

The deploy health check SHALL verify that the target node's Raft replication log gap is within an acceptable threshold before marking the node as healthy. A node that responds to GetHealth but has not caught up on replication SHALL be considered not yet healthy.

#### Scenario: Node healthy with acceptable log gap

- **WHEN** the coordinator checks health of a recently upgraded node
- **AND** the node responds to GetHealth with status "healthy"
- **AND** the leader's Raft metrics show the node's matched_log_index is within MAX_HEALTHY_LOG_GAP of the leader's last_log_index
- **THEN** check_health() SHALL return Ok(true)

#### Scenario: Node healthy but large log gap

- **WHEN** the coordinator checks health of a recently upgraded node
- **AND** the node responds to GetHealth with status "healthy"
- **AND** the leader's Raft metrics show the node's matched_log_index is more than MAX_HEALTHY_LOG_GAP behind the leader's last_log_index
- **THEN** check_health() SHALL return Ok(false)

#### Scenario: Node not in replication map

- **WHEN** the coordinator checks health of a recently upgraded node
- **AND** the leader's Raft metrics do not contain a replication entry for the target node
- **THEN** check_health() SHALL return Ok(false)

#### Scenario: GetHealth fails but metrics available

- **WHEN** the coordinator checks health and the GetHealth RPC fails
- **THEN** check_health() SHALL return Err regardless of replication state

### Requirement: MAX_HEALTHY_LOG_GAP constant

A constant MAX_HEALTHY_LOG_GAP SHALL be defined in aspen-constants to control the acceptable Raft replication lag for deploy health checks.

#### Scenario: Default value

- **WHEN** MAX_HEALTHY_LOG_GAP is not overridden
- **THEN** it SHALL default to 100 entries

### Requirement: Health check uses local metrics

The coordinator SHALL query Raft replication progress via the local ClusterController::get_metrics() rather than sending an additional RPC to the target node. The leader has authoritative knowledge of each follower's replication state.

#### Scenario: No extra RPC for log gap

- **WHEN** the coordinator checks health of a target node
- **THEN** it SHALL call get_metrics() on the local controller
- **AND** it SHALL NOT send a GetRaftMetrics or similar RPC to the target node for log gap verification
