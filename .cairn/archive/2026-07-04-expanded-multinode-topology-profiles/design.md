# Design: expanded multinode topology profiles

Add a pure topology-profile model that names scenario role shapes independently from the command runner. A profile binds a topology id, required roles, member set, allowed links, evidence scope, and required receipt classes. The profile matrix is separate from execution cost profiles so a scenario can say both "protocol simulation" and "control quorum topology" without overloading one field.

Initial profile families:

- pairwise transport handoff for sender and receiver behavior;
- control quorum for replicated control-plane state and read admission evidence;
- restart and rejoin for durable state-root recovery;
- subscriber peer for non-member observation without Raft membership rights;
- wrong topology and wrong role fixtures for negative coverage.

The core validates that commands and receipts reference declared nodes and roles. The shell may choose simulation, local multiprocess, or VM execution, but it must preserve the topology profile id in metadata and receipts.

Topology profile tests should assert both accepted and denied cases. A subscriber must not be silently promoted to a voting member, and a transport-only peer must not satisfy authority, policy, quorum, or membership evidence.
