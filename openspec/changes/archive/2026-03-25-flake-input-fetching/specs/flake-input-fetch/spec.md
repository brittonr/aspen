## ADDED Requirements

### Requirement: Fetch GitHub archive inputs

The flake input resolver SHALL fetch GitHub-type inputs by downloading the archive tarball from `https://github.com/{owner}/{repo}/archive/{rev}.tar.gz`, unpacking it, and returning the unpacked directory path.

#### Scenario: GitHub input not in local store

- **WHEN** `resolve_all_inputs()` encounters a GitHub input with `is_local = false`
- **AND** the locked input has `owner`, `repo`, and `rev` fields
- **THEN** the resolver SHALL download the archive tarball over HTTPS
- **AND** unpack it to a temporary directory
- **AND** set the input's `store_path` to the unpacked directory
- **AND** set `is_local = true`

#### Scenario: GitHub input missing required fields

- **WHEN** a GitHub input lacks `owner`, `repo`, or `rev`
- **THEN** the resolver SHALL return an error indicating which field is missing

### Requirement: Fetch GitLab archive inputs

The flake input resolver SHALL fetch GitLab-type inputs by downloading from `https://{host}/{owner}/{repo}/-/archive/{rev}/{repo}-{rev}.tar.gz`.

#### Scenario: GitLab input with custom host

- **WHEN** a GitLab input has a `url` field specifying a custom host
- **THEN** the resolver SHALL extract the host from the URL
- **AND** download from that host's archive endpoint

### Requirement: Fetch tarball inputs

The flake input resolver SHALL fetch tarball-type inputs by downloading from the `url` field in the locked input.

#### Scenario: Tarball input with URL

- **WHEN** `resolve_all_inputs()` encounters a tarball input with `is_local = false`
- **AND** the locked input has a `url` field
- **THEN** the resolver SHALL download the tarball over HTTPS
- **AND** unpack it to a temporary directory
- **AND** set the input's `store_path` to the unpacked directory

#### Scenario: Tarball input missing URL

- **WHEN** a tarball input lacks a `url` field
- **THEN** the resolver SHALL return an error

### Requirement: narHash integrity verification

The resolver SHALL verify fetched input content against the `narHash` from flake.lock before accepting it.

#### Scenario: narHash matches after fetch

- **WHEN** a fetched input is unpacked to a directory
- **AND** the NAR hash of the directory matches the `narHash` from flake.lock
- **THEN** the resolver SHALL accept the input and set `is_local = true`

#### Scenario: narHash mismatch after fetch

- **WHEN** a fetched input is unpacked to a directory
- **AND** the NAR hash does not match the `narHash` from flake.lock
- **THEN** the resolver SHALL reject the input
- **AND** return an error containing both the expected and computed hashes
- **AND** clean up the temporary directory

#### Scenario: Input without narHash

- **WHEN** a fetched input has no `narHash` in flake.lock
- **THEN** the resolver SHALL accept the input without hash verification
- **AND** log a warning that content was not verified

### Requirement: Fetch caching

The resolver SHALL cache fetched inputs to avoid redundant downloads for inputs with the same `narHash`.

#### Scenario: Cache hit on second evaluation

- **WHEN** `resolve_all_inputs()` is called for an input with narHash `H`
- **AND** a previous call already fetched and verified an input with the same narHash `H`
- **THEN** the resolver SHALL return the cached directory path without downloading

#### Scenario: Cache keyed by narHash

- **WHEN** two different inputs (different node keys) share the same `narHash`
- **THEN** the resolver SHALL reuse the same cached directory for both

### Requirement: Resource bounds on fetching

Fetching SHALL enforce resource limits to prevent abuse.

#### Scenario: Download size limit

- **WHEN** a download exceeds 2 GB
- **THEN** the resolver SHALL abort the download and return an error

#### Scenario: Download timeout

- **WHEN** a download takes longer than 300 seconds
- **THEN** the resolver SHALL abort and return a timeout error

#### Scenario: Maximum concurrent fetches

- **WHEN** more than 4 inputs need fetching simultaneously
- **THEN** the resolver SHALL serialize fetches beyond the concurrency limit

### Requirement: Tarball unpacking handles GitHub archive structure

GitHub archive tarballs contain a single top-level directory named `{repo}-{rev}/`. The resolver SHALL strip this prefix so the unpacked path points directly to the source tree.

#### Scenario: GitHub tarball with prefix directory

- **WHEN** a GitHub archive is unpacked
- **AND** the tarball contains a single top-level directory
- **THEN** the resolver SHALL use the contents of that directory as the `outPath`
- **AND** not include the `{repo}-{rev}/` prefix in the path

#### Scenario: Tarball with multiple top-level entries

- **WHEN** a tarball contains multiple top-level entries (not a single directory)
- **THEN** the resolver SHALL use the unpack directory directly as the `outPath`

## MODIFIED Requirements

### Requirement: Evaluator resolves flake input

The evaluator SHALL use `snix-glue`'s `SnixStoreIO` to resolve store paths from Aspen's `BlobService`, `DirectoryService`, and `PathInfoService`. Inputs not present in the local store SHALL be fetched over HTTP before evaluation begins.

#### Scenario: Evaluator reads file from store

- **WHEN** a Nix expression reads a file from `/nix/store/...`
- **THEN** the evaluator SHALL resolve the file contents through Aspen's BlobService

#### Scenario: Evaluator resolves flake input via HTTP fetch

- **WHEN** a flake.lock references a GitHub input not in the local store
- **THEN** the resolver SHALL fetch the input over HTTPS before starting evaluation
- **AND** the call-flake.nix expression SHALL receive the fetched path as an `outPath` override
