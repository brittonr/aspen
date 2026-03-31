## ADDED Requirements

### Requirement: Federated URL parsing

git-remote-aspen SHALL recognize URLs of the form `aspen://<ticket>/fed:<origin-pubkey-base32>:<repo-id-hex>` and route them through the local cluster's federation layer instead of direct Forge RPC.

#### Scenario: Parse a federated URL

- **WHEN** git-remote-aspen receives a URL `aspen://<ticket>/fed:<origin-key>:<repo-hex>`
- **THEN** it extracts the local cluster ticket, origin public key, and upstream repo ID as separate fields

#### Scenario: Reject malformed federated URL

- **WHEN** git-remote-aspen receives a URL `aspen://<ticket>/fed:<garbage>`
- **THEN** it returns an error indicating the federated ID is malformed

#### Scenario: Non-federated URLs unchanged

- **WHEN** git-remote-aspen receives a URL `aspen://<ticket>/<repo-hex>` (no `fed:` prefix)
- **THEN** it routes through direct Forge RPC as before

### Requirement: Federated list refs

When git-remote-aspen issues a `list` command for a federated URL, the local cluster SHALL return refs from the origin cluster's repository by performing a federation sync if needed.

#### Scenario: List refs for a new federated repo

- **WHEN** the git client sends `list` for a federated repo that has no local mirror
- **THEN** the local cluster creates a mirror repo, syncs refs from the origin via federation, and returns the ref list to the git client

#### Scenario: List refs for a cached mirror

- **WHEN** the git client sends `list` for a federated repo that has a local mirror synced within the last 30 seconds
- **THEN** the local cluster returns refs from the existing mirror without contacting the origin

#### Scenario: List refs with stale mirror

- **WHEN** the git client sends `list` for a federated repo whose mirror was last synced more than 30 seconds ago
- **THEN** the local cluster performs an incremental federation pull before returning refs

#### Scenario: Origin cluster unreachable on first clone

- **WHEN** the git client sends `list` for a federated repo with no local mirror and the origin cluster is unreachable
- **THEN** the local cluster returns an error indicating the origin is unreachable

### Requirement: Federated fetch objects

When git-remote-aspen issues `fetch` commands for a federated URL, the local cluster SHALL serve git objects from the local mirror, syncing from the origin first if the mirror lacks the requested objects.

#### Scenario: Fetch objects from populated mirror

- **WHEN** the git client requests objects by SHA-1 and the local mirror contains all of them
- **THEN** the local cluster returns the objects from the mirror without contacting the origin

#### Scenario: Fetch objects triggers sync

- **WHEN** the git client requests objects that the local mirror does not have
- **THEN** the local cluster performs a federation sync for the missing objects, imports them into the mirror, and returns them

### Requirement: Deterministic mirror repo ID

The local mirror repo ID SHALL be derived as `blake3(origin_pubkey_bytes || upstream_repo_id_bytes)` so that repeated clones of the same federated repo reuse the same mirror.

#### Scenario: Same federated repo produces same mirror

- **WHEN** two separate `git clone` commands target the same federated repo on the same local cluster
- **THEN** both use the same mirror repo ID

#### Scenario: Different origin keys produce different mirrors

- **WHEN** two federated repos have the same upstream repo ID but different origin public keys
- **THEN** they produce different mirror repo IDs

### Requirement: RPC variants for federated git operations

The client API SHALL include `FederationGitListRefs` and `FederationGitFetch` request variants that the local cluster node handles by proxying through federation.

#### Scenario: FederationGitListRefs round-trip

- **WHEN** a client sends `FederationGitListRefs { origin_key, repo_id, origin_addr_hint }`
- **THEN** the server returns a response with the same shape as `GitBridgeListRefsResponse`

#### Scenario: FederationGitFetch round-trip

- **WHEN** a client sends `FederationGitFetch { origin_key, repo_id, want, have, origin_addr_hint }`
- **THEN** the server returns a response with the same shape as `GitBridgeFetchResponse`

### Requirement: NixOS VM integration test

A two-cluster NixOS VM test SHALL verify the full federated git clone flow end-to-end.

#### Scenario: Clone across clusters

- **WHEN** Alice creates and federates a repo on her cluster, and Bob runs `git clone aspen://<bob-ticket>/fed:<alice-key>:<repo-id>` against his cluster
- **THEN** Bob gets a working git checkout with the correct file contents

### Requirement: Federation clone supports repos with submodules

Federation clone MUST succeed for repositories containing gitlink (submodule) tree entries. Gitlink entries (mode 160000) are preserved during import and exported with their original SHA-1.

#### Scenario: Federated clone of repo with submodules

- **WHEN** a federated clone is performed on a repository containing trees with mode 160000 entries
- **THEN** the clone completes successfully with all objects transferred and git reports no missing or bad objects

#### Scenario: Round-trip integrity for submodule trees

- **WHEN** a tree containing gitlinks is pushed to alice's forge, then fetched via federation clone through bob
- **THEN** the fetched tree has identical content and SHA-1 to the original
