## ADDED Requirements

### Requirement: Cluster overview page

The system SHALL display a cluster health overview at `/cluster`. The page SHALL show each node's ID, Raft role (leader/follower/learner), health status, and uptime. The page SHALL auto-refresh every 10 seconds.

#### Scenario: View healthy cluster

- **WHEN** user navigates to `/cluster` with a 3-node cluster where all nodes are healthy
- **THEN** the page displays a table with 3 rows, each showing node ID, role (one leader, two followers), "healthy" status badge in green, and uptime

#### Scenario: View cluster with degraded node

- **WHEN** user navigates to `/cluster` and one node is unreachable
- **THEN** the page displays the unreachable node with a red "unreachable" status badge. Other nodes show green "healthy".

#### Scenario: Cluster metrics summary

- **WHEN** user views the cluster overview page
- **THEN** the page displays summary metrics at the top: total nodes, healthy count, current leader ID, and cluster uptime (time since first node joined)

#### Scenario: Navigation from nav bar

- **WHEN** user clicks "Cluster" in the main navigation bar
- **THEN** the browser navigates to `/cluster`
