## MODIFIED Requirements

### Requirement: Federation fetch retrieves content

The `federation fetch` command SHALL fetch both ref entries AND git objects from the remote cluster, then create or update a local mirror repo. Previously, fetch only stored ref entries in KV.

#### Scenario: Fetch with content transfer

- **WHEN** a user runs `aspen-cli federation fetch --peer <id> --fed-id <id>`
- **THEN** the CLI connects to the remote, fetches ref entries, fetches git objects for all refs, creates/updates a local mirror repo, and reports counts of fetched vs already-present objects

#### Scenario: Fetch incremental update

- **WHEN** a user runs `federation fetch` for a resource that was previously fetched
- **THEN** only objects not present in the local mirror are transferred, using `have_hashes` from the existing mirror's blob store

## ADDED Requirements

### Requirement: Federation pull command

A new `federation pull` CLI command SHALL perform incremental sync of a previously mirrored federation resource without requiring the federated ID — it looks up the mirror's origin metadata.

#### Scenario: Pull updates existing mirror

- **WHEN** a user runs `aspen-cli federation pull --repo <mirror-repo-id>`
- **THEN** the system reads the mirror's origin metadata, connects to the origin cluster, fetches only new refs and objects, and updates the local mirror

#### Scenario: Pull on non-mirror repo fails

- **WHEN** a user runs `federation pull` on a repo that is not a federation mirror
- **THEN** the command fails with an error indicating the repo is not a mirror

### Requirement: Fetch reports transfer statistics

The `federation fetch` response SHALL include counts of objects transferred by type (commits, trees, blobs), total bytes transferred, and objects skipped.

#### Scenario: Fetch statistics reported

- **WHEN** a federation fetch completes
- **THEN** the CLI prints per-type object counts, total bytes, and already-present counts
