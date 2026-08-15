# Project Delta: Nickel contract prelude library

### Requirement: Nickel contract modules share common domain helpers
r[molten.project.nickel_contract_prelude.shared_helpers] Repository-owned Nickel contract modules SHOULD import shared pure helper contracts for common domains such as non-empty strings, non-empty arrays, BLAKE3 refs, stable ids, positive integers, exact schema metadata, allowed values, and distinct string collections.

#### Scenario: Contract module uses shared helper
- GIVEN a Nickel contract module needs to validate a BLAKE3 ref or non-empty array
- WHEN the module is reviewed
- THEN it imports the shared helper instead of copying a divergent local predicate unless a local domain-specific exception is documented

#### Scenario: Shared helper tightens consistently
- GIVEN a shared helper predicate is tightened to reject a malformed common value
- WHEN dependent contract modules evaluate their fixtures
- THEN every importer observes the same reviewed behavior

### Requirement: Shared Nickel prelude remains authoring-time only
r[molten.project.nickel_contract_prelude.authoring_boundary] Shared Nickel contract helpers MUST remain part of source-controlled authoring and fixture validation and MUST NOT introduce runtime Nickel evaluation for production startup, plugin admission, policy authority, or receipt verification.

#### Scenario: Runtime consumes checked exports
- GIVEN a contract module imports the shared prelude for fixture validation
- WHEN runtime admission or startup validation runs
- THEN it consumes checked exported JSON or Preserves evidence and does not invoke the Nickel prelude as live authority
