# Durable state ports: digest-named record delta

## ADDED Requirements

### Requirement: Declared-interest records are digest-named
r[molten.durable_state_ports.digest_named_records] Where Molten maintains an admitted store of declared-interest records, it MUST store one record per declared fact in a file named by the BLAKE3 digest of the canonical record bytes, MUST reject a record whose name and content disagree, and MUST merge the store as a deterministic union deduplicated by digest.

#### Scenario: Concurrent writers both survive
- GIVEN two owners declare interest in the same store at the same time
- WHEN both records are written
- THEN both records are present and the merged view contains both declared facts.

#### Scenario: Removal retracts one fact
- GIVEN a store with three valid records
- WHEN one record file is removed
- THEN the merged view loses exactly that fact and keeps the other two.

#### Scenario: Name and content must agree
- GIVEN a record file whose stored bytes do not hash to its file name
- WHEN the store is read
- THEN the record is rejected and does not participate in the merge.

#### Scenario: Merge ignores file order
- GIVEN the same set of valid records read in two different directory orders
- WHEN the merged view is computed twice
- THEN both views are identical.
