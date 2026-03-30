## Why

Federated `git clone` fails for large repos (~33K objects) because the mirror's DAG exporter can't walk the object graph. After federation sync successfully imports all objects into the mirror, `export_commit_dag` returns only ~7,500 of ~33,800 objects. Git reports "Could not read" for missing parent commits.

The root cause is a BLAKE3 hash domain mismatch: commits and trees embed references to their dependencies using the **origin cluster's** BLAKE3 hashes (from the original `SignedObject` wrappers), but the federation import re-signs objects with the **mirror's** secret key, producing different BLAKE3 hashes. The exporter's BFS follows origin BLAKE3 hashes that don't exist in the mirror's KV namespace, silently terminating the walk.

## What Changes

- Add an origin→mirror BLAKE3 hash remapping index populated during federation import
- Modify the mirror's DAG exporter to resolve origin BLAKE3 references through the remap index
- Add diagnostic logging when the BFS encounters unresolvable BLAKE3 hashes (currently silent)

## Capabilities

### New Capabilities

- `federation-blake3-remap`: BLAKE3 hash remapping from origin to mirror namespace during federated clone export

### Modified Capabilities

- `federation-git-import`: Federation import must record origin→mirror BLAKE3 mappings for each imported object
- `convergent-import`: Convergent retry must also build remap entries for recovered objects

## Impact

- `crates/aspen-forge/src/git/bridge/exporter.rs` — DAG walk uses remap index
- `crates/aspen-forge-handler/src/handler/handlers/federation.rs` — import writes remap entries
- `crates/aspen-forge-handler/src/handler/handlers/federation_git.rs` — mirror fetch passes remap to exporter
- KV storage: new key prefix for remap index (`forge:remap:{mirror_repo}:{origin_blake3} → mirror_blake3`)
