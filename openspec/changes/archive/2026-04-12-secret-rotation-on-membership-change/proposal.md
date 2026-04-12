## Why

When Aspen changes cluster membership (add/remove Raft voters), no cryptographic material rotates. A removed node still holds valid shares from the old configuration. If that node's disk is later compromised, the attacker could combine it with other old shares to reconstruct the cluster secret. Oxide's trust-quorum handles this with epoch-based reconfiguration: each membership change creates a new secret, new shares, and encrypts old secrets with the new one. Removed nodes' old shares become useless for the current epoch.

This depends on `shamir-cluster-secret` being implemented first.

## What Changes

- Add a reconfiguration protocol to `aspen-trust` that triggers on Raft membership changes
- Raft leader collects K shares from the old configuration to reconstruct the old secret
- Leader generates a new secret, splits into new shares for the new membership
- Old secret(s) are encrypted with the new secret and stored in the configuration (encrypted secret chain)
- New shares and encrypted secret chain are distributed to new members via Raft log entries
- Each reconfiguration increments an epoch counter, aligned with Raft membership log index

## Capabilities

### New Capabilities

- `trust-reconfiguration`: Epoch-based secret rotation protocol triggered by Raft membership changes, with encrypted secret chain for backward access to historical secrets

### Modified Capabilities

## Impact

- `crates/aspen-trust/` — reconfiguration protocol state machine (sans-IO)
- `crates/aspen-raft/` — hook into openraft `change_membership` to trigger trust reconfiguration
- `crates/aspen-core/` — `TrustEpoch`, `EncryptedSecretChain` types
- Cluster operations: membership changes become a two-phase operation (collect old shares → distribute new shares)
