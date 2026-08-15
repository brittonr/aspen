## ADDED Requirements

### Requirement: Federation read-only roles are hint-only
r[molten.peer_subscriber.federation_readonly] Molten MUST treat federation inventory, catalog, and anti-entropy subscriptions as readback or hint surfaces that cannot import artifacts, mutate registries, or establish trust without receiver-driven verification and admission.

#### Scenario: Read-only inventory cannot import artifact
- GIVEN a read-only subscriber receives a federation inventory item for an artifact
- WHEN the receiver has not fetched, hash-verified, and admitted the artifact through local policy
- THEN the artifact remains unimported
- AND the inventory projection is recorded only as hint/readback evidence.
