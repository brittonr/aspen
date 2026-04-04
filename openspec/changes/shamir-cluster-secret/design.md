## Context

Aspen clusters replicate all state (including secrets) in plaintext across every Raft member. The secrets engine stores Transit keys, PKI private keys, and KV secrets directly in redb values. Disk compromise on any node exposes everything.

Oxide's trust-quorum uses GF(2^8) Shamir sharing (their `gfss` crate) to split a 32-byte rack secret into K-of-N shares. Each node stores only its share. Reconstruction requires cooperation of K nodes. Derived keys encrypt sensitive data at rest.

## Goals / Non-Goals

**Goals:**

- Implement GF(2^8) Shamir secret sharing (split a 32-byte secret into N shares, reconstruct from K)
- Create a cluster root secret during `init_cluster` and distribute shares to all members
- Store shares in redb with SHA3-256 digest verification
- Derive purpose-scoped encryption keys via HKDF-SHA3-256
- Zero sensitive memory on drop via `zeroize`
- Constant-time comparison of secret data via `subtle`

**Non-Goals:**

- Verifiable secret sharing (VSS) — we trust Raft leader, not Byzantine nodes
- Proactive secret sharing — periodic resharing without reconfiguration
- Hardware security module (HSM) integration
- Encrypting Raft log entries (only secrets engine data)

## Decisions

**GF(2^8) Shamir, not Ed25519+Ristretto.** Oxide's trust-quorum uses GF(2^8) for the current protocol (their `gfss` crate). It's simpler, faster, and doesn't require elliptic curve math. The secret is 32 bytes — one byte per polynomial coefficient across 32 independent polynomials. Each share is 33 bytes (1 byte x-coordinate + 32 bytes of y-values).

**Port `gfss` rather than vendoring.** Oxide's `gfss` is ~400 lines of focused Shamir code. Porting it into `aspen-trust` avoids pulling in Oxide's dependency graph while keeping the exact same math. Alternatively, evaluate `sharks` or `vsss-rs` crates if they meet constant-time requirements.

**Threshold defaults to majority: `(N/2) + 1`.** Same formula used throughout Aspen for Raft quorum. Configurable via cluster init parameters.

**HKDF key derivation with scoped contexts.** Each derived key includes a purpose string, cluster ID, and epoch in the HKDF info parameter. This prevents cross-purpose key reuse (confused deputy). Pattern from trust-quorum: `b"aspen-v1-<purpose>"` + cluster_id + epoch bytes.

**Raft leader as coordinator.** Unlike Oxide which uses Nexus (external control plane) to linearize reconfigurations, Aspen uses the Raft leader. The leader generates the secret, splits it, and distributes shares as part of a Raft-committed cluster init entry. Share distribution piggybacks on the existing Raft snapshot mechanism.

**Share storage in a separate redb table.** Shares are stored in `trust_shares` table, not mixed with application KV data. This allows different access patterns (shares are read-only after init, read at unlock time).

## Risks / Trade-offs

- **Availability impact**: Reconstructing the secret requires K nodes to be reachable. If the cluster drops below threshold, the secret cannot be reconstructed until nodes recover. This is by design — it's the security/availability trade-off.
- **Init complexity**: Cluster initialization gains a share distribution step. Failure during distribution requires retry or re-init.
- **Secret in memory**: After reconstruction, the secret exists in process memory. `zeroize` helps, but a memory dump could still capture it. Future: consider short-lived reconstruction (use derived key, drop secret immediately).
- **No backward compatibility**: Existing clusters don't have shares. Migration requires a "bootstrap trust" procedure that creates shares from the existing (unprotected) state.
