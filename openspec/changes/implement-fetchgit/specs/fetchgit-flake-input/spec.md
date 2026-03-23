## ADDED Requirements

### Requirement: flake_lock resolver handles git input type

The `resolve_all_inputs()` and `resolve_all_inputs_with_fetch()` functions in `flake_lock.rs` SHALL resolve `type = "git"` locked inputs by cloning the repository at the specified URL and rev, extracting the checkout to a local directory, and returning the path as the resolved `outPath`.

#### Scenario: Git input with rev and narHash

- **WHEN** a flake.lock contains `{ "type": "git", "url": "https://example.com/repo", "rev": "abc123...", "narHash": "sha256-..." }`
- **THEN** `resolve_all_inputs_with_fetch()` SHALL clone the repo, checkout the rev, verify the narHash, and set `is_local = true` with the unpacked directory as `store_path`

#### Scenario: Git input without narHash

- **WHEN** a flake.lock contains a git input with `rev` but no `narHash`
- **THEN** the resolver SHALL clone and checkout the rev but skip narHash verification, logging a warning

#### Scenario: Git input without rev

- **WHEN** a flake.lock contains a git input with no `rev` field
- **THEN** the resolver SHALL clone the default branch HEAD and use that rev

#### Scenario: Git input fetch fails

- **WHEN** the git clone fails (network error, auth failure, invalid URL)
- **THEN** the resolver SHALL set `is_local = false` and log the error, allowing the eval path to fall back to subprocess

### Requirement: fetch_and_clone_git function

A `fetch_and_clone_git()` function in `fetch.rs` SHALL clone a git repository into a bare cache directory, extract the specified rev to a destination directory via `git archive`, and return the path to the extracted content.

#### Scenario: Clone and extract with rev

- **WHEN** `fetch_and_clone_git("https://example.com/repo.git", Some("abc123"), None, false, dest)` is called
- **THEN** the function clones into a bare cache repo, runs `git archive abc123 | tar -x` into `dest`, and returns the path to the extracted content

#### Scenario: Clone with ref and no rev

- **WHEN** `fetch_and_clone_git(url, None, Some("refs/tags/v1.0"), false, dest)` is called
- **THEN** the function clones with the specified ref, resolves it to a rev via `git rev-parse`, and extracts that rev

#### Scenario: Clone with submodules

- **WHEN** `fetch_and_clone_git(url, Some(rev), None, true, dest)` is called
- **THEN** the function performs a non-bare clone with `--recurse-submodules`, checks out the rev, and returns the path including submodule contents

#### Scenario: URL exceeds length limit

- **WHEN** `fetch_and_clone_git()` is called with a URL longer than `MAX_URL_LENGTH`
- **THEN** the function SHALL return an `InvalidInput` error

#### Scenario: Clone timeout

- **WHEN** a git clone takes longer than `GIT_CLONE_TIMEOUT_SECS`
- **THEN** the function SHALL kill the git process and return a `TimedOut` error

### Requirement: Git fetch uses FetchCache

The `fetch_git_input()` function SHALL use the existing `FetchCache` to avoid redundant clones. Cache entries are keyed by `narHash` (when available) or `url+rev` (when narHash is absent).

#### Scenario: Cache hit by narHash

- **WHEN** `fetch_git_input()` is called for a git input whose narHash is already cached
- **THEN** the cached path is returned without running any git commands

#### Scenario: Cache miss triggers clone

- **WHEN** `fetch_git_input()` is called for a git input not in cache
- **THEN** the function clones the repo, extracts the rev, verifies narHash, caches the result, and returns the path

### Requirement: Git clone enforces resource bounds

All git operations SHALL enforce Tiger Style resource limits.

#### Scenario: Clone size limit

- **WHEN** a git repository's cloned data exceeds `MAX_DOWNLOAD_SIZE` bytes
- **THEN** the clone process SHALL be killed and an error returned

#### Scenario: Clone timeout

- **WHEN** a git clone does not complete within `GIT_CLONE_TIMEOUT_SECS`
- **THEN** the clone process SHALL be killed and a timeout error returned

#### Scenario: Ref name validation

- **WHEN** `fetch_and_clone_git()` receives a `ref` value containing shell metacharacters
- **THEN** the function SHALL reject the value before passing it to the git CLI
