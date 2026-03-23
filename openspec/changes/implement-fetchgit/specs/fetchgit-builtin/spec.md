## ADDED Requirements

### Requirement: fetchGit accepts url as string argument

The `builtins.fetchGit` builtin SHALL accept a single string argument interpreted as a git URL, cloning the repository and returning an attrset with the checkout contents.

#### Scenario: String argument is a git URL

- **WHEN** `builtins.fetchGit "https://example.com/repo.git"` is evaluated
- **THEN** the system clones the repository at the URL's HEAD and returns an attrset containing `outPath` pointing to a valid store path

#### Scenario: String argument is a local path

- **WHEN** `builtins.fetchGit "/path/to/local/repo"` is evaluated
- **THEN** the system reads the local git repository and returns an attrset with `outPath` pointing to a store path containing the repo contents

### Requirement: fetchGit accepts attrset argument with url

The `builtins.fetchGit` builtin SHALL accept an attrset argument with a required `url` key and optional `rev`, `ref`, `submodules`, `shallow`, `allRefs`, `narHash`, and `name` keys.

#### Scenario: Attrset with url and rev

- **WHEN** `builtins.fetchGit { url = "https://example.com/repo.git"; rev = "abc123..."; }` is evaluated
- **THEN** the system clones the repository and checks out the specified commit

#### Scenario: Attrset with url and ref

- **WHEN** `builtins.fetchGit { url = "https://example.com/repo.git"; ref = "refs/tags/v1.0"; }` is evaluated
- **THEN** the system fetches the specified ref and checks out its tip commit

#### Scenario: Attrset with url, rev, and submodules

- **WHEN** `builtins.fetchGit { url = "https://example.com/repo.git"; rev = "abc123..."; submodules = true; }` is evaluated
- **THEN** the system checks out the commit with all submodules initialized and included in the store path

#### Scenario: Unknown attribute key is rejected

- **WHEN** `builtins.fetchGit { url = "https://example.com/repo.git"; badKey = true; }` is evaluated
- **THEN** the system SHALL return an `UnexpectedArgumentBuiltin` error naming the invalid key

### Requirement: fetchGit returns Nix-compatible attrset

The builtin SHALL return an attrset with the following keys: `outPath` (store path string), `rev` (40-char hex commit hash), `shortRev` (first 7 chars of rev), `lastModified` (unix timestamp integer), `lastModifiedDate` (string in `%Y%m%d%H%M%S` format), `revCount` (integer, 0 when shallow), `narHash` (SRI string), `submodules` (boolean).

#### Scenario: Return attrset has all required keys

- **WHEN** `builtins.fetchGit { url = "https://example.com/repo.git"; rev = "abc123..."; }` is evaluated
- **THEN** the returned attrset SHALL contain `outPath`, `rev`, `shortRev`, `lastModified`, `lastModifiedDate`, `revCount`, `narHash`, and `submodules`

#### Scenario: outPath is a valid Nix store path

- **WHEN** the builtin returns successfully
- **THEN** `outPath` SHALL start with `/nix/store/` and the store path SHALL be a content-addressed path computed from the NAR hash of the checkout contents

#### Scenario: Shallow clone sets revCount to zero

- **WHEN** `builtins.fetchGit { url = "..."; shallow = true; }` is evaluated
- **THEN** the returned `revCount` SHALL be `0`

### Requirement: fetchGit verifies narHash when provided

When the `narHash` attribute is provided in the input attrset, the builtin SHALL verify the NAR hash of the fetched content matches the expected value and fail with a hash mismatch error if they differ.

#### Scenario: narHash matches fetched content

- **WHEN** `builtins.fetchGit { url = "..."; rev = "..."; narHash = "sha256-correct..."; }` is evaluated
- **THEN** the fetch succeeds and the returned `narHash` matches the input

#### Scenario: narHash does not match fetched content

- **WHEN** `builtins.fetchGit { url = "..."; rev = "..."; narHash = "sha256-wrong..."; }` is evaluated
- **THEN** the system SHALL return a `HashMismatch` error containing both expected and actual hashes

### Requirement: fetchGit ingests checkout into castore

The fetched git checkout SHALL be ingested into snix's BlobService and DirectoryService, and a PathInfo entry SHALL be persisted to the PathInfoService with the computed NAR hash, size, and CA hash.

#### Scenario: Checkout is persisted to castore services

- **WHEN** a successful `fetchGit` completes
- **THEN** the checkout's file tree is stored in BlobService (file contents) and DirectoryService (directory structure), and a PathInfo entry exists for the computed store path

### Requirement: fetchGit enforces resource bounds

The builtin SHALL enforce Tiger Style resource limits on clone size, checkout depth, and submodule recursion.

#### Scenario: Clone exceeds maximum size

- **WHEN** a git repository's checkout exceeds `MAX_GIT_CHECKOUT_SIZE` bytes
- **THEN** the system SHALL return an error indicating the size limit was exceeded

#### Scenario: Submodule depth exceeds limit

- **WHEN** a repository has submodules nested deeper than `MAX_SUBMODULE_DEPTH` levels
- **THEN** the system SHALL return an error indicating the depth limit was exceeded

#### Scenario: Git binary not found

- **WHEN** the `git` binary is not in `$PATH`
- **THEN** the system SHALL return an error with a message indicating `git` is required

### Requirement: Fetch::Git variant has proper fields

The `Fetch::Git` enum variant SHALL contain fields for `url`, `rev`, `r#ref`, `shallow`, `submodules`, `all_refs`, and `exp_nar_sha256`, replacing the empty `Git()` stub. `store_path()` SHALL compute a CA store path when `exp_nar_sha256` is provided. `ingest()` SHALL perform the clone and return `(Node, CAHash, u64)`.

#### Scenario: store_path with known narHash

- **WHEN** `Fetch::Git { exp_nar_sha256: Some(hash), .. }.store_path("source")` is called
- **THEN** the return value SHALL be `Some(StorePathRef)` computed via `build_ca_path` with `CAHash::Nar(NixHash::Sha256(hash))`

#### Scenario: store_path without narHash

- **WHEN** `Fetch::Git { exp_nar_sha256: None, .. }.store_path("source")` is called
- **THEN** the return value SHALL be `None` (cannot compute store path without fetching)

#### Scenario: ingest performs clone and returns node

- **WHEN** `Fetcher::ingest(Fetch::Git { url, rev, .. })` is called
- **THEN** the system clones the repo, extracts the checkout, ingests into castore, and returns `(root_node, CAHash::Nar(sha256), nar_size)`
