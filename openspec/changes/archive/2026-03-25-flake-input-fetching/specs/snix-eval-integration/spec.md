## MODIFIED Requirements

### Requirement: Evaluator resolves flake input

The evaluator SHALL use `snix-glue`'s `SnixStoreIO` to resolve store paths from Aspen's `BlobService`, `DirectoryService`, and `PathInfoService`. Inputs not present in the local store SHALL be fetched over HTTP before evaluation begins via the flake input resolver.

#### Scenario: Evaluator reads file from store

- **WHEN** a Nix expression reads a file from `/nix/store/...`
- **THEN** the evaluator SHALL resolve the file contents through Aspen's BlobService

#### Scenario: Evaluator resolves flake input via HTTP fetch

- **WHEN** a flake.lock references a GitHub input not in the local store
- **THEN** the resolver SHALL fetch the input over HTTPS before starting evaluation
- **AND** the call-flake.nix expression SHALL receive the fetched path as an `outPath` override
