## ADDED Requirements

### Requirement: Convergent retry loop imports all reachable objects

`federation_import_objects` SHALL run import passes in a loop until no new objects are imported in a pass (fixed-point convergence) or a maximum iteration bound is reached.

#### Scenario: Cross-batch tree→blob dependency resolves in two passes

- **WHEN** batch 1 contains blob B and batch 2 contains tree T referencing blob B, and all objects are passed to the convergent loop
- **THEN** pass 1 imports blob B and skips tree T (or tree T fails due to ordering), pass 2 imports tree T successfully using B's mapping

#### Scenario: Deep dependency chain converges

- **WHEN** objects form a chain blob→subtree→tree→commit where each level arrived in a separate batch
- **THEN** the convergent loop completes in at most 4 passes and all objects have SHA-1→BLAKE3 mappings

#### Scenario: All objects import in first pass

- **WHEN** all objects' dependencies are satisfied within the same pass (no cross-batch issues)
- **THEN** the loop completes after one import pass plus one verification scan, with zero retries

#### Scenario: Convergence bound reached

- **WHEN** objects have genuinely missing dependencies (referenced object not in any batch)
- **THEN** the loop stops after the maximum iteration bound (10) and logs the count and SHA-1 hashes of unimported objects

### Requirement: Partial-success import semantics

`GitImporter::import_objects` SHALL NOT abort the entire import when a single object fails within a wave. It SHALL collect per-object failures and continue with remaining objects.

#### Scenario: One tree fails, sibling trees succeed

- **WHEN** wave 1 contains trees T1 (missing dep) and T2 (all deps satisfied)
- **THEN** T2 is imported successfully, T1's error is recorded in the result, and wave 2 proceeds with objects that depended on T2

#### Scenario: Wave continues after failure

- **WHEN** an object in wave N fails to import
- **THEN** waves N+1 through M still execute for objects whose dependencies are satisfied

### Requirement: Import result reports failures

`ImportResult` SHALL include a `failures` field containing the SHA-1 hash and error description for each object that failed to import.

#### Scenario: Partial import returns both successes and failures

- **WHEN** `import_objects` processes 100 objects, 95 succeed and 5 fail
- **THEN** `result.objects_imported` is 95, `result.failures.len()` is 5, and each failure entry contains the SHA-1 and a human-readable error message

#### Scenario: Full success returns empty failures

- **WHEN** `import_objects` processes 100 objects and all succeed
- **THEN** `result.failures` is empty

### Requirement: Convergent loop filters to unmapped objects

Each iteration of the convergent loop SHALL only attempt objects that do not yet have a SHA-1→BLAKE3 mapping in the mirror's `HashMappingStore`.

#### Scenario: Second pass skips already-imported objects

- **WHEN** pass 1 imports 30,000 of 33,000 objects
- **THEN** pass 2 receives only the remaining 3,000 objects (not all 33,000)
