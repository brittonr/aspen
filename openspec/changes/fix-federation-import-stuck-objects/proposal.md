## Why

Federation import's convergent retry loop stalls with ~2,900 stuck objects out of ~34,000. The root cause: `import_objects` extracts tree entry SHA-1 dependencies from the raw git content, but these SHA-1s reference the *origin cluster's* hashes. When the local import re-serializes an object (tree entry ordering, mode formatting), the resulting SHA-1 differs from the origin's. The dependency resolver can't find the mapping for the origin SHA-1 → local BLAKE3, so the parent tree stays stuck forever.

The convergent loop detects "no progress" and stops, leaving commits that reference unimported trees. `git clone` then fails with "Could not read {sha1}".

## What Changes

- Fix `federation_import_objects` to store origin SHA-1 → BLAKE3 mappings *before* dependency resolution, not just after. Trees that reference sub-objects by their origin SHA-1 will find mappings created during blob import.
- Add a raw-bytes import path that preserves the original git object bytes instead of re-serializing. If the import stores the exact bytes received from the origin, the SHA-1 won't change and the dependency chain stays intact.
- Add a post-import verification pass: after the convergent loop exhausts, re-attempt stuck objects using the now-complete mapping table. Objects stuck due to ordering (not genuinely missing deps) will succeed.

## Capabilities

### New Capabilities

- `federation-import-convergence`: Reliable convergent import that resolves all cross-batch dependencies for federation-synced git objects.

### Modified Capabilities

- `federated-git-clone`: After this fix, federated clone of 34K+ object repos delivers all objects without "Could not read" errors.

## Impact

- **`crates/aspen-forge-handler/src/handler/handlers/federation.rs`** — `federation_import_objects` convergent loop: pre-populate origin SHA-1 mappings, add final-pass retry for stuck objects.
- **`crates/aspen-forge/src/git/bridge/importer.rs`** — `import_objects` / `import_single_object`: accept optional raw bytes to skip re-serialization.
- **`crates/aspen-forge/src/git/bridge/converter/import.rs`** — Import converter: preserve original bytes when provided.
- **`crates/aspen-forge-handler/src/handler/handlers/federation_git.rs`** — `sync_from_origin`: pass origin SHA-1 hints through sync objects to guide mapping pre-population.
