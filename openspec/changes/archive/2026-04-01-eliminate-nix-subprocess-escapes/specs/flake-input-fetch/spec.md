## MODIFIED Requirements

### Requirement: HTTP fetching uses in-process client

The `fetch.rs` module SHALL use `reqwest` for HTTP downloads instead of spawning `curl` subprocesses. Tarball decompression and unpacking SHALL continue to use `flate2`/`tar` in-process.

#### Scenario: Download tarball via reqwest

- **WHEN** a flake input requires fetching a tarball URL (GitHub archive, generic tarball)
- **THEN** the system downloads the content via `reqwest` with streaming response, decompresses via `flate2`, and unpacks via `tar` — no `curl` subprocess

#### Scenario: Download timeout

- **WHEN** the HTTP server does not respond within `FETCH_TIMEOUT_SECS` (300s)
- **THEN** the reqwest client times out and returns an error

#### Scenario: Download size limit

- **WHEN** the response body exceeds `MAX_DOWNLOAD_SIZE` (2 GB)
- **THEN** the system aborts the download and returns an error

#### Scenario: Redirect following

- **WHEN** the URL returns HTTP 301/302 (GitHub archive URLs redirect)
- **THEN** reqwest follows up to 10 redirects (matching curl default behavior)

### Requirement: Git clone uses in-process fetching

The `fetch.rs` git clone operations SHALL continue to use `git` subprocess (git is a build-time dependency, not a nix dependency — this is acceptable).

#### Scenario: Git clone for git-type inputs

- **WHEN** a flake input has `type = "git"`
- **THEN** the system uses `git clone` subprocess (not a nix subprocess, so this is out of scope for elimination)

### Requirement: FetchCache compatibility

The `FetchCache` keyed by narHash SHALL work identically with the reqwest-based fetcher as it did with curl.

#### Scenario: Cache hit avoids re-download

- **WHEN** a second input references the same narHash as an already-fetched input
- **THEN** the cached directory is reused without a second HTTP request

#### Scenario: Concurrent fetch limiting

- **WHEN** multiple inputs are fetched simultaneously
- **THEN** the semaphore limits concurrent fetches to `MAX_CONCURRENT_FETCHES` (4)
