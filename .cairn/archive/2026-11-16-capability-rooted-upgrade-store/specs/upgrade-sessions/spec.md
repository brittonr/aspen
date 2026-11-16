# Upgrade Sessions Delta

## ADDED Requirements

### Requirement: The upgrades store is capability-rooted and publishes atomically
r[molten.upgrades.capability_rooted_store] Molten MUST perform every upgrades-store read, scan and write through a typed store root opened once from the operator-selected directory, refusing symlinks and non-regular members. Content-addressed plans and receipts MUST be published without replacement, and an existing member with different bytes MUST be reported as a conflict. Name pointers and task status MUST be replaced atomically, so a crash leaves them absent, previous or whole, and a pointer's receipt MUST be stored before the pointer.

#### Scenario: A stored plan is never rewritten
- GIVEN a plan already published in the store with different bytes at its content-addressed name
- WHEN the same plan reference is published again
- THEN publication fails with a conflict and the stored bytes are unchanged.

#### Scenario: A link inside the store is refused
- GIVEN a name pointer leaf or a store directory replaced by a symlink to a location outside the store
- WHEN the store reads, scans or writes that member
- THEN the operation fails and nothing outside the store is read or written.

#### Scenario: An interrupted publication is invisible
- GIVEN a publication stage left in a store directory by an interrupted write
- WHEN the store lists pointers or computes its state snapshot reference
- THEN the stage is ignored.
