# Persistent Node Identity Delta

## ADDED Requirements

### Requirement: Key files are published atomically
r[molten.crypto_identity.atomic_key_publication] Molten MUST write node identity and production key files by staged publication in the destination directory: the payload is written and synchronized under a stage name, linked without replacement for first creation or renamed over the current file for rotation and revocation, and the directory is synchronized. A crash MUST leave the destination absent, unchanged, or whole with owner-only permissions, never truncated or partially written. First creation MUST NOT overwrite an existing key, and a failure after the destination changed MUST be reported as an uncertain outcome.

#### Scenario: First creation never overwrites a key
- GIVEN a key file already exists at the destination
- WHEN first-boot generation publishes a new key there
- THEN publication fails and the existing key bytes are unchanged.

#### Scenario: Rotation replaces the whole key with owner-only permissions
- GIVEN an existing key file whose permissions are broader than owner-only
- WHEN rotation publishes the next generation
- THEN the destination holds exactly the new key bytes with owner-only permissions and no stage remains.

#### Scenario: An interrupted publication never becomes the key
- GIVEN a stage left by an interrupted publication of the same key
- WHEN the key is published again
- THEN the stale stage is removed and the destination holds only the new complete key.
