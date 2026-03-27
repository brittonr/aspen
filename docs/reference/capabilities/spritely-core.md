# The Heart of Spritely: Distributed Objects and Capability Security

- **Author**: Spritely Institute (Christine Lemmer-Webber et al.)
- **URL**: https://files.spritely.institute/papers/spritely-core.html

Technical paper on Spritely Goblins — a distributed object programming environment built on capability security. Covers:

- Capability security as ordinary reference passing ("if you don't have it, you can't use it")
- ACL ambient authority vs ocap least authority with concrete Solitaire example
- Spritely Goblins: distributed, transactional object programming
  - Vat model of computation (event loops as transaction boundaries)
  - State as updating behavior (`bcom` / "become")
  - Synchronous (`$`) and asynchronous (`<-`) message passing
  - Transactions make errors survivable (automatic rollback on failure)
  - Promise pipelining (reduces network round trips)
  - Failure propagation through pipelines
- Security as relationships between objects (blogging system tutorial)
  - Reader vs editor capabilities for the same resource
  - Revocation and accountability patterns
  - Guest editing with multi-stakeholder review
- OCapN: protocol for secure distributed systems
  - CapTP (Capability Transport Protocol)
  - Netlayers (transport-agnostic networking)
  - URI schemes for distributed object references
- Portable encrypted storage for persistence
- Safe serialization and upgrade

Key concepts relevant to Aspen:

- **Vat model**: Maps to Aspen's Raft-based transactional state machine — operations commit atomically or roll back
- **Promise pipelining**: Reducing round trips in distributed calls — relevant to Aspen's RPC design
- **OCapN protocol**: Reference architecture for capability-secure networking over QUIC/Tor/etc
- **Distributed object references**: Unforgeable, transferable, attenuable — mirrors Aspen capability tokens
- **Transactional error handling**: Automatic state rollback on failure aligns with Raft's atomic apply
