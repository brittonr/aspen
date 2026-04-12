## Context

The `shamir-cluster-secret` change establishes a root secret split across cluster members. This change addresses what happens when membership changes — adding learners, promoting voters, or removing nodes. Without rotation, removed nodes retain shares that could reconstruct the old secret.

Oxide's trust-quorum uses a coordinator-driven protocol: collect old shares → reconstruct old secret → generate new secret → encrypt old secrets with new → distribute new shares. The coordinator is chosen by their control plane (Nexus). In Aspen, the Raft leader serves this role.

## Goals / Non-Goals

**Goals:**

- Rotate the cluster secret on every membership change (add voter, remove voter, joint consensus transitions)
- Maintain an encrypted secret chain so any current member can derive keys for all historical epochs
- Implement the reconfiguration as a sans-IO state machine (per `sans-io-protocol-pattern`)
- Align trust epochs with Raft log indices for auditability
- Handle crash recovery: if leader fails mid-reconfiguration, new leader restarts the process

**Non-Goals:**

- Rotating secrets on a timer (only on membership change)
- Proactive resharing (redistributing shares without changing the secret)
- Supporting non-voter (learner) shares — only voting members get shares
- Byzantine fault tolerance — we trust Raft consensus

## Decisions

**Raft leader as coordinator.** The Raft leader already serializes membership changes. Adding share collection and distribution to this flow is natural. If the leader changes mid-reconfiguration, the new leader detects the incomplete reconfiguration and restarts it.

**Two-phase within Raft.** Phase 1: Leader sends `GetShare(old_epoch)` to old members via Iroh, collects K responses (not through Raft — this is ephemeral P2P). Phase 2: Leader constructs the new configuration (new shares, encrypted chain) and proposes it as a Raft log entry. Commitment happens through normal Raft consensus.

**Encrypted secret chain.** Each epoch's configuration carries `EncryptedSecretChain`: all prior secrets encrypted with the current epoch's secret using ChaCha20Poly1305, keyed via HKDF with context `b"aspen-v1-secret-chain"` + cluster_id + epoch. A node at epoch N can decrypt the chain to get secrets for epochs 1..N-1. This matches trust-quorum's `encrypted_rack_secrets` pattern.

**Epoch = Raft log index of membership commit.** Rather than a separate counter, the trust epoch is the Raft log index where the membership+trust entry was committed. This gives a total ordering and makes epoch uniqueness automatic.

**Coordinator state machine.** The reconfiguration coordinator is a sans-IO state machine with states: `CollectingOldShares` → `Preparing` → `Committed`. The shell layer (in `aspen-raft`) drives it by feeding in share responses and draining outbound messages.

## Risks / Trade-offs

- **Membership change latency increases.** Collecting K shares adds one round trip before the new config can be proposed. For a 3-node cluster this is ~1-2ms on LAN.
- **Share collection can stall.** If old members are unreachable, the leader can't collect K shares and the reconfiguration blocks. Mitigation: timeout and retry, or abort and let the operator intervene.
- **Chain length grows.** Each reconfiguration adds one encrypted block to the chain. For clusters with frequent membership churn, the chain could grow large. Mitigation: compact old epochs that are no longer needed (all nodes have rotated their derived keys).
- **Complexity.** This is the most complex trust-quorum feature. Bugs here could make the cluster unable to reconstruct secrets. Heavy testing (property tests, simulation, TLA+) is essential.
