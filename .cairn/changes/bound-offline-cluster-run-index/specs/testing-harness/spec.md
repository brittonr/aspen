# Offline cluster run-index intake bound

## ADDED Requirements

### Requirement: Offline cluster verification bounds the run index before assessment

r[molten.testing.receipt_first_cluster_harness.run_index_intake_bound] The local `cluster harness-verify` command MUST read at most 4 MiB plus one byte of `artifact-index.tsv`, MUST deny indexes larger than 4 MiB, and MUST deny a 4,097th artifact row before copying its fields. A 4,096-row index and exact 4 MiB input MUST remain eligible for ordinary artifact and verification-companion checks; neither limit establishes that a run passed or is production evidence.

#### Scenario: Existing supported index remains eligible
- GIVEN an index no larger than 4 MiB with at most 4,096 artifact rows
- WHEN offline cluster verification parses the index
- THEN intake preserves the rows in their existing order and proceeds to ordinary artifact/companion checks.

#### Scenario: Extra artifact is denied before admission
- GIVEN an otherwise well-formed index with 4,097 artifact rows
- WHEN offline verification reads its run directory
- THEN the command rejects the extra artifact before accepting a companion or copying the extra row fields.

#### Scenario: Oversized index file is read within a finite budget
- GIVEN an `artifact-index.tsv` larger than 4 MiB
- WHEN offline verification reads the run directory
- THEN it rejects the file after reading no more than 4 MiB plus one byte, without accepting indexed artifacts or a verification companion.
