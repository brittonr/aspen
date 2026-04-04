## ADDED Requirements

### Requirement: Secret splitting

A 32-byte secret MUST be splittable into N shares with a threshold K, where reconstructing any K shares recovers the original secret and fewer than K shares reveal nothing about it.

#### Scenario: Split and reconstruct with exactly K shares

- **WHEN** a 32-byte secret is split into N=5 shares with K=3
- **THEN** combining any 3 shares reconstructs the original secret

#### Scenario: Fewer than K shares reveal nothing

- **WHEN** fewer than K shares are combined
- **THEN** the result is either an error or a value unrelated to the original secret

#### Scenario: All N shares reconstruct correctly

- **WHEN** all N shares are combined
- **THEN** the original secret is reconstructed

### Requirement: Share digest validation

Each share MUST have a SHA3-256 digest that can be verified before reconstruction to detect corruption or tampering.

#### Scenario: Valid share passes digest check

- **WHEN** a share is verified against its stored SHA3-256 digest
- **THEN** the check succeeds if the share is unmodified

#### Scenario: Corrupted share fails digest check

- **WHEN** a share with a flipped bit is verified against its stored digest
- **THEN** the check fails and the share is rejected before reconstruction

### Requirement: Memory safety for secret data

Secret material (the root secret, shares, derived keys) MUST be zeroed in memory when dropped.

#### Scenario: Secret dropped after use

- **WHEN** a `ClusterSecret` or `Share` goes out of scope
- **THEN** its backing memory is overwritten with zeros via `zeroize`

### Requirement: Constant-time comparison

Comparison of secret data (shares, digests, secrets) MUST use constant-time operations to prevent timing side channels.

#### Scenario: Two shares compared for equality

- **WHEN** two shares are compared
- **THEN** the comparison takes the same time regardless of where the first difference occurs

### Requirement: Key derivation from cluster secret

Purpose-specific encryption keys MUST be derivable from the cluster secret using HKDF-SHA3-256 with a context string, cluster ID, and epoch.

#### Scenario: Derive a secrets-at-rest key

- **WHEN** a key is derived with context `b"aspen-v1-secrets-at-rest"`, cluster_id, and epoch
- **THEN** the result is a 32-byte key unique to that (purpose, cluster, epoch) tuple

#### Scenario: Different contexts produce different keys

- **WHEN** two keys are derived with different context strings but the same cluster_id and epoch
- **THEN** the keys are distinct (with overwhelming probability)
