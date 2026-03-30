## MODIFIED Requirements

### Requirement: Convergent retry loop imports all reachable objects

`federation_import_objects` SHALL run import passes in a loop until no new objects are imported in a pass (fixed-point convergence) or a maximum iteration bound is reached. Each pass SHALL also write origin→mirror BLAKE3 remap entries for newly imported objects.

#### Scenario: Cross-batch tree→blob dependency resolves in two passes

- **WHEN** batch 1 contains blob B and batch 2 contains tree T referencing blob B, and all objects are passed to the convergent loop
- **THEN** pass 1 imports blob B and skips tree T (or tree T fails due to ordering), pass 2 imports tree T successfully using B's mapping, and both have remap entries

#### Scenario: Deep dependency chain converges

- **WHEN** objects form a chain blob→subtree→tree→commit where each level arrived in a separate batch
- **THEN** the convergent loop completes in at most 4 passes and all objects have SHA-1→BLAKE3 mappings and remap entries

#### Scenario: All objects import in first pass

- **WHEN** all objects' dependencies are satisfied within the same pass (no cross-batch issues)
- **THEN** the loop completes after one import pass plus one verification scan, with zero retries

#### Scenario: Convergence bound reached

- **WHEN** objects have genuinely missing dependencies (referenced object not in any batch)
- **THEN** the loop stops after the maximum iteration bound (10) and logs the count and SHA-1 hashes of unimported objects
