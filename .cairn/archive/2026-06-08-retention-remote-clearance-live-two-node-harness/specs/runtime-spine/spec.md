## Requirements

### Requirement: Remote clearance live two-node harness

r[molten.retention.remote_clearance_live_two_node_harness] Molten MUST test the live multi-host remote-clearance happy path with two local node roots, bound live tickets, real node-control live send receipts, real receive receipts, real ingress refs, final import-workflow evidence, and destructive admission through imported peer clearance.

#### Scenario: Two-node live clearance succeeds through imported clearance

- GIVEN requester and peer node roots have bound live tickets, peer-admission evidence, and authority grants for the `gate` live ingress operation
- WHEN the requester sends a clearance request to the peer, the peer receives it, the peer sends a response to the requester, and the requester receives it
- THEN Molten imports the peer response through `retention-remote-gc-clearance-import-v1`, stores a passing live workflow bound to real send/receive/ingress refs, and passes destructive admission only through the imported peer clearance
