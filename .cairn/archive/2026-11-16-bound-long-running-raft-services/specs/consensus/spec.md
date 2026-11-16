# Consensus Delta

## ADDED Requirements

### Requirement: Long-running Raft services use rate, concurrency and retention bounds
r[molten.consensus.long_running_service_bounds] Reliability and collaboration Raft services MUST be long-running services. Their peer ingress MUST admit at most a configured number of deliveries per non-zero window and pause polling while that budget is spent; a frame whose payload fails to decode MUST be dropped, counted and logged without ending ingress, while a transport or listener failure MUST end ingress with its error. A public host's connection bound MUST limit simultaneously served connections, not connections over its lifetime. A service MUST stop only on operator stop or teardown, cancellation, a failed replica effect, or an ingress transport failure.

#### Scenario: Ingress budget refills after its window
- GIVEN an ingress pump admits two deliveries per window and three frames are waiting
- WHEN two are delivered
- THEN the pump polls nothing until the window ends
- AND then delivers the third frame.

#### Scenario: Undecodable frame does not stop ingress
- GIVEN a peer sends a frame whose payload does not decode and then a valid frame
- WHEN ingress polls them
- THEN the first is dropped and counted and the second is delivered.

#### Scenario: Connections beyond the bound wait
- GIVEN a host serves at most one connection at a time and one connection is open
- WHEN a second client connects and sends a request
- THEN the second request is served after the first completes and the host keeps serving.

### Requirement: Hosts snapshot at a validated threshold
r[molten.consensus.snapshot_threshold] A replica profile MUST declare a snapshot threshold greater than zero and less than its log-entry bound. When a running voter retains at least that many applied entries above its snapshot, its hosting service MUST issue a snapshot with the application's state ref, persisting the snapshot before the log is compacted. A denied or failed snapshot MUST be reported through the host's failure path and MUST NOT be ignored.

#### Scenario: Writes continue past the log bound
- GIVEN a group whose log holds four entries and whose threshold is two
- WHEN ten writes are proposed and hosts snapshot at the threshold
- THEN every write commits.

#### Scenario: Without snapshots the log bound denies
- GIVEN the same group without snapshots
- WHEN the log is full
- THEN the next proposal is denied.

### Requirement: Duplicate requests are recognized within a deterministic window
r[molten.consensus.bounded_request_dedup] A replica profile MUST declare a duplicate-request window W. After applying index i, a replica MUST retain exactly the request refs committed at indices in (i - W, i], independent of when it snapshotted. A retry MUST be recognized as a duplicate, returning the prior committed index without a new append, only while its original commit is within the last W applied entries; an older retry MUST be treated as a new request. Snapshots MUST carry the window at their boundary, installation MUST restore it, and validation MUST reject a snapshot whose retained refs fall outside (boundary - W, boundary] or share an index.

#### Scenario: Retry within the window
- GIVEN a request committed at index i and the replica has applied fewer than W further entries
- WHEN the request is proposed again
- THEN the leader reports it committed at i and appends nothing.

#### Scenario: Snapshot timing does not change the window
- GIVEN one replica snapshots at every threshold and another never does
- WHEN both have applied the same index
- THEN their retained request refs are identical.

#### Scenario: Malformed window is rejected
- GIVEN an installed snapshot retains a request ref above its boundary or at or below boundary - W
- WHEN a replica installs it
- THEN the installation is rejected.

### Requirement: Raft durable logs compact behind durable snapshots
r[molten.consensus.durable_log_compaction] When its durable namespace declares retention authority and the port is given that authority, a Raft durability port MUST, after persisting and synchronizing a snapshot, write a compaction base holding the hard state and the entries above the snapshot boundary, then retire superseded snapshots, then truncate only records the snapshot covers. Recovery after a crash between any two steps MUST rebuild the newest valid snapshot plus the hard state and entries above it. Recovery MUST accept retained records starting after sequence zero only when a snapshot covers every earlier record, and MUST reject a compacted record stream that does not begin with that snapshot's compaction base. Without retention authority, the port MUST NOT remove records or snapshots, and the namespace limits bound the service.

#### Scenario: Compaction stays within namespace limits
- GIVEN a namespace that holds at most N snapshots and the threshold is t
- WHEN more than N × t writes commit with snapshots
- THEN one snapshot remains and the record count stays below the namespace limit.

#### Scenario: Crash between compaction steps
- GIVEN a crash after the snapshot, the compaction base or snapshot retirement
- WHEN the replica recovers
- THEN its snapshot, log, commit index, retained request refs and term equal the compacted state.
